#![feature(atomic_bool_fetch_not)]
#![feature(associated_type_bounds)]
extern crate core;

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
use crate::event::PyO3NoteEvent;
use crate::params::{ModeParam, PyO3PluginParams2};
use crate::source_path::SourcePath;

mod common_types;
mod editor_vizia;
pub mod event;
mod params;
mod source_path;

type SysEx = ();

pub struct PyO3Plugin {
    params: Arc<PyO3PluginParams2>,
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
    paused_on_error: bool,
}

impl Default for PyO3Plugin {
    fn default() -> Self {
        let (status_in, status_out) = triple_buffer::triple_buffer(&Status::default());
        Self {
            params: Arc::new(PyO3PluginParams2::default()),
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
            paused_on_error: false,
        }
    }
}

impl PyO3Plugin {
    fn publish_status(&mut self) {
        self.status_in.lock().write(self.status.clone());
    }

    fn update_python_source(&mut self) {
        let data_version = self.data_version.load(Ordering::Relaxed);
        if data_version != self.seen_data_version {
            self.seen_data_version = data_version;
            let path = { self.params.source_path.0.lock().clone() };
            nih_warn!("data_version={:?} path={:?}", data_version, path);
            if path == "" {
                self.python_source = None;
                self.status.file_status = FileStatus::Unloaded;
                self.status.runtime_stats = None;
                self.publish_status();
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
                    self.status.file_status =
                        FileStatus::Loaded(path.to_string(), size.try_into().unwrap());
                    self.status.paused_on_error = false;
                    self.publish_status();
                }
                Err(e) => {
                    nih_log!("python not loaded: {}", e);
                    self.status.file_status = FileStatus::Error(e.to_string());
                }
            }
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
        struct PythonProcessResult(Vec<Vec<f32>>, Vec<PyO3NoteEvent>);

        if let Some(python_source) = &self.python_source {
            let buf = buffer.as_slice();
            let result = Python::with_gil(|py| -> Result<PythonProcessResult, PyErr> {
                py.run(python_source.as_str(), None, None)?;
                let tmp = buf.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
                let pybuf = PyList::new(py, tmp);
                let had_events = !events.is_empty();
                let events = PyList::new(py, events);
                let locals = [("buffer", pybuf), ("events", events)].into_py_dict(py);

                let result: &PyAny = py.eval("process(buffer, events)", None, Some(locals))?;
                let result: Result<PythonProcessResult, PyErr> = result.extract();

                Ok(result?)
            });
            match result {
                Ok(PythonProcessResult(in_buffer, events)) => {
                    self.copyback_buffer(buffer, &in_buffer)?;
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
        struct FrameStats {
            d: Duration,
            events_to_pyo3: usize,
            events_from_pyo3: usize,
        }
        let ret = if mode == ModeParam::Run && !self.status.paused_on_error {
            let mut frame_stats = FrameStats {
                d: Duration::from_secs(0),
                events_to_pyo3: 0,
                events_from_pyo3: 0,
            };
            let mut events: Vec<PyO3NoteEvent> = vec![];
            while let Some(next_event) = context.next_event() {
                frame_stats.events_to_pyo3 += 1;
                events.push(next_event.into());
            }
            let elapsed = Instant::now();
            let result = self.run_python(buffer, events);
            let d = elapsed.elapsed();
            match result {
                Ok(processed_events) => {
                    self.status.eval_status = EvalStatus::Ok;
                    if !processed_events.is_empty() {
                        nih_warn!("python returned {} events", processed_events.len());
                    }
                    processed_events.into_iter().for_each(|e| {
                        frame_stats.events_from_pyo3 += 1;
                        context.send_event(e.into())
                    });
                }
                Err(e) => {
                    self.status.eval_status = (EvalStatus::Error(e));
                    self.status.paused_on_error = true;
                    self.publish_status();
                }
            };
            frame_stats.d = d;
            Some(frame_stats)
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
            None
        } else {
            None
        };

        if let (Some(frame_stats), Some(rt)) = ((ret, self.status.runtime_stats.as_mut())) {
            rt.iterations += 1;
            rt.total_duration += frame_stats.d;
            rt.last_duration = frame_stats.d;
            rt.events_to_pyo3 += frame_stats.events_to_pyo3;
            rt.events_from_pyo3 += frame_stats.events_from_pyo3;
            // this gets up to 938 at 48.0kHz - must be more efficient way.
            // e.g. could sum to more coarse elements as only stats_update_every second's precision is needed
            self.last_sec.push_back((self.now, frame_stats.d));
            self.last_sec_sum += frame_stats.d;
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
