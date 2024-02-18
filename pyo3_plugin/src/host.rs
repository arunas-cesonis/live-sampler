use crate::common_types::{EvalError, EvalStatus, FileStatus, RuntimeStats, Status};
use crate::event::PyO3NoteEvent;
use crate::host;
use nih_plug::buffer::Buffer;
use pyo3::prelude::PyModule;
use pyo3::types::{IntoPyDict, PyList, PyTuple};
use pyo3::{
    pyfunction, wrap_pyfunction, FromPyObject, IntoPy, Py, PyAny, PyErr, Python, ToPyObject,
};
use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;

// FIXME: host.print() has to be called single tuple, e.g. host.print((1, 2, 3)); it should work with multiple args
#[pyfunction(name = "print")]
#[pyo3(text_signature = "(*args)")]
fn host_print(args: &PyTuple) {
    let mut iter = args.iter();
    if let Some(a) = iter.next() {
        print!("{}", a);
        for a in iter {
            print!(" {}", a);
        }
    }
    print!("\n");
}

struct FrameStats {
    now: usize,
    sample_rate: f32,
    d: Duration,
    events_to_pyo3: usize,
    events_from_pyo3: usize,
}

#[derive(Default)]
struct Stats {
    rt: RuntimeStats,
    last_sec: VecDeque<(usize, Duration)>,
    last_sec_sum: Duration,
}

impl Stats {
    pub fn runtime_stats(&self) -> &RuntimeStats {
        &self.rt
    }
    pub fn record(&mut self, frame_stats: FrameStats) {
        self.rt.iterations += 1;
        self.rt.total_duration += frame_stats.d;
        self.rt.last_duration = frame_stats.d;
        self.rt.events_to_pyo3 += frame_stats.events_to_pyo3;
        self.rt.events_from_pyo3 += frame_stats.events_from_pyo3;
        self.last_sec.push_back((frame_stats.now, frame_stats.d));
        self.last_sec_sum += frame_stats.d;
        while let Some((t, d)) = self.last_sec.front().clone() {
            if frame_stats.now - t >= (10.0 * frame_stats.sample_rate) as usize {
                self.last_sec_sum -= *d;
                self.last_sec.pop_front();
            } else {
                break;
            }
        }
        self.rt.last_rolling_avg = self.last_sec_sum / self.last_sec.len() as u32;
        self.rt.window_size = self.last_sec.len();
    }
}

#[derive(Default)]
pub struct Host {
    python_source: Option<String>,
    python_state: Option<Py<PyAny>>,
    status: Status,
    stats: Option<Stats>,
}

impl Host {
    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn runtime_stats(&self) -> Option<&RuntimeStats> {
        self.stats.as_ref().map(|x| x.runtime_stats())
    }

    pub fn unload_source(&mut self) {
        self.python_source = None;
        self.python_state = None;
        self.status.file_status = FileStatus::Unloaded;
        self.status.eval_status = EvalStatus::NotExecuted;
        self.status.paused_on_error = false;
        self.stats = None;
    }

    pub fn load_source_from_string(&mut self, source: String) {
        self.unload_source();
        self.python_source = Some(source);
        self.python_state = None;
        self.status.eval_status = EvalStatus::NotExecuted;
        self.status.paused_on_error = false;
        self.stats = Some(Stats::default());
    }

    pub fn load_source<A: AsRef<Path>>(&mut self, path: &A) {
        self.unload_source();
        let source = std::fs::read_to_string(&path);
        match source {
            Ok(source) => {
                self.status.file_status = FileStatus::Loaded(
                    path.as_ref().to_path_buf().display().to_string(),
                    source.len(),
                );
                self.load_source_from_string(source);
            }
            Err(e) => {
                self.status.file_status = FileStatus::Error(e.to_string());
            }
        }
    }

    pub fn run(
        &mut self,
        now: usize,
        sample_rate: f32,
        buffer: &mut Buffer,
        events: Vec<PyO3NoteEvent>,
    ) -> Result<Vec<PyO3NoteEvent>, EvalError> {
        #[derive(FromPyObject)]
        struct PythonProcessResult(Py<PyAny>, Vec<Vec<f32>>, Vec<PyO3NoteEvent>);

        if let Some(python_source) = &self.python_source {
            let mut frame_stats = host::FrameStats {
                now: now,
                sample_rate: sample_rate,
                d: Duration::from_secs(0),
                events_to_pyo3: events.len(),
                events_from_pyo3: 0,
            };

            let buf = buffer.as_slice();
            let result = Python::with_gil(|py| -> Result<(PythonProcessResult, Duration), PyErr> {
                let host_module = PyModule::new(py, "host")?;
                host_module.add_function(wrap_pyfunction!(host_print, host_module)?)?;
                let tmp = buf.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
                let pybuf: Py<PyAny> = PyList::new(py, tmp).into_py(py);
                let events: Py<PyAny> = PyList::new(py, events).into_py(py);
                let state = self
                    .python_state
                    .take()
                    .unwrap_or(PyList::empty(py).to_object(py));
                let globals = [("host", host_module)].into_py_dict(py);
                let locals =
                    [("state", state), ("buffer", pybuf), ("events", events)].into_py_dict(py);

                let time = std::time::Instant::now();
                py.run(python_source.as_str(), Some(globals), Some(locals))?;
                let result: &PyAny = py.eval(
                    "process(state, buffer, events)",
                    Some(globals),
                    Some(locals),
                )?;
                let d = time.elapsed();

                let result: Result<PythonProcessResult, PyErr> = result.extract();

                Ok((result?, d))
            });
            match result {
                Ok((PythonProcessResult(new_state, in_buffer, events), d)) => {
                    frame_stats.d = d;
                    frame_stats.events_from_pyo3 = events.len();
                    let stats = self.stats.as_mut().unwrap();
                    stats.record(frame_stats);

                    self.copyback_buffer(buffer, &in_buffer)?;
                    self.python_state = Some(new_state);
                    self.status.eval_status = EvalStatus::Ok;
                    Ok(events)
                }
                Err(e) => {
                    self.status.eval_status =
                        EvalStatus::Error(EvalError::PythonError(e.to_string()));
                    self.status.paused_on_error = true;
                    Err(EvalError::PythonError(e.to_string()))
                }
            }
        } else {
            Err(EvalError::OtherError("no source loaded".to_string()))
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
}
