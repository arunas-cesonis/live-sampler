#![feature(atomic_bool_fetch_not)]
extern crate core;

use nih_plug::params::persist::PersistentField;
use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nih_plug::prelude::*;

use nih_plug_vizia::ViziaState;
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyList};
use pyo3::{PyErr, Python};

use crate::common_types::{EvalError, EvalStatus, FileStatus, RuntimeStats, Status};
use crate::source_path::SourcePath;

mod common_types;
mod editor_vizia;
mod source_path;

type SysEx = ();

pub struct PyO3Plugin {
    params: Arc<PyO3PluginParams>,
    sample_rate: f32,
    data_version: Arc<AtomicUsize>,
    status_in: Arc<parking_lot::Mutex<triple_buffer::Input<Status>>>,
    status_out: Arc<parking_lot::Mutex<triple_buffer::Output<Status>>>,
    seen_data_version: usize,
    python_source: Option<String>,
    status: Status,
    now: usize,
    last_sec: VecDeque<(usize, Duration)>,
    last_sec_sum: Duration,
    stats_updated: usize,
    stats_update_every: Duration,
}

#[derive(Params)]
pub struct PyO3PluginParams {
    #[persist = "source-path"]
    source_path: SourcePath,
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
    #[persist = "bypass-param"]
    bypass: MyBoolParam,
}

pub struct MyBoolParam {
    value: AtomicBool,
}

impl<'a> PersistentField<'a, bool> for MyBoolParam {
    fn set(&self, new_value: bool) {
        self.value.store(new_value, Ordering::Relaxed);
    }
    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&bool) -> R,
    {
        f(&self.value.load(Ordering::Relaxed))
    }
}

impl Default for PyO3PluginParams {
    fn default() -> Self {
        Self {
            editor_state: editor_vizia::default_state(),
            #[cfg(debug_assertions)]
            source_path: SourcePath(Arc::new(parking_lot::Mutex::new("./a.py".to_string()))),
            #[cfg(not(debug_assertions))]
            source_path: SourcePath::default(),
            bypass: MyBoolParam {
                value: AtomicBool::new(false),
            },
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
            data_version: Arc::new(AtomicUsize::new(1)),
            seen_data_version: 0,
            python_source: None,
            last_sec: VecDeque::new(),
            last_sec_sum: Duration::from_secs(0),
            status: Status::default(),
            now: 0,
            stats_updated: 0,
            stats_update_every: Duration::from_secs(1),
        }
    }
}

// #[pyfunction]
// #[pyo3(name = "print")]
// fn python_print(a: &PyAny) {
//     nih_warn!("python: {:?}", a);
// }
//
// #[pymodule]
// fn module_with_functions(_py: Python, m: &PyModule) -> PyResult<()> {
//     m.add_function(wrap_pyfunction!(python_print, m)?)?;
//     Ok(())
// }

impl PyO3Plugin {
    fn publish_status(&mut self) {
        self.status_in.lock().write(self.status.clone());
    }
    fn update_file_status(&mut self, file_status: FileStatus, always_publish: bool) {
        if self.status.file_status != file_status {
            self.status.file_status = file_status;
            if !always_publish {
                self.publish_status();
            }
        }
        if always_publish {
            self.publish_status();
        }
    }

    fn update_eval_status(&mut self, eval_status: EvalStatus) {
        if self.status.eval_status != eval_status {
            self.status.eval_status = eval_status;
            self.publish_status();
        }
    }

