use std::fmt::Debug;

use nih_plug::nih_warn;

use crate::clip::Clip;
pub use crate::common_types::LoopMode;
use crate::common_types::{Params, RecordingMode};
use crate::recorder::Recorder;
use crate::utils::normalize_offset;
use crate::voice::Voice;
use crate::volume::Volume;

#[derive(Clone, Debug)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    reverse_speed: f32,
    recording_state: Recording,
    voices: Vec<Voice>,
    now: usize,
    passthru_on: bool,
    passthru_volume: Volume,
    last_recorded_offset: Option<usize>,
    errors: Vec<String>,
    recorder: Recorder,
}

#[derive(Clone, Debug, PartialEq)]
enum Recording {
    Idle,
    NoteTriggered,
    Always,
}

impl Default for Recording {
    fn default() -> Self {
        Recording::Idle
    }
}

#[derive(Clone, Default, Debug)]
pub struct VoiceInfo {
    pub start: f32,
    pub end: f32,
    pub pos: f32,
}

fn starting_offset(loop_start_percent: f32, data_len: usize) -> f32 {
    let len_f32 = data_len as f32;
    let start = loop_start_percent * len_f32;
    start
}

fn loop_length(loop_length_percent: f32, data_len: usize) -> f32 {
    let len_f32 = data_len as f32;
    let start = loop_length_percent * len_f32;
    start
}

impl Channel {
    fn new(params: &Params) -> Self {
        Channel {
            data: vec![],
            write: 0,
            reverse_speed: 1.0,
            voices: vec![],
            recording_state: Recording::Idle,
            now: 0,
            passthru_on: false,
            passthru_volume: Volume::new(if params.auto_passthru { 1.0 } else { 0.0 }),
            last_recorded_offset: None,
            errors: vec![],
            recorder: Recorder::new(),
        }
    }

    pub fn is_recording(&self) -> bool {
        match self.recording_state {
            Recording::NoteTriggered | Recording::Always => true,
            Recording::Idle => false,
        }
    }

