extern crate core;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use nih_plug::prelude::*;

use crate::common_types::{EvalStatus, RuntimeStats, Status, UICommand};
use crate::event::PyO3NoteEvent;
use crate::host::Host;
use crate::params::{ModeParam, PyO3PluginParams2};
use crate::source_state::SourceState;

mod common_types;
mod editor_vizia;
pub mod event;
mod host;
mod params;
mod source_path;
mod source_state;

type SysEx = ();

pub struct PyO3Plugin {
    sample_rate: Option<f32>,
    params: Arc<PyO3PluginParams2>,
    commands: Option<crossbeam_channel::Receiver<UICommand>>,
    status_in: Arc<parking_lot::Mutex<triple_buffer::Input<Status>>>,
    status_out: Arc<parking_lot::Mutex<triple_buffer::Output<Status>>>,
    runtime_stats_in: Arc<parking_lot::Mutex<triple_buffer::Input<Option<RuntimeStats>>>>,
    runtime_stats_out: Arc<parking_lot::Mutex<triple_buffer::Output<Option<RuntimeStats>>>>,
    now: usize,
    stats_updated: usize,
    stats_update_every: Duration,
    status: Status,
    source_state: SourceState,
    host: Host,
}

// unsafe impl Send for PyO3Plugin {}

impl Default for PyO3Plugin {
    fn default() -> Self {
        let (status_in, status_out) = triple_buffer::triple_buffer(&Status::default());
        let (runtime_stats_in, runtime_stats_out) = triple_buffer::triple_buffer(&None);
        Self {
            sample_rate: None,
            params: Arc::new(PyO3PluginParams2::default()),
            commands: None,
            status_in: Arc::new(parking_lot::Mutex::new(status_in)),
            status_out: Arc::new(parking_lot::Mutex::new(status_out)),
            runtime_stats_in: Arc::new(parking_lot::Mutex::new(runtime_stats_in)),
            runtime_stats_out: Arc::new(parking_lot::Mutex::new(runtime_stats_out)),
            now: 0,
            stats_updated: 0,
            stats_update_every: Duration::from_secs(1),
            source_state: SourceState::Empty,
            status: Status::default(),
            host: Host::default(),
        }
    }
}

impl PyO3Plugin {
    fn publish_status(&mut self) {
        self.status_in.lock().write(self.status.clone());
    }
    fn publish_stats(&mut self) {
        self.runtime_stats_in
            .lock()
            .write(self.host.runtime_stats().cloned());
    }

    fn write_source_path(&self, path: String) {
        *self.params.source_path.0.lock() = path;
    }

    fn read_source_path(&self) -> String {
        self.params.source_path.0.lock().clone()
    }

    fn check_watcher(&mut self) {
        if let Some(fst) = self
            .source_state
            .reload_watched(self.params.watch_source_path.value())
        {
            self.status.eval_status = EvalStatus::NotExecuted;
            self.status.file_status = fst;
            self.publish_stats();
            self.publish_status();
        }
    }

    fn load(&mut self) {
        let fst = self.source_state.load_updated_path(
            self.read_source_path(),
            self.params.watch_source_path.value(),
        );
        if !fst.is_loaded() {
            self.host.clear();
        }
        self.status.eval_status = EvalStatus::NotExecuted;
        self.status.file_status = fst;
        self.publish_stats();
        self.publish_status();
    }

    fn do_ui_command(&mut self, cmd: UICommand) {
        match cmd {
            UICommand::Reload => {
                self.load();
            }
            UICommand::Reset => {
                self.host.clear();
                self.status.eval_status = EvalStatus::NotExecuted;
                self.publish_status();
            }
        }
    }

    fn recv_and_do_ui_commands(&mut self) {
        let cmds = if let Some(rx) = &self.commands {
            let mut cmds = vec![];
            while let Ok(cmd) = rx.try_recv() {
                cmds.push(cmd);
            }
            cmds
        } else {
            vec![]
        };
        for cmd in cmds {
            self.do_ui_command(cmd);
        }
    }
}

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
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = SysEx;

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let (tx, rx) = crossbeam_channel::unbounded();
        let tx = Arc::new(tx);
        self.commands = Some(rx);
        let data = editor_vizia::Data {
            //source_path: self.params.source_path.clone(),
            commands: tx,
            params: self.params.clone(),
            status: self.status_out.lock().read().clone(),
            status_out: self.status_out.clone(),
            runtime_stats_out: self.runtime_stats_out.clone(),
        };

        editor_vizia::create2(self.params.editor_state.clone(), data)
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = Some(buffer_config.sample_rate);
        std::env::var("PYO3_PLUGIN_SOURCE_PATH")
            .into_iter()
            .for_each(|path| {
                self.write_source_path(path);
                self.load();
            });
        true
    }

    fn reset(&mut self) {
        self.host.clear();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // FIXME: oops this is not per sample! make this per sample when introducing generic parameters

        let mode = self.params.mode.value();
        self.recv_and_do_ui_commands();
        self.check_watcher();

        if let Some(source) = self.source_state.get_source() {
            let sample_rate = self.sample_rate.unwrap();
            if mode == ModeParam::Run && !self.status.eval_status.is_error() {
                let mut events: Vec<PyO3NoteEvent> = vec![];
                while let Some(next_event) = context.next_event() {
                    events.push(next_event.into());
                }
                let result = self.host.run(self.now, sample_rate, buffer, events, source);
                match result {
                    Ok(processed_events) => {
                        processed_events
                            .into_iter()
                            .for_each(|e| context.send_event(e.into()));
                        if self.status.eval_status != EvalStatus::Ok {
                            self.status.eval_status = EvalStatus::Ok;
                            self.publish_status();
                        }
                    }
                    Err(e) => {
                        self.status.eval_status = EvalStatus::Error(e);
                        self.publish_status();
                    }
                };
                if self.params.editor_state.is_open()
                    && self.now - self.stats_updated
                    >= (self.stats_update_every.as_secs_f64() * sample_rate as f64) as usize
                {
                    self.stats_updated = self.now;
                    self.publish_stats();
                    self.publish_status();
                }
            } else if mode == ModeParam::Bypass {
                let mut next_event = context.next_event();
                for (sample_id, _channel_samples) in buffer.iter_samples().enumerate() {
                    let mut events = vec![];
                    while let Some(event) = next_event {
                        if event.timing() != sample_id as u32 {
                            break;
                        }
                        events.push(event);
                        next_event = context.next_event();
                    }
                    for e in events {
                        context.send_event(e);
                    }
                }
            } // mode == ModeParam::Pause
        }

        self.now += buffer.samples();

        ProcessStatus::Normal
    }
}

impl ClapPlugin for PyO3Plugin {
    const CLAP_ID: &'static str = "com.pyo3plugin";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("PyO3Plugin");
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
    const VST3_CLASS_ID: [u8; 16] = *b"PyO3Pluginnnnnnn";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(PyO3Plugin);
nih_export_vst3!(PyO3Plugin);