    fn update_python_source(&mut self) {
        let data_version = self.data_version.load(Ordering::Relaxed);
        if data_version != self.seen_data_version {
            self.seen_data_version = data_version;
            let path = { self.params.source_path.0.lock().clone() };
            nih_warn!("data_version={:?} path={:?}", data_version, path);
            if path == "" {
                self.update_file_status(FileStatus::Unloaded, false);
                self.python_source = None;
                self.status.runtime_stats = None;
                return;
            }
            let ret = std::fs::read_to_string(&*path);
            match ret {
                Ok(source) => {
                    let size = source.len();
                    self.python_source = Some(source);
                    self.status.runtime_stats = Some(RuntimeStats::new());
                    self.last_sec = Default::default();
                    self.last_sec_sum = Default::default();
                    nih_log!("python source: {}", self.python_source.as_ref().unwrap());
                    self.update_file_status(
                        FileStatus::Loaded(path.to_string(), size.try_into().unwrap()),
                        true,
                    );
                }
                Err(e) => {
                    nih_log!("python not laoded: {}", e);
                    self.update_file_status(FileStatus::Error(e.to_string()), false);
                }
            }
        }
    }

    fn copyback_buffer(&self, buf: &mut Buffer, result: Vec<Vec<f32>>) -> Result<(), EvalError> {
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

    fn run_python(&mut self, buffer: &mut Buffer) -> Result<(), EvalError> {
        if let Some(python_source) = &self.python_source {
            let buf = buffer.as_slice();
            let result = Python::with_gil(|py| -> Result<Vec<Vec<f32>>, PyErr> {
                //let m = py.import("module_with_functions")?;
                //let globals = [("module_with_functions", m)].into_py_dict(py);
                py.run(python_source.as_str(), None, None)?;
                let tmp = buf.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
                let pybuf = PyList::new(py, tmp);
                let locals = [("buffer", pybuf)].into_py_dict(py);

                let result: Vec<Vec<f32>> =
                    py.eval("process(buffer)", None, Some(locals))?.extract()?;
                Ok(result)
            });
            self.copyback_buffer(
                buffer,
                result.map_err(|e| EvalError::PythonError(e.to_string()))?,
            )?;
        } else {
            return Err(EvalError::OtherError("no source loaded".to_string()));
        }
        Ok(())
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
            params: self.params.clone(),
            status: self.status_out.lock().read().clone(),
            status_out: self.status_out.clone(),
        };

        editor_vizia::create(self.params.editor_state.clone(), data)
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

    fn reset(&mut self) {
        self.python_source = None;
        self.status = Default::default();
        self.last_sec = Default::default();
        self.last_sec_sum = Default::default();
        self.seen_data_version = 0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.update_python_source();
        let bypass = self.params.bypass.value.load(Ordering::Relaxed);
        let d = if !bypass {
            let elapsed = Instant::now();
            let result = self.run_python(buffer);
            let d = elapsed.elapsed();
            match result {
                Ok(()) => {
                    self.update_eval_status(EvalStatus::Ok);
                }
                Err(e) => {
                    self.update_eval_status(EvalStatus::Error(e));
                }
            };
            Some(d)
        } else {
            None
        };

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

        if let (Some(d), Some(rt)) = (d, self.status.runtime_stats.as_mut()) {
            rt.iterations += 1;
            rt.total_duration += d;
            rt.last_duration = d;
            // this gets up to 938 at 48.0kHz - must be more efficient way.
            // e.g. could sum to more coarse elements as only stats_update_every second's precision is needed
            self.last_sec.push_back((self.now, d));
            self.last_sec_sum += d;
            while let Some((t, d)) = self.last_sec.front().clone() {
                if self.now - t >= (10.0 * self.sample_rate) as usize {
                    self.last_sec_sum -= *d;
                    self.last_sec.pop_front();
                } else {
                    break;
                }
            }
            rt.last_rolling_avg = self.last_sec_sum / self.last_sec.len() as u32;
            rt.window_size = self.last_sec.len();
            rt.sample_rate = self.sample_rate;

            if self.params.editor_state.is_open() {
                if self.now - self.stats_updated
                    >= (self.stats_update_every.as_secs_f64() * self.sample_rate as f64) as usize
                {
                    self.status_in.lock().write(self.status.clone());
                    self.stats_updated = self.now;
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
