#![allow(unused)]
use nih_plug::prelude::*;
use std::sync::Arc;

#[derive(Clone, Default)]
struct Buf {
    read: usize,
    data: Vec<f32>,
}

impl Buf {
    pub fn new() -> Self {
        Self {
            read: 0,
            data: vec![],
        }
    }
    pub fn write(&mut self, value: f32) {
        self.data.push(value);
    }
    pub fn read(&mut self) -> f32 {
        if self.data.is_empty() {
            0.0
        } else {
            let i = self.read % self.data.len();
            self.read += 1;
            self.data[i]
        }
    }
    pub fn clear(&mut self) {
        self.data.clear();
        self.read = 0;
    }
    pub fn rewind(&mut self) {
        self.read = 0;
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
    pub fn rewind(&mut self) {
        self.v.iter_mut().for_each(|b| b.rewind());
    }
    pub fn ensure_channel_count(&mut self, count: usize) {
        if count >= self.v.len() {
            self.v.resize(count + 1, Buf::default());
        }
    }
    pub fn write(&mut self, channel: usize, value: f32) {
        self.v[channel].write(value);
    }
    pub fn read(&mut self, channel: usize) -> f32 {
        self.v[channel].read()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Idle,
    Playing,
    Recording,
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
            state: State::Idle,
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
    type SysExMessage = ();
    type BackgroundTask = ();

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

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
        self.state = State::Idle;
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
        let prev_state = self.state;
        self.count += 1;
        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();

            while let Some(event) = next_event {
                if (event.timing() != sample_id as u32) {
                    //nih_warn!("discard sample_id={} event={:?}", sample_id, event);
                    break;
                }
                //nih_warn!("USE sample_id={} event={:?}", sample_id, event);
                match event {
                    NoteEvent::NoteOn { note, .. } => match note {
                        48 => match self.state {
                            State::Idle | State::Playing => {
                                self.buf.clear();
                                self.state = State::Recording;
                                nih_warn!("start recording");
                            }
                            State::Recording => {
                                nih_warn!("already recording");
                            }
                        },
                        50 => match self.state {
                            State::Idle | State::Recording => {
                                nih_warn!("start playing");
                                self.state = State::Playing;
                            }
                            State::Playing => {
                                nih_warn!("already playing");
                            }
                        },
                        _ => (),
                    },
                    NoteEvent::NoteOff { note, .. } => match note {
                        48 => match self.state {
                            State::Recording => {
                                self.state = State::Idle;
                            }
                            State::Idle => (),
                            State::Playing => (),
                        },
                        50 => match self.state {
                            State::Recording => (),
                            State::Idle => (),
                            State::Playing => {
                                self.state = State::Idle;
                                self.buf.rewind();
                            }
                        },
                        _ => (),
                    },
                    _ => {
                        nih_warn!("ignore event {:?}", event);
                    }
                }
                next_event = context.next_event();
            }

            match self.state {
                State::Playing => {
                    for (channel, sample) in channel_samples.iter_mut().enumerate() {
                        *sample = self.buf.read(channel);
                    }
                }
                State::Recording => {
                    for (channel, sample) in channel_samples.iter_mut().enumerate() {
                        self.buf.write(channel, *sample);
                        *sample = 0.0;
                    }
                }
                State::Idle => {
                    for (channel, sample) in channel_samples.iter_mut().enumerate() {
                        *sample = 0.0;
                    }
                }
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
