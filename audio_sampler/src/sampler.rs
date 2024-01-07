use std::fmt::Debug;

use nih_plug::nih_warn;

use crate::volume::Volume;

fn calc_sample_pos_f32(data_len: usize, read: f32) -> f32 {
    let len_f32 = data_len as f32;
    let i = read % len_f32;
    let i = if i < 0.0 { i + len_f32 } else { i };
    i
}

fn calc_sample_pos(data_len: usize, read: f32) -> usize {
    calc_sample_pos_f32(data_len, read) as usize
}

#[derive(Clone, Debug, Default)]
struct Voice {
    note: u8,
    read: f32,
    volume: Volume,
    speed: f32,
    finished: bool,
}

#[derive(Clone, Debug)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    global_speed: f32,
    voices: Vec<Voice>,
    recording: bool,
    now: usize,
    note_on_count: usize,
    passthru_on: bool,
    passthru_volume: Volume,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub attack_samples: usize,
    pub decay_samples: usize,
    pub auto_passthru: bool,
    pub speed: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            auto_passthru: true,
            attack_samples: 100,
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
            global_speed: 1.0,
            voices: vec![],
            recording: false,
            now: 0,
            note_on_count: 0,
            passthru_on: false,
            passthru_volume: Volume::new(if params.auto_passthru { 1.0 } else { 0.0 }),
        }
    }

    fn log(&self, params: &Params, s: String) {
        nih_warn!(
            "voices={} note_on={} attack={} decay={} passthru_vol={:.2} {}",
            self.voices.len(),
            self.note_on_count,
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
        let read = (pos * self.data.len() as f32).round();
        let mut voice = Voice {
            note,
            read,
            volume: Volume::new(0.0),
            speed: 1.0,
            finished: false,
        };
        voice.volume.to(self.now, params.attack_samples, velocity);
        self.voices.push(voice);
        self.note_on_count += 1;
        self.handle_passthru(params);
        self.log(params, format!("START PLAYING note={}", note));
    }

    pub fn stop_playing(&mut self, note: u8, params: &Params) {
        for i in 0..self.voices.len() {
            let voice = &mut self.voices[i];
            if voice.note == note && !voice.finished {
                Self::finish_voice(self.now, voice, params);

                self.note_on_count -= 1;
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
        self.global_speed = -1.0;
    }

    pub fn unreverse(&mut self, _params: &Params) {
        self.global_speed = 1.0;
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
            if self.note_on_count == 0 {
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
        let mut removed = vec![];
        for (i, voice) in self.voices.iter_mut().enumerate() {
            if !self.data.is_empty() {
                let y = self.data[calc_sample_pos(self.data.len(), voice.read)];
                output += y * voice.volume.value(self.now);
                voice.read += voice.speed * self.global_speed * params.speed;
            };
            voice.volume.step(self.now);
            if voice.volume.is_static_and_mute() && voice.finished {
                removed.push(i);
            }
        }

        while let Some(j) = removed.pop() {
            self.voices.remove(j);
        }

        self.handle_passthru(params);

        // if (params.auto_passthru
        //     && self.passthru_volume.is_static_and_mute()
        //     && self.note_on_count == 0)
        // {
        //     nih_error!("{}", self.dump_before_death());
        //     panic!("unexpected state");
        // }
        output += value * self.passthru_volume.value(self.now);
        self.passthru_volume.step(self.now);

        self.now += 1;
        *sample = output;
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
}
