use crossbeam_queue::ArrayQueue;
use std::fmt::Debug;
use std::sync::Arc;
use std::thread::park_timeout_ms;

use log::log;
use nih_plug::nih_warn;
use nih_plug::prelude::Enum;

use crate::volume::Volume;

fn calc_sample_index_f32(len_f32: f32, read: f32) -> f32 {
    let i = read % len_f32;
    let i = if i < 0.0 { i + len_f32 } else { i };
    i
}

/**
 * calculate sample index from read position and read direction
 */
fn calc_sample_index(data_len: usize, read: f32, reverse: bool) -> usize {
    let index = calc_sample_index_f32(data_len as f32, read) as usize;
    if reverse {
        if index > 0 {
            index - 1
        } else {
            data_len - 1
        }
    } else {
        calc_sample_index_f32(data_len as f32, read) as usize
    }
}

#[derive(Clone, Debug, Default)]
struct Voice {
    note: u8,
    read: f32,
    played: f32,
    volume: Volume,
    start_percent: f32,
    speed: f32,
    speed_ping_pong: f32,
    finished: bool,
}

#[derive(Clone, Debug)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    reverse_speed: f32,
    voices: Vec<Voice>,
    recording: bool,
    now: usize,
    passthru_on: bool,
    passthru_volume: Volume,
}

#[derive(Debug, Enum, PartialEq, Clone)]
pub enum LoopMode {
    PlayOnce,
    PingPong,
    Loop,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub attack_samples: usize,
    pub decay_samples: usize,
    pub auto_passthru: bool,
    pub loop_mode: LoopMode,
    pub loop_length_percent: f32,
    pub speed: f32,
}

#[derive(Clone, Default, Debug)]
pub struct VoiceInfo {
    pub start: f32,
    pub end: f32,
    pub pos: f32,
}

#[derive(Clone, Default, Debug)]
pub struct Info {
    pub voices: Vec<VoiceInfo>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            auto_passthru: true,
            attack_samples: 100,
            loop_mode: LoopMode::PlayOnce,
            loop_length_percent: 1.0,
            decay_samples: 100,
            speed: 1.0,
        }
    }
}

impl Channel {
    fn new(params: &Params) -> Self {
        Channel {
            data: vec![],
            write: 0,
            reverse_speed: 1.0,
            voices: vec![],
            recording: false,
            now: 0,
            passthru_on: false,
            passthru_volume: Volume::new(if params.auto_passthru { 1.0 } else { 0.0 }),
        }
    }

    fn log(&self, params: &Params, s: String) {
        nih_warn!(
            "voices={} voices(!finished)={} attack={} decay={} passthru_vol={:.2} {}",
            self.voices.len(),
            self.voices.iter().filter(|v| !v.finished).count(),
            params.attack_samples,
            params.decay_samples,
            self.passthru_volume.value(self.now),
            s
        );
    }

    fn finish_voice(now: usize, voice: &mut Voice, params: &Params) {
        voice.volume.to(now, params.decay_samples, 0.0);
        voice.finished = true;
    }

    pub fn start_playing(&mut self, pos: f32, note: u8, velocity: f32, params: &Params) {
        assert!(pos >= 0.0 && pos <= 1.0);
        let read = (pos * self.data.len() as f32).round();
        let mut voice = Voice {
            note,
            read,
            start_percent: pos,
            played: 0.0,
            volume: Volume::new(0.0),
            speed: 1.0,
            speed_ping_pong: 1.0,
            finished: false,
        };
        voice.volume.to(self.now, params.attack_samples, velocity);
        self.voices.push(voice);
        self.handle_passthru(params);
        self.log(params, format!("START PLAYING note={}", note));
    }

    pub fn stop_playing(&mut self, note: u8, params: &Params) {
        for i in 0..self.voices.len() {
            let voice = &mut self.voices[i];
            if voice.note == note && !voice.finished {
                Self::finish_voice(self.now, voice, params);

                self.handle_passthru(params);
                self.log(params, format!("STOP PLAYING note={}", note));

                return;
            }
        }
        // This is not an error as some DAWs will send note off events for notes
        // that were never played, e.g. REAPER
        nih_warn!("could not find voice {note}")
    }

    pub fn reverse(&mut self, _params: &Params) {
        self.reverse_speed = -1.0;
    }

    pub fn unreverse(&mut self, _params: &Params) {
        self.reverse_speed = 1.0;
    }

    pub fn start_recording(&mut self, _params: &Params) {
        if !self.recording {
            assert_eq!(self.write, 0);
            self.recording = true;
        }
    }

    pub fn stop_recording(&mut self, _params: &Params) {
        // This is not an error as some DAWs will send note off events for notes
        // that were never played, e.g. REAPER
        if self.recording {
            self.recording = false;
            self.data.truncate(self.write);
            self.write = 0;
        }
    }

