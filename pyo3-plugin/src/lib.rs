mod common_types;
mod editor_vizia;

use nih_plug::params::persist::PersistentField;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::common_types::{FileStatus, Status};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

type SysEx = ();

pub struct PyO3Plugin {
    params: Arc<PyO3PluginParams>,
    sample_rate: f32,
    data_version: Arc<AtomicUsize>,
    status_in: Arc<parking_lot::Mutex<triple_buffer::Input<Status>>>,
    status_out: Arc<parking_lot::Mutex<triple_buffer::Output<Status>>>,
    seen_data_version: usize,
}

#[derive(Default)]
pub struct SourcePath(Arc<parking_lot::Mutex<String>>);

impl<'a> PersistentField<'a, String> for SourcePath {
    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&String) -> R,
    {
        f(&self.0.lock())
    }
    fn set(&self, new_value: String) {
        *self.0.lock() = new_value;
    }
}

#[derive(Params)]
pub struct PyO3PluginParams {
    //    #[id = "speed"]
    //    pub speed: FloatParam,
    //    #[id = "fade time"]
    //    pub fade_time: FloatParam,
    #[persist = "source-path"]
    source_path: SourcePath,
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for PyO3PluginParams {
    fn default() -> Self {
        Self {
            editor_state: editor_vizia::default_state(),
            source_path: SourcePath::default(),
        }
    }
}

impl Default for PyO3Plugin {
    fn default() -> Self {
        let (status_in, status_out) = triple_buffer::triple_buffer(&Status::default());
        Self {
            params: Arc::new(PyO3PluginParams::default()),
            sample_rate: 0.0,
            status_in: Arc::new(parking_lot::Mutex::new(status_in)),
            status_out: Arc::new(parking_lot::Mutex::new(status_out)),

            data_version: Arc::new(AtomicUsize::new(0)),
            seen_data_version: 0,
        }
    }
}

impl PyO3Plugin {}

impl Plugin for PyO3Plugin {
    const NAME: &'static str = "PyO3Plugin";
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

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Using vizia as Iced doesn't support drawing bitmap images under OpenGL

        let data = editor_vizia::Data {
            version: self.data_version.clone(),
            params: self.params.clone(),
            status_out: self.status_out.clone(),
        };

        editor_vizia::create(self.params.editor_state.clone(), data)
    }

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

        if let data_version = self.data_version.load(Ordering::Relaxed) {
            if data_version != self.seen_data_version {
                self.seen_data_version = data_version;
                let path = self.params.source_path.0.lock();
                let ret = std::fs::metadata(&*path);
                match ret {
                    Ok(metadata) => {
                        let size = metadata.len();
                        let status = Status {
                            file_status: FileStatus::Loaded(
                                path.to_string(),
                                size.try_into().unwrap(),
                            ),
                        };
                        self.status_in.lock().write(status);
                    }
                    Err(e) => {
                        let status = Status {
                            file_status: FileStatus::Error(e.to_string()),
                        };
                        self.status_in.lock().write(status);
                    }
                }
                //self.sampler = self.sampler_params(context);
            }
        }

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
