#![allow(unused)]
use nih_plug::prelude::*;
use std::sync::Arc;

#[derive(Clone, Default)]
struct Buf {
    read: usize,
    write: usize,
    data: Vec<f32>,
}

impl Buf {
    pub fn new() -> Self {
        Self {
            read: 0,
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
    pub fn read(&mut self, reverse: bool) -> f32 {
        if self.data.is_empty() {
            0.0
        } else {
            let i = self.read % self.data.len();
            self.read = if reverse {
                if self.read == 0 {
                    self.data.len()
                } else {
                    self.read - 1
                }
            } else {
                self.read + 1
            };
            self.data[i]
        }
    }
    pub fn rewind_write(&mut self) {
        self.write = 0;
    }
    pub fn clear(&mut self) {
        self.data.clear();
        self.read = 0;
        self.write = 0;
    }
    pub fn rewind_read(&mut self) {
        self.read = 0;
    }
    pub fn seek(&mut self, pos: f32) {
        assert!(
            pos >= 0.0 && pos <= 1.0,
            "pos is not in range 0.0 1.0: {}",
            pos
        );
        self.read = ((self.data.len() as f32) * pos) as usize;
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
    pub fn read(&mut self, channel: usize, reverse: bool) -> f32 {
        self.v[channel].read(reverse)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Playing {
    reverse: bool,
}

#[derive(Clone, Default, Debug, Eq, PartialEq)]
struct State {
    recording: bool,
    playing: Option<Playing>,
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
    fn start_playing(&mut self, pos: f32, reverse: bool) {
        if self.state.playing.is_none() {
            self.buf.seek(pos);
            self.state.playing = Some(Playing { reverse });
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
            self.state.playing = None;
            nih_warn!("stop_playing(): ok");
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
        self.state.playing = None;
        self.state.recording = false;
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
                match event {
                    NoteEvent::NoteOn { note, .. } => match note {
                        48 => self.start_recording(),
                        60..=75 => {
                            let pos = (note - 60) as f32 / 16.0;
                            self.start_playing(pos, false);
                        }
                        84..=91 => {
                            let pos = (note - 84) as f32 / 16.0;
                            self.start_playing(1.0 - pos, true);
                        }
                        _ => (),
                    },
                    NoteEvent::NoteOff { note, .. } => match note {
                        48 => {
                            self.stop_recording();
                        }
                        60..=75 => {
                            self.stop_playing();
                        }
                        84..=91 => {
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
                if self.state.recording {
                    self.buf.write(channel, *sample);
                }
                *sample = if let Some(Playing { reverse }) = self.state.playing {
                    self.buf.read(channel, reverse)
                } else {
                    0.0
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
