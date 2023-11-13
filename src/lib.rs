#![allow(unused)]

mod editor;
mod sampler;
mod volume;

use crate::sampler::Sampler;
use crate::volume::Volume;
use dasp::Signal;
use lazy_static::lazy_static;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::iter::Rev;
use std::num::IntErrorKind::PosOverflow;
use std::ops::{DerefMut, Range};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Copy, Clone)]
enum Action {
    StartRecording,
    StopRecording,
    StartPlaying { position: f32 },
    StopPlaying,
    ReversePlayback,
    UnreversePlayback,
}
pub struct LiveSampler {
    audio_io_layout: AudioIOLayout,
    params: Arc<LiveSamplerParams>,
    sample_rate: f32,
    sampler: sampler::Sampler,
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
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
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
                2.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1000.0,
                },
            )
            .with_unit(" ms"), //with_smoother(SmoothingStyle::Logarithmic(50.0))
            // .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            //.with_string_to_value(formatters::s2v_f32_gain_to_db()),
            editor_state: editor::default_state(),
        }
    }
}
struct NoteMappings {
    on: [Option<Action>; 127],
    off: [Option<Action>; 127],
}

fn note_on_mapping() -> NoteMappings {
    let mut on = [None; 127];
    let mut off = [None; 127];
    on[0] = Some(Action::StartRecording);
    off[0] = Some(Action::StopRecording);
    on[1] = Some(Action::ReversePlayback);
    off[1] = Some(Action::UnreversePlayback);
    for i in 0..16 {
        let t = (i as f32) * (1.0 / 16.0);
        on[i] = Some((Action::StartPlaying { position: t }));
        off[i] = Some((Action::StopPlaying));
    }
    NoteMappings { on, off }
}

lazy_static! {
    static ref NOTE_MAPPINGS: NoteMappings = note_on_mapping();
}

impl Default for LiveSampler {
    fn default() -> Self {
        Self {
            audio_io_layout: AudioIOLayout::default(),
            params: Arc::new(LiveSamplerParams::default()),
            sample_rate: -1.0,
            sampler: Sampler::new(0),
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
    const VENDOR: &'static str = "seunje";
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
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    type SysExMessage = ();

    type BackgroundTask = ();

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            //self.peak_meter.clone(),
            self.params.editor_state.clone(),
        )
    }

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
        self.sampler = Sampler::new(self.channel_count());
        true
    }

    fn reset(&mut self) {
        let channel_count: usize = self.channel_count();
        self.sampler = Sampler::new(self.channel_count());
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let channels = buffer.channels();
        let mut next_event = context.next_event();
        let m = &NOTE_MAPPINGS;

        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // Smoothing is optionally built into the parameters themselves
            let gain = self.params.gain.smoothed.next();
            let params_speed = self.params.speed.smoothed.next();
            let params_gain = self.params.gain.smoothed.next();
            let params_passthru = self.params.passthru.value();
            let params_fade_time = self.params.fade_time.smoothed.next();
            let params_fade_samples = (params_fade_time * self.sample_rate / 1000.0) as usize;
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    //nih_warn!("discard sample_id={} event={:?}", sample_id, event);
                    break;
                }
                nih_warn!("event {:?}", event);
                match event {
                    NoteEvent::NoteOn { velocity, note, .. } => match note {
                        0 => self.sampler.start_recording(),
                        1 => nih_error!("reverse not implemented"),
                        7..=23 => {
                            let pos = (note - 7) as f32 / 16.0;
                            self.sampler.start_playing(pos, note, velocity);
                        }
                        _ => (),
                    },
                    NoteEvent::NoteOff { velocity, note, .. } => match note {
                        0 => self.sampler.stop_recording(),
                        1 => nih_error!("un-reverse not implemented"),
                        7..=23 => self.sampler.stop_playing(note),
                        _ => (),
                    },
                    _ => (),
                }
                next_event = context.next_event();
            }

            self.sampler.process_sample(
                channel_samples,
                &sampler::Params {
                    auto_passthru: params_passthru,
                },
            );
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
