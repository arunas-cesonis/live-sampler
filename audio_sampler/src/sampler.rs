use std::fmt::Debug;

use crate::clip::Clip;
pub use crate::common_types::LoopMode;
use crate::common_types::{InitParams, Note, Params, RecordingMode};
use crate::recorder;
use crate::recorder::Recorder;
use crate::time_value::{TimeOrRatio, TimeValue};
use crate::utils::normalize_offset;
use crate::voice::Voice;
use crate::volume::Volume;

#[derive(Clone, Debug)]
pub(crate) struct Channel {
    pub(crate) data: Vec<f32>,
    pub(crate) voices: Vec<Voice>,
    pub(crate) now: usize,
    pub(crate) passthru_on: bool,
    pub(crate) passthru_volume: Volume,
    pub(crate) recorder: Recorder,
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

impl Channel {
    fn reset(&mut self) {
        self.data.clear();
        self.voices.clear();
        self.now = 0;
        self.passthru_on = false;
        self.passthru_volume = Volume::new(0.0);
        self.recorder = Recorder::new();
    }
    fn new(params: &InitParams) -> Self {
        Channel {
            data: vec![],
            voices: vec![],
            now: 0,
            passthru_on: false,
            passthru_volume: Volume::new(if params.auto_passthru { 1.0 } else { 0.0 }),
            recorder: Recorder::new(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recorder().is_recording()
    }

    pub fn recorder(&self) -> &Recorder {
        &self.recorder
    }

    fn finish_voice(&mut self, now: usize, index: usize, params: &Params) {
        let voice = &mut self.voices[index];
        assert!(!voice.finished);
        //eprintln!("now={} stop playing voice={:?}", self.now, voice);
        let channel_index: usize = voice.note.channel.into();
        voice.volume.to(now, params.decay_samples, 0.0);
        voice.finished = true;
    }

    pub fn start_playing(
        &mut self,
        loop_start_percent: f32,
        note: Note,
        velocity: f32,
        params: &Params,
    ) {
        if self.data.is_empty() {
            return;
        }

        assert!(loop_start_percent >= 0.0 && loop_start_percent <= 1.0);
        let offset = loop_start_percent * self.data.len() as f32;
        let length: f32 = params.loop_length(self.data.len());
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
        eprintln!("now={} start playing voice={:?}", self.now, voice);
        voice.volume.to(self.now, params.attack_samples, velocity);
        self.voices.push(voice);
        self.handle_passthru(params);
    }

    pub fn stop_playing(&mut self, note: Note, params: &Params) {
        // None is not an error here as some DAWs will send note off events for notes
        // that were never played, e.g. REAPER
        if let Some(i) = self
            .voices
            .iter()
            .position(|v| v.note == note && !v.finished)
        {
            self.finish_voice(self.now, i, params);
            self.handle_passthru(params);
        }
    }

    pub fn start_recording(&mut self, params: &Params) {
        self.recorder.start(&mut self.data, &params.into());
    }

    pub fn stop_recording(&mut self, params: &Params) {
        self.recorder.stop(&mut self.data, &params.into());
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

    fn should_remove_voice(voice: &Voice, params: &Params) -> bool {
        voice.volume.is_static_and_mute() && voice.finished
    }

    fn play_voices(&mut self, params: &Params) -> f32 {
        let mut output = 0.0;
        let mut finished: Vec<usize> = vec![];
        for (i, voice) in self.voices.iter_mut().enumerate() {
            // prevents voice playing 1 unnecessary
            // sample at the end when voice is cancelled by note and does not have any decay time
            if Self::should_remove_voice(voice, params) {
                continue;
            }

            let speed = params.speed();

            let len_f32 = self.data.len() as f32;

            voice.clip.update_speed(self.now, speed);
            voice
                .clip
                .update_length(self.now, params.loop_length(self.data.len()) as usize);
            let offset = ((params.start_offset_percent + voice.loop_start_percent) * len_f32)
                .floor() as usize;
            voice.clip.update_offset(offset);
            let index = voice.clip.sample_index(self.now, self.data.len());
            let value = self.data[index] * voice.volume.value(self.now);
            // eprintln!(
            //     "self.now={} play value={} voice={:?}",
            //     self.now, value, voice
            // );

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
            self.finish_voice(self.now, j, params);
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

    pub fn process_sample<'a>(&mut self, io: &mut f32, params: &Params) {
        let input = *io;

        self.recorder
            .process_sample(input, &mut self.data, &params.into());

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

        //eprintln!("self.now={} play output={}", self.now, output);
        self.now += 1;
        *io = output;
    }
}

#[derive(Clone, Debug)]
pub struct Sampler {
    pub(crate) channels: Vec<Channel>,
}

#[derive(Default, Clone, Debug)]
pub struct WaveformSummary {
    pub data: Vec<f32>,
    pub min: f32,
    pub max: f32,
}

impl Sampler {
    pub fn reset(&mut self) {
        self.channels.iter_mut().for_each(|ch| {
            ch.reset();
        });
    }
    pub fn print_error_info(&self) -> String {
        self.channels[0].recorder().print_error_info()
    }
    pub fn iter_active_notes(&self) -> impl Iterator<Item = Note> + '_ {
        self.channels[0]
            .voices
            .iter()
            .filter_map(|v| if !v.finished { Some(v.note) } else { None })
    }
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

    pub fn new(channel_count: usize, params: &InitParams) -> Self {
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

    pub fn start_playing(&mut self, pos: f32, note: Note, velocity: f32, params: &Params) {
        self.each(|ch| ch.start_playing(pos, note, velocity, params));
    }

    pub fn stop_playing(&mut self, note: Note, params: &Params) {
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
            self.channels[i].process_sample(sample, params);
        }
    }

    pub fn process_frame<'a>(&mut self, frame: &mut [&'a mut f32], params: &Params) {
        for j in 0..frame.len() {
            self.channels[j].process_sample(frame[j], params);
        }
    }

    pub fn get_frames_processed(&self) -> usize {
        self.channels[0].now
    }

    pub fn is_recording(&self) -> bool {
        let yes = self.channels[0].recorder.is_recording();
        debug_assert!(
            self.channels
                .iter()
                .all(|x| x.recorder.is_recording() == yes),
            "is_recording mismatch"
        );
        yes
    }

    pub fn get_last_recorded_offsets(&self) -> Vec<Option<usize>> {
        self.channels
            .iter()
            .map(|x| x.recorder().last_recorded_offset())
            .collect()
    }

    pub fn get_data_len(&self) -> usize {
        let ch = &self.channels[0];
        ch.data.len()
    }

    pub fn get_voice_info(&self, params: &Params) -> Vec<VoiceInfo> {
        let data_len_f32 = self.channels[0].data.len() as f32;

        let l = params.loop_length(self.get_data_len());
        self.channels[0]
            .voices
            .iter()
            .map(|v| {
                let start = v.loop_start_percent;
                let end = (v.loop_start_percent + l / data_len_f32) % 1.0;
                let pos = v.last_sample_index as f32 / data_len_f32;
                VoiceInfo { start, end, pos }
            })
            .collect()
    }
}
