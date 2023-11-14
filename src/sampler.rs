use crate::volume::Volume;
use nih_plug::{nih_error, nih_warn};
use nih_plug_vizia::vizia::image::flat::Error::ChannelCountMismatch;
use nih_plug_vizia::vizia::prelude::Key::ChannelDown;
use std::collections::{HashMap, LinkedList};
use std::fmt::{Debug, Formatter};

fn calc_sample_pos(data_len: usize, read: f32) -> usize {
    let len_f32 = (data_len as f32);
    let i = read % len_f32;
    let i = if i < 0.0 { i + len_f32 } else { i };
    let i = i as usize;
    i
}

#[derive(Clone, Debug, Default)]
struct Voice {
    note: u8,
    read: f32,
    volume: Volume,
    speed: f32,
}

#[derive(Clone, Debug)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    read: f32,
    voices: Vec<Voice>,
    recording: bool,
    playing: bool,
    now: usize,
    note_on_count: usize,
    passthru_on: bool,
    passthru_volume: Volume,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub fade_samples: usize,
    pub auto_passthru: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            fade_samples: 100,
            auto_passthru: true,
        }
    }
}

impl Channel {
    fn new(params: &Params) -> Self {
        Channel {
            data: vec![],
            write: 0,
            read: 0.0,
            voices: vec![],
            recording: false,
            playing: false,
            now: 0,
            note_on_count: 0,
            passthru_on: false,
            passthru_volume: Volume::new(if params.auto_passthru { 1.0 } else { 0.0 }),
        }
    }
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    fn dump_befpre_death(&mut self) -> String {
        self.data.clear();
        format!("{:?}", self)
    }

    fn log(&self, params: &Params, s: String) {
        nih_warn!(
            "voices={} note_on={} fade_time={} {}",
            self.voices.len(),
            self.note_on_count,
            params.fade_samples,
            s
        );
    }

    pub fn start_playing(&mut self, pos: f32, note: u8, velocity: f32, params: &Params) {
        let read = (pos * self.data.len() as f32).round();
        let mut voice = Voice {
            note,
            read,
            volume: Volume::new(0.0),
            speed: 1.0,
        };
        voice.volume.to(self.now, params.fade_samples, velocity);
        self.voices.push(voice);
        self.note_on_count += 1;
        if params.auto_passthru {
            if self.passthru_on {
                self.passthru_on = false;
                self.passthru_volume.to(self.now, params.fade_samples, 0.0);
            }
        }
        self.log(params, format!("START PLAYING note={}", note));
    }

    pub fn stop_playing(&mut self, note: u8, params: &Params) {
        for i in 0..self.voices.len() {
            if self.voices[i].note == note {
                self.voices[i].volume.to(self.now, params.fade_samples, 0.0);
                break;
            }
        }
        self.note_on_count -= 1;
        if params.auto_passthru {
            if !self.passthru_on {
                self.passthru_on = true;
                self.passthru_volume.to(self.now, params.fade_samples, 1.0);
            }
        }
        self.log(params, format!("STOP PLAYING note={}", note));
    }

    pub fn start_recording(&mut self, params: &Params) {
        assert!(!self.recording);
        assert!(self.write == 0);
        self.recording = true;
    }

    pub fn stop_recording(&mut self, params: &Params) {
        if self.recording {
            self.recording = false;
            self.data.truncate(self.write);
            self.write = 0;
            //log!("STOP RECORIDNG (wrote {} samples)", self.data.len());
        } else {
            //log!("STOP RECORIDNG (already stopped)");
        }
    }

    pub fn process_sample<'a>(&mut self, sample: &mut f32, params: &Params) -> bool {
        let value = *sample;

        if self.recording {
            assert!(self.write <= self.data.len());
            if self.write == self.data.len() {
                self.data.push(value);
            } else {
                self.data[self.write] = value;
            }
            self.write += 1;
        }

        let mut output = 0.0;
        let mut removed = vec![];
        for (i, voice) in self.voices.iter_mut().enumerate() {
            if !self.data.is_empty() {
                let y = self.data[calc_sample_pos(self.data.len(), voice.read)];
                output += y * voice.volume.value(self.now);
                voice.read += voice.speed;
            };
            voice.volume.step(self.now);
            if voice.volume.is_static_and_mute() {
                removed.push(i);
            }
        }

        while let Some(j) = removed.pop() {
            self.voices.remove(j);
        }

        if (params.auto_passthru
            && self.passthru_volume.is_static_and_mute()
            && self.note_on_count == 0)
        {
            nih_error!("{}", self.dump_befpre_death());
            return false;
        }
        output += value * self.passthru_volume.value(self.now);
        self.passthru_volume.step(self.now);

        //if !removed.is_empty() {
        //    if removed.len() == 1 {
        //        self.voices.remove(removed[0]);
        //    } else {
        //        let mut tmp = vec![];
        //        std::mem::swap(&mut tmp, &mut self.voices);
        //        let mut j = 0;
        //        for (i, voice) in tmp.into_iter().enumerate() {
        //            if j >= removed.len() || i != removed[j] {
        //                self.voices.push(voice);
        //            } else {
        //                j += 1;
        //            }
        //        }
        //    }
        //}

        //if self.playing {
        //    let data_value = if self.data.is_empty() {
        //        0.0
        //    } else {
        //        self.data[calc_sample_pos(self.data.len(), self.read)]
        //    };
        //    output += data_value;
        //    self.read += 1.0;
        //} else {
        //    output += value;
        //}
        self.now += 1;
        *sample = output;
        true
    }
}

#[derive(Debug)]
pub struct Sampler {
    channels: Vec<Channel>,
}

impl Sampler {
    pub fn new(channel_count: usize, params: &Params) -> Self {
        Self {
            channels: vec![Channel::new(&params); channel_count],
        }
    }
    fn each<F>(&mut self, f: F)
    where
        F: FnMut(&mut Channel),
    {
        self.channels.iter_mut().for_each(f)
    }
    pub fn start_playing(&mut self, pos: f32, note: u8, velocity: f32, params: &Params) {
        self.each(|ch| ch.start_playing(pos, note, velocity, params));
    }

    pub fn stop_playing(&mut self, note: u8, params: &Params) {
        self.each(|ch| ch.stop_playing(note, params));
    }

    pub fn start_recording(&mut self, params: &Params) {
        self.each(|ch| Channel::start_recording(ch, params));
    }

    pub fn stop_recording(&mut self, params: &Params) {
        self.each(|ch| Channel::stop_recording(ch, params));
    }

    pub fn process_sample<'a>(
        &mut self,
        iter: impl IntoIterator<Item = &'a mut f32>,
        params: &Params,
    ) {
        for (i, sample) in iter.into_iter().enumerate() {
            if !self.channels[i].process_sample(sample, params) {
                panic!("doh");
            }
        }
    }
}