    fn handle_passthru(&mut self, params: &Params) {
        if params.auto_passthru {
            let have_unfinished_voices = self.voices.iter().any(|v| !v.finished);
            if !have_unfinished_voices {
                if !self.passthru_on {
                    self.passthru_on = true;
                    self.passthru_volume
                        .to(self.now, params.attack_samples, 1.0);
                }
            } else {
                if self.passthru_on {
                    self.passthru_on = false;
                    self.passthru_volume.to(self.now, params.decay_samples, 0.0);
                }
            }
        } else {
            if self.passthru_on {
                self.passthru_on = false;
                self.passthru_volume.to(self.now, params.decay_samples, 0.0);
            }
        }
    }

    fn record_sample(&mut self, value: f32) {
        assert!(self.recording && self.write <= self.data.len());
        if self.write == self.data.len() {
            self.data.push(value);
        } else {
            self.data[self.write] = value;
        }
        self.write += 1;
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
        let mut finished: Vec<usize> = vec![];
        for (i, voice) in self.voices.iter_mut().enumerate() {
            if !self.data.is_empty() {
                let len_f32 = self.data.len() as f32;
                let loop_start = voice.start_percent * len_f32;
                let loop_end = ((voice.start_percent + params.loop_length_percent) % 1.0) * len_f32;
                let loop_length = params.loop_length_percent * len_f32;
                // calculate playback speed
                let speed = voice.speed * self.reverse_speed * params.speed * voice.speed_ping_pong;

                // calculate sample index from read position and read direction
                let index = voice.read.floor() as usize;
                let index = if speed < 0.0 {
                    if index == 0 {
                        self.data.len() - 1
                    } else {
                        index - 1
                    }
                } else {
                    index
                };
                let value = self.data[index];
                output += value * voice.volume.value(self.now);
                let read = (voice.read + speed) % len_f32;
                let mut read = if read < 0.0 { read + len_f32 } else { read };
                eprintln!(
                    "mode={:?} s={} e={} len={} next={} voice.played={}",
                    params.loop_mode, loop_start, loop_end, loop_length, read, voice.played
                );
                let mut played = voice.played + speed;
                match params.loop_mode {
                    LoopMode::PlayOnce => {
                        if !voice.finished {
                            if played.abs() >= loop_length {
                                eprintln!("FINSHED");
                                finished.push(i);
                            }
                        }
                    }
                    LoopMode::Loop => {
                        if played.abs() >= loop_length {
                            let overrun = played.abs() % loop_length;
                            if speed > 0.0 {
                                played = overrun;
                                read = (loop_start + overrun) % len_f32;
                            } else {
                                played = -overrun;
                                read = loop_end - overrun;
                                if read < 0.0 {
                                    read += len_f32;
                                }
                            }
                        }
                    }
                    _ => (),
                };
                voice.played = played;
                voice.read = read;
            };
        }

        // remove voices that are finished and mute
        while let Some(j) = finished.pop() {
            Self::finish_voice(self.now, &mut self.voices[j], params);
        }

        // update voice volumes and find voices that can be removed (finished and mute)
        let mut removed = vec![];
        for (i, voice) in self.voices.iter_mut().enumerate() {
            voice.volume.step(self.now);
            if voice.volume.is_static_and_mute() && voice.finished {
                removed.push(i);
            }
        }

        // remove voices that are finished and mute
        while let Some(j) = removed.pop() {
            self.voices.remove(j);
        }

        /**
         * passthru handling
         */
        {
            // Sample processing
            // 1. Calculate output value based on state
            // 2. Make updates to state for next sample
            // 3. Update envolope values for next sample

            // its important output is calculated before updating state & volume
            let passhtru_value = self.passthru_volume.value(self.now);
            output += value * passhtru_value;

            // update volume
            self.passthru_volume.step(self.now);

            // update state
            self.handle_passthru(params);
            //eprintln!(
            //    "now={} passthru_on={} passthru={:?} passhtru_value={:.2} output={:.2}",
            //    self.now, self.passthru_on, self.passthru_volume, passhtru_value, output
            //);
        }

        self.now += 1;
        *sample = output;
    }
}

#[derive(Clone, Debug)]
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

    pub fn reverse(&mut self, params: &Params) {
        self.each(|ch| Channel::reverse(ch, params));
    }

    pub fn unreverse(&mut self, params: &Params) {
        self.each(|ch| Channel::unreverse(ch, params));
    }

    pub fn stop_recording(&mut self, params: &Params) {
        self.each(|ch| Channel::stop_recording(ch, params))
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

    pub fn process_frame<'a>(&mut self, mut frame: &mut [&'a mut f32], params: &Params) {
        for j in 0..frame.len() {
            self.channels[j].process_sample(frame[j], params);
        }
    }

    pub fn get_info(&self, params: &Params) -> Info {
        let data_len_f32 = self.channels[0].data.len() as f32;
        Info {
            voices: self.channels[0]
                .voices
                .iter()
                .map(|v| {
                    let start = v.start_percent;
                    let end = (v.start_percent + params.loop_length_percent) % 1.0;
                    let pos = calc_sample_index_f32(data_len_f32, v.read) / data_len_f32;
                    VoiceInfo { start, end, pos }
                })
                .collect(),
        }
    }
}

//struct ProcessIter<'a, I> {
//    sampler: &'a mut Sampler,
//    iter: I,
//    params: &'a Params,
//}
