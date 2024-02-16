use std::num::NonZeroU32;
use std::sync::Arc;

use nih_plug::prelude::*;

type SysEx = ();

pub struct PyO3Plugin {
    params: Arc<PyO3PluginParams>,
    sample_rate: f32,
}

#[derive(Params)]
struct PyO3PluginParams {
    #[id = "passthru"]
    pub passthru: BoolParam,
    //    #[id = "speed"]
    //    pub speed: FloatParam,
    //    #[id = "fade time"]
    //    pub fade_time: FloatParam,
}

impl Default for PyO3PluginParams {
    fn default() -> Self {
        Self {
            passthru: BoolParam::new("Pass through", true),
        }
    }
}

impl Default for PyO3Plugin {
    fn default() -> Self {
        Self {
            params: Arc::new(PyO3PluginParams::default()),
            sample_rate: 0.0,
        }
    }
}

impl PyO3Plugin {}

impl Plugin for PyO3Plugin {
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
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();

        for (sample_id, _channel_samples) in buffer.iter_samples().enumerate() {
            //let params = self.sampler_params(context);
            //let params = &params;

            let mut events = vec![];
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                events.push(event);
                next_event = context.next_event();
            }

            // let events = self.sampler.process_sample(events, params);
            for e in events {
                //let e = set_event_timing(e, sample_id as u32);
                //nih_warn!("OUTPUT: {:?}", e);
                context.send_event(e);
            }

            //self.sampler.process_sample(channel_samples, params);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for PyO3Plugin {
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

impl Vst3Plugin for PyO3Plugin {
    const VST3_CLASS_ID: [u8; 16] = *b"MidiSamplerPlugi";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(PyO3Plugin);
nih_export_vst3!(PyO3Plugin);
