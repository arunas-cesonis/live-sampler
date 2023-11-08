#![allow(unused)]
mod volume_env;

use crate::volume_env::{PolyVolumeEnv, VolumeEnv};
use nih_plug::prelude::*;
use std::ops::DerefMut;
use std::sync::Arc;

#[derive(Clone, Debug)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    read: f32,
    recording: bool,
    reverse_playback: bool,
    playback_volume: f32,
    passthru_volume: f32,
}
impl Channel {
    pub fn calc_sample_pos(&self) -> usize {
        let len_f32 = (self.data.len() as f32);
        let i = self.read % len_f32;
        let i = if i < 0.0 { i + len_f32 } else { i };
        let i = i as usize;
        i
    }
}
impl Default for Channel {
    fn default() -> Self {
        Channel {
            data: Vec::new(),
            write: 0,
            read: 0.0,
            recording: false,
            reverse_playback: false,
            playback_volume: 0.0,
            passthru_volume: 1.0,
        }
    }
}

#[derive(Clone, Default, Debug)]
struct Channels {
    channels: Vec<Channel>,
}

impl Channels {
    pub fn new(count: usize) -> Self {
        Self {
            channels: vec![Channel::default(); count],
        }
    }
    pub fn each<F>(&mut self, f: F)
    where
        F: FnMut(&mut Channel),
    {
        self.channels.iter_mut().for_each(f)
    }
}
pub struct LiveSampler {
    channels: Channels,
    audio_io_layout: AudioIOLayout,
    params: Arc<LiveSamplerParams>,
    sample_rate: f32,
}

#[derive(Params)]
struct LiveSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "passthru"]
    pub passthru: BoolParam,
    #[id = "speed"]
    pub speed: FloatParam,
    //#[id = "fade time"]
    //pub fade_time: FloatParam,
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
            //fade_time: FloatParam::new(
            //    "Fade time",
            //    100.0,
            //    FloatRange::Linear {
            //        min: 0.0,
            //        max: 1000.0,
            //    },
            //)
            //.with_unit(" samples"), //with_smoother(SmoothingStyle::Logarithmic(50.0))
            //.with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            //.with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Default for LiveSampler {
    fn default() -> Self {
        Self {
            channels: Channels::new(0),
            audio_io_layout: AudioIOLayout::default(),
            params: Arc::new(LiveSamplerParams::default()),
            sample_rate: -1.0,
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
        self.channels = Channels::new(self.channel_count());
        self.sample_rate = buffer_config.sample_rate;
        let channel_count: usize = self.channel_count();
        true
    }

    fn reset(&mut self) {
        let channel_count: usize = self.channel_count();
        self.channels = Channels::new(self.channel_count());
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let channels = buffer.channels();
        let mut next_event = context.next_event();

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
                nih_warn!(
                    "stuff r={} w={} p={} rev={} pass={} play={}",
                    read = self.channels.channels[0].read,
                    write = self.channels.channels[0].write,
                    sample_pos = self.channels.channels[0].calc_sample_pos(),
                    reverse = self.channels.channels[0].reverse_playback,
                    passthru = self.channels.channels[0].passthru_volume,
                    playback = self.channels.channels[0].playback_volume
                );
                match event {
                    NoteEvent::NoteOn { note, .. } => match note {
                        47 => {
                            self.channels.each(|ch| ch.passthru_volume = 1.0);
                        }
                        48 => {
                            self.channels.each(|ch| {
                                if !ch.recording {
                                    ch.write = 0;
                                    ch.recording = true;
                                }
                            });
                        }
                        49 => {
                            self.channels.each(|ch| ch.reverse_playback = true);
                        }
                        60..=75 => {
                            let pos = (note - 60) as f32 / 16.0;
                            self.channels.each(|ch| {
                                ch.read = ((ch.data.len() as f32) * pos);
                                ch.playback_volume = 1.0;
                                ch.passthru_volume = 0.0;
                            });
                        }
                        _ => (),
                    },
                    NoteEvent::NoteOff { note, .. } => match note {
                        47 => {
                            self.channels.each(|ch| ch.passthru_volume = 0.0);
                        }
                        48 => {
                            self.channels.each(|ch| {
                                ch.recording = false;
                                ch.data.truncate(ch.write);
                                ch.write = 0;
                            });
                        }
                        49 => {
                            self.channels.each(|ch| ch.reverse_playback = false);
                        }
                        60..=75 => {
                            self.channels.each(|ch| {
                                ch.playback_volume = 0.0;
                                ch.passthru_volume = 1.0;
                            });
                        }
                        _ => (),
                    },
                    _ => (),
                }
                next_event = context.next_event();
            }

            let params_speed = self.params.speed.smoothed.next();
            let params_gain = self.params.gain.smoothed.next();
            let params_passthru = if self.params.passthru.value() {
                1.0
            } else {
                0.0
            };
            for (channel, sample) in channel_samples.iter_mut().enumerate() {
                let value = *sample;
                let mut ch = &mut self.channels.channels[channel];

                if ch.recording {
                    if ch.write >= ch.data.len() {
                        ch.data.push(value);
                    } else {
                        ch.data[ch.write] = value;
                    }
                    ch.write += 1;
                }

                let mut output = 0.0;
                output += value * ch.passthru_volume * params_passthru;
                output += if !ch.data.is_empty() {
                    ch.playback_volume * ch.data[ch.calc_sample_pos()]
                } else {
                    0.0
                };

                let direction = if ch.reverse_playback { -1.0f32 } else { 1.0f32 };
                ch.read += params_speed * direction;
                output *= params_gain;

                *sample = output;
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
