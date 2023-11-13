use crate::volume::Volume;
use nih_plug::nih_warn;
use nih_plug_vizia::vizia::image::flat::Error::ChannelCountMismatch;
use std::collections::HashMap;

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
    volume: f32,
    speed: f32,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    read: f32,
    voices: Vec<Voice>,
    recording: bool,
    playing: bool,
    now: usize,
    note_on_count: usize,
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
    pub fn start_playing(&mut self, pos: f32, note: u8, velocity: f32, params: &Params) {
        let read = (pos * self.data.len() as f32).round();
        let voice = Voice {
            note,
            read,
            volume: velocity,
            speed: 1.0,
        };
        self.voices.push(voice);
        self.note_on_count += 1;
        nih_warn!("** START PLAYING voice {}", note);
    }

    pub fn stop_playing(&mut self, note: u8, params: &Params) {
        for i in 0..self.voices.len() {
            if self.voices[i].note == note {
                self.voices.remove(i);
                break;
            }
        }
        self.note_on_count -= 1;
        nih_warn!("** STOP PLAYING: voice {note}");
    }

    pub fn start_recording(&mut self) {
        assert!(!self.recording);
        assert!(self.write == 0);
        self.recording = true;
        nih_warn!("** START RECORIDNG");
    }

    pub fn stop_recording(&mut self) {
        if self.recording {
            self.recording = false;
            self.data.truncate(self.write);
            self.write = 0;
            nih_warn!("** STOP RECORIDNG (wrote {} samples)", self.data.len());
        } else {
            nih_warn!("** STOP RECORDING: ALREADY STOPPED");
        }
    }

    pub fn process_sample<'a>(&mut self, sample: &mut f32, params: &Params) {
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

        for voice in self.voices.iter_mut() {
            if !self.data.is_empty() {
                let y = self.data[calc_sample_pos(self.data.len(), voice.read)];
                output += y * voice.volume;
                voice.read += voice.speed;
            };
        }

        if self.note_on_count == 0 {
            if params.auto_passthru {
                output += value;
            }
        }
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
    }
}

#[derive(Debug)]
pub struct Sampler {
    channels: Vec<Channel>,
}

impl Sampler {
    pub fn new(channel_count: usize) -> Self {
        Self {
            channels: vec![Channel::default(); channel_count],
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

    pub fn start_recording(&mut self) {
        self.each(Channel::start_recording);
    }

    pub fn stop_recording(&mut self) {
        self.each(Channel::stop_recording);
    }

    pub fn process_sample<'a>(
        &mut self,
        iter: impl IntoIterator<Item = &'a mut f32>,
        params: &Params,
    ) {
        for (i, sample) in iter.into_iter().enumerate() {
            self.channels[i].process_sample(sample, params);
        }
    }
}
