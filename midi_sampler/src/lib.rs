extern crate core;

//mod event_sampler3;
mod event_sampler6;
mod utils;

use std::sync::Arc;

use crate::event_sampler6::EventSampler;
use crate::utils::set_event_timing;
use nih_plug::prelude::*;

type SysEx = ();

pub struct MIDISampler {
    params: Arc<MIDISamplerParams>,
    sampler: EventSampler<SysEx>,
    sample_rate: f32,
}

#[derive(Params)]
struct MIDISamplerParams {
    #[id = "passthru"]
    pub passthru: BoolParam,
    //    #[id = "speed"]
    //    pub speed: FloatParam,
    //    #[id = "fade time"]
    //    pub fade_time: FloatParam,
}

impl Default for MIDISamplerParams {
    fn default() -> Self {
        Self {
            passthru: BoolParam::new("Pass through", true),
        }
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
    fn sampler_params(&self, context: &mut impl ProcessContext<Self>) -> event_sampler6::Params {
        event_sampler6::Params {
            sample_rate: self.sample_rate,
            passthru: self.params.passthru.value(),
            pos_beats: context.transport().pos_beats(),
            pos_samples: context.transport().pos_samples(),
            pos_seconds: context.transport().pos_seconds(),
            tempo: context.transport().tempo,
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
        _audioio_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.sampler = EventSampler::default();
        true
    }

    fn reset(&mut self) {
        self.sampler = EventSampler::default();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();

        for (sample_id, _channel_samples) in buffer.iter_samples().enumerate() {
            let params = self.sampler_params(context);
            let params = &params;

            let mut events = vec![];
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                events.push(event);
                next_event = context.next_event();
            }

            let events = self.sampler.process_sample(events, params);
            for e in events {
                let e = set_event_timing(e, sample_id as u32);
                //nih_warn!("OUTPUT: {:?}", e);
                context.send_event(e);
            }

            //self.sampler.process_sample(channel_samples, params);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for MIDISampler {
    const CLAP_ID: &'static str = "com.midisampler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("MIDI sampler");
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
    const VST3_CLASS_ID: [u8; 16] = *b"MidiSamplerPlugi";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(MIDISampler);
nih_export_vst3!(MIDISampler);
