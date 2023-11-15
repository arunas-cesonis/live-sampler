#![allow(unused)]

use std::iter::Rev;
use std::num::IntErrorKind::PosOverflow;
use std::ops::{DerefMut, Range};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use dasp::Signal;
use lazy_static::lazy_static;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

use crate::sampler::Sampler;
use crate::volume::Volume;

mod sampler;
mod volume;

pub struct LiveSampler {
    audio_io_layout: AudioIOLayout,
    params: Arc<LiveSamplerParams>,
    sample_rate: f32,
    sampler: Sampler,
}

#[derive(Params)]
struct LiveSamplerParams {
    #[id = "auto_passthru"]
    pub auto_passthru: BoolParam,
    #[id = "speed"]
    pub speed: FloatParam,
    #[id = "fade time"]
    pub fade_time: FloatParam,
}

impl Default for LiveSamplerParams {
    fn default() -> Self {
        Self {
            auto_passthru: BoolParam::new("Pass through", true),
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
                    min: -2.0,
                    max: 2.0,
                },
            ),
            fade_time: FloatParam::new(
                "Fade time",
                2.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1000.0,
                },
            )
            .with_unit(" ms"),
        }
    }
}

impl Default for LiveSampler {
    fn default() -> Self {
        Self {
            audio_io_layout: AudioIOLayout::default(),
            params: Arc::new(LiveSamplerParams::default()),
            sample_rate: -1.0,
            sampler: Sampler::new(0, &sampler::Params::default()),
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
    fn sampler_params(&self) -> sampler::Params {
        let params_speed = self.params.speed.smoothed.next();
        let params_gain = self.params.gain.smoothed.next();
        let params_passthru = self.params.auto_passthru.value();
        let params_fade_time = self.params.fade_time.smoothed.next();
        let params_fade_samples = (params_fade_time * self.sample_rate / 1000.0) as usize;
        let params = sampler::Params {
            fade_samples: params_fade_samples,
            auto_passthru: params_passthru,
            speed: params_speed,
        };
        params
    }
}

impl Plugin for LiveSampler {
    const NAME: &'static str = "Live Sampler";
    const VENDOR: &'static str = "seunje";
    const URL: &'static str = "https://github.com/arunas-cesonis/live-sampler";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
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
        self.sampler = Sampler::new(self.channel_count(), &self.sampler_params());
        true
    }

    fn reset(&mut self) {
        let channel_count: usize = self.channel_count();
        self.sampler = Sampler::new(self.channel_count(), &self.sampler_params());
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
            let params = self.sampler_params();
            let params = &params;
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                //nih_warn!("event {:?}", event);
                // assert!(event.voice_id().is_none());
                match event {
                    NoteEvent::NoteOn { velocity, note, .. } => match note {
                        0 => self.sampler.start_recording(params),
                        1 => self.sampler.reverse(params),
                        12..=27 => {
                            let pos = (note - 12) as f32 / 16.0;
                            self.sampler.start_playing(pos, note, velocity, params);
                        }
                        _ => (),
                    },
                    NoteEvent::NoteOff { velocity, note, .. } => match note {
                        0 => self.sampler.stop_recording(params),
                        1 => self.sampler.unreverse(params),
                        12..=27 => self.sampler.stop_playing(note, params),
                        _ => (),
                    },
                    _ => (),
                }
                next_event = context.next_event();
            }

            self.sampler.process_sample(channel_samples, params);
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
