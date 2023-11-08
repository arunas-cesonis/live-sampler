#![allow(unused)]
mod volume_env;

use crate::volume_env::{PolyVolumeEnv, VolumeEnv};
use nih_plug::prelude::*;
use std::ops::DerefMut;
use std::sync::Arc;

#[derive(Clone, Default)]
struct Buf {
    read: f32,
    write: usize,
    data: Vec<f32>,
}

impl Buf {
    pub fn new() -> Self {
        Self {
            read: 0.0,
            write: 0,
            data: vec![],
        }
    }
    pub fn write(&mut self, value: f32) {
        assert!(self.write <= self.data.len());
        if self.write == self.data.len() {
            self.data.push(value);
        } else {
            self.data[self.write] = value;
        }
        self.write += 1;
    }

    pub fn calc_sample_pos(&self) -> usize {
        let len_f32 = (self.data.len() as f32);
        let i = self.read % len_f32;
        let i = if i < 0.0 { i + len_f32 } else { i };
        let i = i as usize;
        i
    }

    pub fn read(&mut self, speed: f32) -> f32 {
        if self.data.is_empty() {
            0.0
        } else {
            let i = self.calc_sample_pos();
            self.read += speed;
            self.data[i]
        }
    }

    pub fn rewind_write(&mut self) {
        self.write = 0;
    }

    pub fn truncate_written(&mut self) {
        self.data.truncate(self.write);
        self.write = 0;
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.read = 0.0;
        self.write = 0;
    }

    pub fn rewind_read(&mut self) {
        self.read = 0.0;
    }

    pub fn seek(&mut self, pos: f32) {
        assert!(
            pos >= 0.0 && pos <= 1.0,
            "pos is not in range 0.0 1.0: {}",
            pos
        );
        self.read = ((self.data.len() as f32) * pos);
    }
}

#[derive(Clone, Default)]
struct Bufs {
    v: Vec<Buf>,
}

impl Bufs {
    pub fn new(channel_count: usize) -> Self {
        Bufs {
            v: vec![Buf::new(); channel_count],
        }
    }
    pub fn len(&self) -> usize {
        self.v.len()
    }
    pub fn clear(&mut self) {
        self.v.iter_mut().for_each(|b| b.clear());
    }
    pub fn rewind_read(&mut self) {
        self.v.iter_mut().for_each(|b| b.rewind_read());
    }
    pub fn rewind_write(&mut self) {
        self.v.iter_mut().for_each(|b| b.rewind_write());
    }
    #[cfg(debug_assertions)]
    pub fn check_write_positions_at_zero(&self) {
        assert!(self.v.iter().all(|b| b.write == 0))
    }
    pub fn truncate_written(&mut self) {
        self.v.iter_mut().for_each(|b| b.truncate_written());
    }
    pub fn calc_sample_pos(&self) -> usize {
        self.v[0].calc_sample_pos()
    }
    pub fn seek(&mut self, pos: f32) {
        self.v.iter_mut().for_each(|b| b.seek(pos));
    }
    //pub fn ensure_channel_count(&mut self, count: usize) {
    //    if count >= self.v.len() {
    //        self.v.resize(count + 1, Buf::default());
    //    }
    //}
    pub fn write(&mut self, channel: usize, value: f32) {
        self.v[channel].write(value);
    }
    pub fn read(&mut self, channel: usize, speed: f32) -> f32 {
        self.v[channel].read(speed)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Playing {
    speed: f32,
}

#[derive(Clone, Default, Debug, PartialEq)]
struct State {
    recording: bool,
    playing: Option<Playing>,
    passthru: bool,
}
pub struct LiveSampler {
    audio_io_layout: AudioIOLayout,
    params: Arc<LiveSamplerParams>,
    sample_rate: f32,
    buf: Bufs,
    volume: PolyVolumeEnv,
    passthru_volume: PolyVolumeEnv,
    now: Vec<usize>,
    state: State,
}

#[derive(Params)]
struct LiveSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "passthru"]
    pub passthru: BoolParam,
    #[id = "speed"]
    pub speed: FloatParam,
    #[id = "fade time"]
    pub fade_time: FloatParam,
}

impl Default for LiveSamplerParams {
    fn default() -> Self {
        Self {
            passthru: BoolParam::new("Pass through", true),
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            speed: FloatParam::new(
                "Speed",
                1.0,
                FloatRange::Linear {
                    min: 0.125,
                    max: 1.0,
                },
            ),
            fade_time: FloatParam::new(
                "Fade time",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1000.0,
                },
            )
            .with_unit(" samples"), //with_smoother(SmoothingStyle::Logarithmic(50.0))
                                    //.with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                                    //.with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Default for LiveSampler {
    fn default() -> Self {
        Self {
            audio_io_layout: AudioIOLayout::default(),
            params: Arc::new(LiveSamplerParams::default()),
            sample_rate: -1.0,
            buf: Bufs::default(),
            volume: PolyVolumeEnv::new(Vec::new()),
            passthru_volume: PolyVolumeEnv::new(Vec::new()),
            state: State::default(),
            now: Vec::new(),
        }
    }
}

impl LiveSampler {
    fn channel_count(&self) -> usize {
        let channel_count: usize = self
            .audio_io_layout
            .main_output_channels
            .unwrap()
            .get()
            .try_into()
            .unwrap();
        channel_count
    }
    fn toggle_passthru(&mut self, new_passthru: bool) {
        if new_passthru != self.state.passthru {
            let mut passthru = &mut self.state.passthru;
            *passthru = new_passthru;
        }
    }
    fn toggle_reverse(&mut self, new_reverse: bool) {
        if let Some(Playing { speed, .. }) = &mut self.state.playing {
            let old_reverse = *speed < 0.0;
            if new_reverse != old_reverse {
                *speed = -1.0 * *speed;
            }
        }
    }

