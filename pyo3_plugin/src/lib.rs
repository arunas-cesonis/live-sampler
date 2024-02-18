#![feature(atomic_bool_fetch_not)]
#![feature(associated_type_bounds)]
extern crate core;

use std::collections::VecDeque;
use std::io::Write;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use pyo3::ffi::{PyObject_Repr, Py_None};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict, PyList, PyTuple};
use pyo3::{PyErr, Python};

use crate::common_types::{EvalError, EvalStatus, FileStatus, RuntimeStats, Status};
use crate::event::{NoteOn, PyO3NoteEvent};
use crate::host::{Host, Stats};
use crate::params::{ModeParam, PyO3PluginParams2};
use crate::source_path::SourcePath;

mod common_types;
mod editor_vizia;
pub mod event;
mod host;
mod params;
mod source_path;

type SysEx = ();

pub struct PyO3Plugin {
    params: Arc<PyO3PluginParams2>,
    sample_rate: f32,
    data_version: Arc<AtomicUsize>,
    status_in: Arc<parking_lot::Mutex<triple_buffer::Input<Status>>>,
    status_out: Arc<parking_lot::Mutex<triple_buffer::Output<Status>>>,
    runtime_stats_in: Arc<parking_lot::Mutex<triple_buffer::Input<Option<RuntimeStats>>>>,
    runtime_stats_out: Arc<parking_lot::Mutex<triple_buffer::Output<Option<RuntimeStats>>>>,
    seen_data_version: usize,
    python_source: Option<String>,
    python_state: Option<Py<PyAny>>,
    now: usize,
    last_sec: VecDeque<(usize, Duration)>,
    last_sec_sum: Duration,
    stats_updated: usize,
    stats_update_every: Duration,
    paused_on_error: bool,
    host: Host,
    stats: Stats,
}

// unsafe impl Send for PyO3Plugin {}

impl Default for PyO3Plugin {
    fn default() -> Self {
        let (status_in, status_out) = triple_buffer::triple_buffer(&Status::default());
        let (runtime_stats_in, runtime_stats_out) = triple_buffer::triple_buffer(&None);
        Self {
            params: Arc::new(PyO3PluginParams2::default()),
            sample_rate: 0.0,
            status_in: Arc::new(parking_lot::Mutex::new(status_in)),
            status_out: Arc::new(parking_lot::Mutex::new(status_out)),
            runtime_stats_in: Arc::new(parking_lot::Mutex::new(runtime_stats_in)),
            runtime_stats_out: Arc::new(parking_lot::Mutex::new(runtime_stats_out)),
            data_version: Arc::new(AtomicUsize::new(1)),
            seen_data_version: 0,
            python_source: None,
            python_state: None,
            last_sec: VecDeque::new(),
            last_sec_sum: Duration::from_secs(0),
            now: 0,
            stats_updated: 0,
            stats_update_every: Duration::from_secs(1),
            paused_on_error: false,
            host: Host::default(),
            stats: Stats::default(),
        }
    }
}

impl PyO3Plugin {
    fn publish_status(&mut self) {
        self.status_in.lock().write(self.host.status().clone());
    }

    fn update_python_source(&mut self) {
        let data_version = self.data_version.load(Ordering::Relaxed);
        if data_version != self.seen_data_version {
            self.seen_data_version = data_version;
            let path = { self.params.source_path.0.lock().clone() };
            self.host.load_source(&path);
        }
    }

    fn copyback_buffer(&self, buf: &mut Buffer, result: &[Vec<f32>]) -> Result<(), EvalError> {
        let nc = buf.channels();
        let ns = buf.samples();
        if nc != result.len() {
            return Err(EvalError::OtherError(format!(
                "Number of channels returned from python ({}) does not match the buffer ({}):",
                result.len(),
                nc
            )));
        }
        if let Some((i, xlen)) = result.iter().enumerate().find_map(|(i, x)| {
            if x.len() != ns {
                Some((i, x.len()))
            } else {
                None
            }
        }) {
            return Err(EvalError::OtherError(format!(
                "Number of samples returned from python ({}) does not match the number of samples in the buffer ({}) at channel {}",
                xlen, ns, i
            )));
        }
        let sl = buf.as_slice();
        for i in 0..ns {
            for j in 0..nc {
                sl[j][i] = result[j][i];
            }
        }
        Ok(())
    }

    fn run_python(
        &mut self,
        buffer: &mut Buffer,
        events: Vec<PyO3NoteEvent>,
    ) -> Result<Vec<PyO3NoteEvent>, EvalError> {
        #[derive(FromPyObject)]
        struct PythonProcessResult(Py<PyAny>, Vec<Vec<f32>>, Vec<PyO3NoteEvent>);

        if let Some(python_source) = &self.python_source {
            let buf = buffer.as_slice();
            let result = Python::with_gil(|py| -> Result<PythonProcessResult, PyErr> {
                //let host_module = PyModule::new(py, "host")?;
                //host_module.add_function(wrap_pyfunction!(host_print, host_module)?)?;
                let tmp = buf.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
                let pybuf: Py<PyAny> = PyList::new(py, tmp).into_py(py);
                let events: Py<PyAny> = PyList::new(py, events).into_py(py);
                let state = self
                    .python_state
                    .take()
                    .unwrap_or(PyList::empty(py).to_object(py));
                //let globals = [("host", host_module)].into_py_dict(py);
                let locals =
                    [("state", state), ("buffer", pybuf), ("events", events)].into_py_dict(py);
                py.run(python_source.as_str(), None, Some(locals))?;

                let result: &PyAny =
                    py.eval("process(state, buffer, events)", None, Some(locals))?;
                let result: Result<PythonProcessResult, PyErr> = result.extract();

                Ok(result?)
            });
            match result {
                Ok(PythonProcessResult(new_state, in_buffer, events)) => {
                    self.copyback_buffer(buffer, &in_buffer)?;
                    self.python_state = Some(new_state);
                    Ok(events)
                }
                Err(e) => {
                    nih_error!("python error: {}", e);
                    Err(EvalError::PythonError(e.to_string()))
                }
            }
        } else {
            Err(EvalError::OtherError("no source loaded".to_string()))
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
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = SysEx;

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let data = editor_vizia::Data {
            version: self.data_version.clone(),
            //source_path: self.params.source_path.clone(),
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
        self.update_python_source();
        let mode = self.params.mode.value();

        if mode == ModeParam::Run && !self.host.status().paused_on_error {
            let mut events: Vec<PyO3NoteEvent> = vec![];
            while let Some(next_event) = context.next_event() {
                events.push(next_event.into());
            }
            let result = self.host.run(self.now, self.sample_rate, buffer, events);
            match result {
                Ok(processed_events) => {
                    if !processed_events.is_empty() {
                        nih_warn!("python returned {} events", processed_events.len());
                    }
                    processed_events
                        .into_iter()
                        .for_each(|e| context.send_event(e.into()));
                }
                Err(e) => {
                    self.publish_status();
                }
            };
            if self.params.editor_state.is_open() {
                if self.now - self.stats_updated
                    >= (self.stats_update_every.as_secs_f64() * self.sample_rate as f64) as usize
                {
                    self.runtime_stats_in
                        .lock()
                        .write(self.host.runtime_stats().cloned());
                    self.status_in.lock().write(self.host.status().clone());
                    self.stats_updated = self.now;
                }
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
