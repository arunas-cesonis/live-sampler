use std::fmt::Debug;

use crate::clip;
use crate::clip::Clip;
pub use crate::common_types::LoopMode;
use crate::common_types::{InitParams, Note, Params};
use crate::recorder::Recorder;
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
    pub(crate) next_voice_id: usize,
}

#[derive(Clone, Default, Debug)]
pub struct VoiceInfo {
    pub start: f32,
    pub end: f32,
    pub pos: f32,
}

fn starting_offset(loop_start_percent: f32, data_len: usize) -> f32 {
    let len_f32 = data_len as f32;
    loop_start_percent * len_f32
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
            next_voice_id: 0,
        }
    }

    pub fn recorder(&self) -> &Recorder {
        &self.recorder
    }

    fn finish_voice(&mut self, now: usize, index: usize, params: &Params) {
        let voice = &mut self.voices[index];
        assert!(!voice.finished);
        //eprintln!("now={} stop playing voice={:?}", self.now, voice);
        voice.volume.to(now, params.decay_samples, 0.0);
        voice.finished_at = now;
        voice.finished = true;
    }

    pub fn set_note_speed(&mut self, note: Note, speed: f32) {
        for v in &mut self.voices {
            if v.note == note {
                v.speed = speed;
                return;
            }
        }
        #[cfg(debug_assertions)]
        {
            panic!("set_note_speed: note not found: {:?}", note);
        }
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
        let offset = starting_offset(loop_start_percent, self.data.len());
        let length = params.loop_length(self.data.len());
        let clip2 = Clip::new(
            self.now,
            offset,
            params.speed(),
            length,
            self.data.len() as clip::T,
            match params.loop_mode {
                LoopMode::Loop | LoopMode::PlayOnce => clip::Mode::Loop,
                LoopMode::PingPong => clip::Mode::PingPong,
            },
        );
        let mut voice = Voice {
            note: note,
            loop_start_percent,
            played: 0.0,
            clip2,
            volume: Volume::new(0.0),
            finished: false,
            is_at_zero_crossing: false,
            finished_at: 0,
            last_sample_index: 0,
            last_sample_value: 0.0,
            speed: 1.0,
        };
        self.next_voice_id += 1;
        voice.volume.to(self.now, params.attack_samples, velocity);
        // #[cfg(debug_assertions)]
        // nih_warn!("start_playing: voice={:?}", voice);
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

    fn should_remove_voice(now: usize, voice: &Voice, params: &Params) -> bool {
        if voice.finished {
            match params.note_off_behavior {
                crate::common_types::NoteOffBehaviour::ZeroCrossing => {
                    if now - voice.finished_at >= params.decay_samples {
                        return true;
                    }
                    voice.is_at_zero_crossing
                }
                crate::common_types::NoteOffBehaviour::Decay => voice.volume.is_static_and_mute(),
                crate::common_types::NoteOffBehaviour::DecayAndZeroCrossing => {
                    if !voice.volume.is_static_and_mute() {
                        if now - voice.finished_at >= params.decay_samples {
                            return true;
                        }
                        voice.is_at_zero_crossing
                    } else {
                        true
                    }
                }
            }
        } else {
            false
        }
    }

    fn play_voices(&mut self, params: &Params) -> f32 {
        let mut output = 0.0;
        let mut finished: Vec<usize> = vec![];
        for (i, voice) in self.voices.iter_mut().enumerate() {
            // prevents voice playing 1 unnecessary
            // sample at the end when voice is cancelled by note and does not have any decay time
            if Self::should_remove_voice(self.now, voice, params) {
                continue;
            }
            let voice_speed = voice.speed * params.speed();

            voice
                .clip2
                .update_length(self.now, params.loop_length(self.data.len()) as clip::T);
            voice.clip2.update_speed(self.now, voice_speed);
            voice
                .clip2
                .update_data_length(self.now, self.data.len() as clip::T);
            voice.clip2.update_mode(
                self.now,
                match params.loop_mode {
                    LoopMode::Loop | LoopMode::PlayOnce => clip::Mode::Loop,
                    LoopMode::PingPong => clip::Mode::PingPong,
                },
            );
            let index = voice.clip2.offset(self.now).floor() as usize;

            let value = self.data[index] * voice.volume.value(self.now);

            output += value;
            voice.played += voice_speed;
            voice.is_at_zero_crossing =
                value.signum() != voice.last_sample_value.signum() || value == 0.0;
            voice.last_sample_index = index;
            voice.last_sample_value = value;

            if !voice.finished
                && params.loop_mode == LoopMode::PlayOnce
                && voice.played.abs() >= params.loop_length(self.data.len()).floor()
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
            if Self::should_remove_voice(self.now, voice, params) {
                removed.push(i);
            }
        }

        // remove voices that are finished and mute
        while let Some(j) = removed.pop() {
            // #[cfg(debug_assertions)]
            // nih_warn!("removing: voice={:?}", self.voices[j]);
            self.voices.remove(j);
        }
        output
    }

    pub fn process_sample<'a>(&mut self, input: f32, params: &Params) -> f32 {
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
        output
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
    pub fn print_error_info(&self, channel: usize) -> String {
        self.channels[channel].recorder().print_error_info()
    }
    pub fn iter_active_notes(&self, channel: usize) -> impl Iterator<Item = Note> + '_ {
        self.channels[channel].voices.iter().filter_map(|v| {
            if !v.finished {
                Some(v.note)
            } else {
                None
            }
        })
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
            channels: vec![Channel::new(params); channel_count],
        }
    }
    fn each<F>(&mut self, f: F)
    where
        F: FnMut(&mut Channel),
    {
        self.channels.iter_mut().for_each(f)
    }

    pub fn set_note_speed(&mut self, note: Note, speed: f32) {
        self.each(|ch| ch.set_note_speed(note, speed))
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

    pub fn process_sample<'a>(&mut self, channel: usize, input: f32, params: &Params) -> f32 {
        self.channels[channel].process_sample(input, params)
    }

    pub fn process_frame<'a>(&mut self, frame: &mut [&'a mut f32], params: &Params) {
        for j in 0..frame.len() {
            *frame[j] = self.process_sample(j, *frame[j], params);
        }
    }

    pub fn get_frames_processed(&self, channel: usize) -> usize {
        self.channels[channel].now
    }

    pub fn is_recording(&self, channel: usize) -> bool {
        let yes = self.channels[channel].recorder.is_recording();
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

    pub fn get_data_len(&self, channel: usize) -> usize {
        let ch = &self.channels[channel];
        ch.data.len()
    }

    pub fn get_voice_info(&self, channel: usize, params: &Params) -> Vec<VoiceInfo> {
        let data_len_f32 = self.channels[channel].data.len() as f32;

        let l = params.loop_length(self.get_data_len(0));
        self.channels[channel]
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

    #[cfg(debug_assertions)]
    pub fn dump_crash_info(&mut self) {
        let data_lengths: Vec<_> = self
            .channels
            .iter()
            .map(|ch| ch.data.len())
            .collect::<Vec<_>>();
        self.channels.iter_mut().for_each(|ch| ch.data.clear());
        eprintln!(
            "sampler just before death: {:#?}\ndatas have been clear, had lengths: {:?}",
            self, data_lengths
        );
        let count = self.channels[0].voices.len();
        for (i, v) in self.channels[0].voices.iter().enumerate() {
            eprintln!("voice[{} of {}]: {:?}", i, count, v);
        }
    }
}