    fn log(&self, params: &Params, s: String) {
        nih_warn!(
            "now={} voices={} voices(!finished)={} attack={} decay={} passthru_vol={:.2} {}",
            self.now,
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

    pub fn start_playing(
        &mut self,
        loop_start_percent: f32,
        note: u8,
        velocity: f32,
        params: &Params,
    ) {
        if self.data.is_empty() {
            return;
        }
        assert!(loop_start_percent >= 0.0 && loop_start_percent <= 1.0);
        let offset = loop_start_percent * self.data.len() as f32;
        let length = params.loop_length_percent * self.data.len() as f32;
        let mut voice = Voice {
            note,
            loop_start_percent,
            played: 0.0,
            clip: Clip::new(self.now, offset as usize, length as usize, 0, params.speed),
            ping_pong_speed: 1.0,
            volume: Volume::new(0.0),
            finished: false,
            last_sample_index: 0,
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
        match self.recording_state {
            Recording::Idle => {
                assert_eq!(self.write, 0);
                self.recording_state = Recording::NoteTriggered;
            }
            _ => {}
        }
    }

    pub fn stop_recording(&mut self, _params: &Params) {
        match self.recording_state {
            Recording::NoteTriggered => {
                self.recording_state = Recording::Idle;
                self.data.truncate(self.write);
                self.write = 0;
            }
            _ => {}
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

    fn play_voices(&mut self, params: &Params) -> f32 {
        let mut output = 0.0;
        let mut finished: Vec<usize> = vec![];
        for (i, voice) in self.voices.iter_mut().enumerate() {
            let speed = self.reverse_speed * params.speed;

            let len_f32 = self.data.len() as f32;

            voice.clip.update_speed(self.now, speed);
            voice
                .clip
                .update_length(self.now, (len_f32 * params.loop_length_percent) as usize);
            let offset = ((params.start_offset_percent + voice.loop_start_percent) * len_f32)
                .floor() as usize;
            voice.clip.update_offset(offset);
            let index = voice.clip.sample_index(self.now, self.data.len());
            let value = self.data[index] * voice.volume.value(self.now);

            output += value;
            voice.played += speed;
            voice.last_sample_index = index;

            if !voice.finished
                && params.loop_mode == LoopMode::PlayOnce
                && voice.played.abs() >= voice.clip.length() as f32
            {
                finished.push(i);
            }
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
        output
    }

    fn start_always_recording(&mut self, params: &Params) {
        assert_ne!(self.recording_state, Recording::Always);
        self.last_recorded_offset = None;
        self.recording_state = Recording::Always;
    }

    fn stop_always_recording(&mut self, params: &Params) {
        assert_eq!(self.recording_state, Recording::Always);
        self.recording_state = Recording::Idle;
    }

    fn handle_recording_state(&mut self, params: &Params) {
        match (params.recording_mode) {
            RecordingMode::AlwaysOn => match self.recording_state {
                Recording::Idle => self.start_always_recording(params),
                Recording::NoteTriggered => {
                    self.stop_recording(params);
                    self.start_always_recording(params);
                }
                Recording::Always => (),
            },
            RecordingMode::NoteTriggered => match self.recording_state {
                Recording::Idle => (),
                Recording::NoteTriggered => (),
                Recording::Always => self.stop_always_recording(params),
            },
        }
    }

    fn record_sample(&mut self, sample: f32, params: &Params) {
        match self.recording_state {
            Recording::Idle => {
                // do nothing
            }
            Recording::NoteTriggered => {
                assert!(self.write <= self.data.len());
                if self.write == self.data.len() {
                    self.data.push(sample);
                } else {
                    self.data[self.write] = sample;
                }
                self.last_recorded_offset = Some(self.write);
                self.write += 1;
            }
            Recording::Always => {
                if let Some(transport_pos_samples) = params.transport_pos_samples {
                    // running in Bitwig have seen transport_pos_samples < 0
                    // debug_assert!(
                    //     transport_pos_samples >= 0,
                    //     "transport_pos_samples={}", //     transport_pos_samples
                    // );
                    let offset = normalize_offset(
                        transport_pos_samples + params.sample_id as i64,
                        params.fixed_size_samples as i64,
                    );
                    assert!(offset >= 0, "offset={}", offset);
                    let offset = offset as usize;
                    self.data.resize(params.fixed_size_samples, 0.0);
                    self.data[offset] = sample;
                    if let Some(prev_offset) = self.last_recorded_offset {
                        if !(offset == 1 + prev_offset
                            || offset == 0 && prev_offset == params.fixed_size_samples - 1)
                        {
                            self.errors
                                .push(format!("skipped {} {}", offset, prev_offset));
                        }
                    }
                    self.last_recorded_offset = Some(offset);
                }
            }
        }
    }

    pub fn process_sample<'a>(&mut self, io: &mut f32, params: &Params) {
        let input = *io;

        self.handle_recording_state(params);
        self.record_sample(input, params);

        let mut output = 0.0;
        if !self.data.is_empty() {
            output += self.play_voices(params);
        }

        // passthru handling
        {
            // Sample processing
            // 1. Calculate output value based on state
            // 2. Make updates to state for next sample
            // 3. Update envolope values for next sample

            // its important output is calculated before updating state & volume
            let passhtru_value = self.passthru_volume.value(self.now);
            output += input * passhtru_value;

            // update volume
            self.passthru_volume.step(self.now);

            // update state
            self.handle_passthru(params);
        }

        self.now += 1;
        *io = output;
    }
}

#[derive(Clone, Debug)]
pub struct Sampler {
    channels: Vec<Channel>,
}

#[derive(Default, Clone, Debug)]
pub struct WaveformSummary {
    pub data: Vec<f32>,
    pub min: f32,
    pub max: f32,
}

impl Sampler {
    pub fn get_waveform_summary(&self, resolution: usize) -> WaveformSummary {
        let data = &self.channels[0].data;
        let step = data.len() as f32 / resolution as f32;
        let mut r = WaveformSummary {
            data: vec![0.0; resolution],
            min: 0.0,
            max: 0.0,
        };
        for i in 0..resolution {
            let a = ((i as f32) * step).floor() as usize;
            let b = (((i + 1) as f32) * step).floor() as usize;
            let n = (b - a) as f32;
            let value = (data[a..b].iter().map(|x| x * x).sum::<f32>() / n).sqrt();
            r.data[i] = value;
            r.min = r.min.min(value);
            r.max = r.max.max(value);
        }
        r
    }

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
        self.each(|ch| Channel::stop_recording(ch, params));
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

    pub fn process_frame<'a>(&mut self, frame: &mut [&'a mut f32], params: &Params) {
        for j in 0..frame.len() {
            self.channels[j].process_sample(frame[j], params);
        }
    }

    pub fn get_errors(&self) -> impl Iterator<Item = &str> {
        self.channels
            .iter()
            .flat_map(|c| c.errors.iter().map(|s| s.as_str()))
    }

    pub fn is_recording(&self) -> Vec<bool> {
        self.channels.iter().map(|x| x.is_recording()).collect()
    }

    pub fn get_last_recorded_offsets(&self) -> Vec<Option<usize>> {
        self.channels
            .iter()
            .map(|x| x.last_recorded_offset)
            .collect()
    }

    pub fn get_data_len(&self) -> usize {
        let ch = &self.channels[0];
        ch.data.len()
    }

    pub fn get_voice_info(&self, params: &Params) -> Vec<VoiceInfo> {
        let data_len_f32 = self.channels[0].data.len() as f32;

        self.channels[0]
            .voices
            .iter()
            .map(|v| {
                let start = v.loop_start_percent;
                let end = (v.loop_start_percent + params.loop_length_percent) % 1.0;
                let pos = v.last_sample_index as f32 / data_len_f32;
                VoiceInfo { start, end, pos }
            })
            .collect()
    }
}
