#![allow(unused)]
use nih_plug::prelude::*;
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
        let lenf = (self.data.len() as f32);
        let i = self.read % lenf;
        let i = if i < 0.0 { i + lenf } else { i };
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
        nih_warn!(
            "seek: pos={} self.read={} self.data.len()={}",
            pos,
            self.read,
            self.data.len()
        );
    }
}

#[derive(Clone, Default)]
struct Bufs {
    v: Vec<Buf>,
}

impl Bufs {
    pub fn clear(&mut self) {
        self.v.iter_mut().for_each(|b| b.clear());
    }
    pub fn rewind_read(&mut self) {
        self.v.iter_mut().for_each(|b| b.rewind_read());
    }
    pub fn rewind_write(&mut self) {
        self.v.iter_mut().for_each(|b| b.rewind_write());
    }
    pub fn calc_sample_pos(&self) -> usize {
        self.v[0].calc_sample_pos()
    }
    pub fn seek(&mut self, pos: f32) {
        self.v.iter_mut().for_each(|b| b.seek(pos));
    }
    pub fn ensure_channel_count(&mut self, count: usize) {
        if count >= self.v.len() {
            self.v.resize(count + 1, Buf::default());
        }
    }
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
    pass_thru: bool,
}
pub struct LiveSampler {
    params: Arc<LiveSamplerParams>,
    sample_rate: f32,
    buf: Bufs,
    count: usize,
    state: State,
}

#[derive(Params)]
struct LiveSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for LiveSamplerParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain 2",
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
        }
    }
}

impl Default for LiveSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(LiveSamplerParams::default()),
            sample_rate: -1.0,
            count: 0,
            buf: Bufs::default(),
            state: State::default(),
        }
    }
}

impl LiveSampler {
    fn toggle_pass_thru(&mut self, new_pass_thru: bool) {
        if new_pass_thru != self.state.pass_thru {
            let mut pass_thru = &mut self.state.pass_thru;
            nih_warn!(
                "toggle_pass_thru({new_pass_thru}): {} -> {}",
                *pass_thru,
                new_pass_thru
            );
            *pass_thru = new_pass_thru;
        } else {
            nih_warn!("toggle_pass_thru({new_pass_thru}): already {new_pass_thru}");
        }
    }
    fn toggle_reverse(&mut self, new_reverse: bool) {
        if let Some(Playing { speed, .. }) = &mut self.state.playing {
            let old_reverse = *speed < 0.0;
            if new_reverse != old_reverse {
                nih_warn!(
                    "toggle_reverse({new_reverse}): {} -> {}",
                    old_reverse,
                    new_reverse
                );
                *speed = -1.0 * *speed;
            } else {
                nih_warn!("toggle_reverse({new_reverse}): already {new_reverse}");
            }
        } else {
            nih_warn!("toggle_reverse({new_reverse}): not playing");
        }
    }
    fn start_playing(&mut self, pos: f32, speed: f32) {
        if self.state.playing.is_none() {
            self.buf.seek(pos);
            self.state.playing = Some(Playing { speed });
            nih_warn!("start_playing({pos}): ok")
        } else {
            nih_warn!("start_playing({pos}): already playing")
        }
    }

    fn start_recording(&mut self) {
        if !self.state.recording {
            self.buf.rewind_write();
            self.state.recording = true;
            nih_warn!("start_recording(): ok");
        } else {
            nih_warn!("start_recording(): already recording");
        }
    }

    fn stop_recording(&mut self) {
        if self.state.recording {
            self.state.recording = false;
            nih_warn!("stop_recording(): ok");
        } else {
            nih_warn!("start_recording(): recording has not been started");
        }
    }

    fn stop_playing(&mut self) {
        if self.state.playing.is_some() {
            nih_warn!("stop_playing(): ok");
            self.state.playing = None;
        } else {
            nih_warn!("stop_playing(): playing has not been started");
        }
    }
}

impl Plugin for LiveSampler {
    const NAME: &'static str = "Live Sampler";
    const VENDOR: &'static str = "Arunas Cesonis";
    const URL: &'static str = "https://github.com/arunas-cesonis/";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
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
        self.sample_rate = buffer_config.sample_rate;
        nih_warn!("initialize: sample_rate: {}", self.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.state = State::default();
        nih_warn!("reset: sample_rate: {}", self.sample_rate);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let channels = buffer.channels();
        self.buf.ensure_channel_count(channels);
        let mut next_event = context.next_event();
        let prev_state = self.state.clone();
        self.count += 1;
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
                            self.toggle_pass_thru(true);
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
                            self.toggle_pass_thru(false);
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
                        nih_warn!("ignore event {:?}", event);
                    }
                }
                next_event = context.next_event();
            }

            for (channel, sample) in channel_samples.iter_mut().enumerate() {
                let value = *sample;
                if self.state.recording {
                    self.buf.write(channel, value);
                }
                *sample = if let Some(Playing { speed }) = &mut self.state.playing {
                    let value = self.buf.read(channel, *speed);
                    value
                } else {
                    if self.state.pass_thru {
                        value
                    } else {
                        0.0
                    }
                };
            }
        }

        if self.state != prev_state {
            nih_warn!("state {:?} <- {:?}", self.state, prev_state);
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