    fn log_prefix(&self) -> String {
        let seconds = format!("{:.3}s", (self.now[0] as f32) / self.sample_rate);
        format!(
            "{:>5} {:>5} {:>5}",
            seconds,
            self.now[0],
            self.volume.value(0, self.now[0])
        )
    }

    fn start_playing(&mut self, pos: f32, speed: f32) {
        if self.state.playing.is_none() {
            self.buf.seek(pos);
            self.state.playing = Some(Playing { speed });
            self.fade(1.0, None);
            self.fade_passthru(0.0, None);
        }
    }

    fn passthru_active(&self) -> bool {
        self.state.passthru || self.params.passthru.value()
    }

    fn start_recording(&mut self) {
        if !self.state.recording {
            nih_warn!("{} start playing", self.log_prefix());
            #[cfg(debug_assertions)]
            self.buf.check_write_positions_at_zero();
            self.state.recording = true;
        }
    }

    fn stop_recording(&mut self) {
        if self.state.recording {
            // Vec::drain
            self.buf.truncate_written();
            self.state.recording = false;
        }
    }

    fn fade_passthru(&mut self, target: f32, time: Option<usize>) {
        self.passthru_volume.retrigger(
            &self.now,
            &self.now,
            time.unwrap_or_else(|| self.params.fade_time.value() as usize),
            target,
        );
    }

    fn fade(&mut self, target: f32, time: Option<usize>) {
        self.volume.retrigger(
            &self.now,
            &self.now,
            time.unwrap_or_else(|| self.params.fade_time.value() as usize),
            target,
        );
    }

    fn stop_playing(&mut self) {
        if self.state.playing.is_some() {
            nih_warn!("{} stop playing", self.log_prefix());
            self.state.playing = None;
            // FIXME: Clicks. Cut at zero crossing. Try lookahead within buffer with better logging again. Try half-note mode then.
            self.fade(0.0, None);
            self.fade_passthru(1.0, None);
        }
    }
}

impl Plugin for LiveSampler {
    const NAME: &'static str = "Live Sampler";
    const VENDOR: &'static str = "Arunas Cesonis";
    const URL: &'static str = "https://github.com/arunas-cesonis/";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[],
            aux_output_ports: &[],

            // Individual ports and the layout as a whole can be named here. By default these names
            // are generated as needed. This layout will be called 'Stereo', while the other one is
            // given the name 'Mono' based no the number of input and output channels.
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    type SysExMessage = ();

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.audio_io_layout = audio_io_layout.clone();
        self.sample_rate = buffer_config.sample_rate;
        let channel_count: usize = self.channel_count();
        self.volume = PolyVolumeEnv::new(vec![0.0; channel_count]);
        self.passthru_volume = PolyVolumeEnv::new(vec![1.0; channel_count]);
        self.buf = Bufs::new(channel_count);
        self.now = vec![0; channel_count];
        nih_warn!("{} initialize", self.log_prefix());
        true
    }

    fn reset(&mut self) {
        let channel_count: usize = self.channel_count();
        self.buf.clear();
        self.state = State::default();
        self.volume = PolyVolumeEnv::new(vec![0.0; channel_count]);
        self.passthru_volume = PolyVolumeEnv::new(vec![1.0; channel_count]);
        nih_warn!("{} reset", self.log_prefix());
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let channels = buffer.channels();
        let mut next_event = context.next_event();
        let prev_state = self.state.clone();

        let params_speed = self.params.speed.smoothed.next();
        let params_gain = self.params.gain.smoothed.next();
        let params_fade_time = self.params.fade_time.smoothed.next() as usize;
        let num_samples = buffer.samples();
        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    //nih_warn!("discard sample_id={} event={:?}", sample_id, event);
                    break;
                }
                //nih_warn!("USE sample_id={} event={:?}", sample_id, event);
                nih_warn!("sample_id={} event={:?}", sample_id, event);
                match event {
                    NoteEvent::NoteOn { note, .. } => match note {
                        47 => {
                            self.toggle_passthru(true);
                        }
                        48 => {
                            self.start_recording();
                        }
                        49 => {
                            self.toggle_reverse(true);
                        }
                        60..=75 => {
                            let pos = (note - 60) as f32 / 16.0;
                            self.start_playing(pos, 1.0);
                        }
                        _ => (),
                    },
                    NoteEvent::NoteOff { note, .. } => match note {
                        47 => {
                            self.toggle_passthru(false);
                        }
                        48 => {
                            self.stop_recording();
                        }
                        49 => {
                            self.toggle_reverse(false);
                        }
                        60..=75 => {
                            self.stop_playing();
                        }
                        _ => (),
                    },
                    _ => {
                        // nih_warn!("ignore event {:?}", event);
                    }
                }
                next_event = context.next_event();
            }

            for (channel, sample) in channel_samples.iter_mut().enumerate() {
                let value = *sample;
                if self.state.recording {
                    self.buf.write(channel, value);
                }
                let new_sample = if let Some(Playing { speed }) = &mut self.state.playing {
                    let value = self.buf.read(channel, *speed * params_speed);
                    let volume = self.volume.value(channel, self.now[channel]);
                    // Maybe
                    let value = value * volume;
                    value
                } else {
                    if self.passthru_active() {
                        let volume = self.passthru_volume.value(channel, self.now[channel]);
                        value * volume
                    } else {
                        0.0
                    }
                };
                *sample = new_sample * gain;
                self.now[channel] += 1;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for LiveSampler {
    const CLAP_ID: &'static str = "com.livesampler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Live sampler");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for LiveSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"LiveSamplerPlugi";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(LiveSampler);
nih_export_vst3!(LiveSampler);
