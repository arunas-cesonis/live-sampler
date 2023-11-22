extern crate core;

//mod event_sampler3;
mod count_map;
mod event_sampler5;
mod utils;

use std::fmt;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use crate::event_sampler5::EventSampler;
use nih_plug::prelude::*;

type SysEx = ();

pub struct MIDISampler {
    params: Arc<MIDISamplerParams>,
    sampler: EventSampler<SysEx>,
    sample_rate: f32,
}

#[derive(Params)]
struct MIDISamplerParams {
    //    #[id = "auto_passthru"]
    //    pub auto_passthru: BoolParam,
    //    #[id = "speed"]
    //    pub speed: FloatParam,
    //    #[id = "fade time"]
    //    pub fade_time: FloatParam,
}

impl Default for MIDISamplerParams {
    fn default() -> Self {
        Self {}
    }
}

impl Default for MIDISampler {
    fn default() -> Self {
        Self {
            params: Arc::new(MIDISamplerParams::default()),
            sample_rate: 0.0,
            sampler: EventSampler::default(),
        }
    }
}

impl MIDISampler {
    fn sampler_params(&self) -> event_sampler5::Params {
        event_sampler5::Params {
            sample_rate: self.sample_rate,
        }
    }
}

impl Plugin for MIDISampler {
    const NAME: &'static str = "MIDI Sampler";
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
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = SysEx;

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        eprintln!("*** INIT ***");
        self.sample_rate = buffer_config.sample_rate;
        self.sampler = EventSampler::default();
        true
    }

    fn reset(&mut self) {
        eprintln!("*** RESET ***");
        self.sampler = EventSampler::default();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            let params = self.sampler_params();
            let params = &params;

            let mut events = vec![];
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                events.push(event);
                next_event = context.next_event();
            }

            let events = self.sampler.process_sample(sample_id, events, params);
            for e in events {
                context.send_event(e);
            }

            //self.sampler.process_sample(channel_samples, params);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for MIDISampler {
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

impl Vst3Plugin for MIDISampler {
    const VST3_CLASS_ID: [u8; 16] = *b"LiveSamplerPlugi";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(MIDISampler);
nih_export_vst3!(MIDISampler);
