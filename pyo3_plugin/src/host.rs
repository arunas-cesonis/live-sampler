use std::collections::VecDeque;
use std::time::Duration;

use nih_plug::buffer::Buffer;
use nih_plug::nih_log;
use pyo3::prelude::PyModule;
use pyo3::types::{IntoPyDict, PyList, PyNone, PyTuple};
use pyo3::{
    pyfunction, wrap_pyfunction, FromPyObject, IntoPy, Py, PyAny, PyErr, Python, ToPyObject,
};

use crate::common_types::{EvalError, RuntimeStats};
use crate::event::{add_pyo3_note_events, PyO3NoteEvent};

use crate::source_state::Source;

// FIXME: host.print() has to be called single tuple, e.g. host.print((1, 2, 3)); it should work with multiple args
#[pyfunction(name = "print")]
#[pyo3(signature = (*args))]
fn host_print(args: &PyTuple) {
    let s = args
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    nih_log!("python: {}", s);
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
        assert!(frame_stats.sample_rate > 0.0);
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

struct State {}

#[derive(Default)]
pub struct Host {
    python_state: Option<Py<PyAny>>,
    host_module: Option<Py<PyAny>>,
    stats: Option<Stats>,
}

fn create_host_module(py: Python) -> Result<&PyModule, PyErr> {
    let host_module = PyModule::new(py, "host")?;
    host_module.add_function(wrap_pyfunction!(host_print, host_module)?)?;
    add_pyo3_note_events(py, &host_module)?;
    Ok(host_module)
}

impl Host {
    pub fn runtime_stats(&self) -> Option<&RuntimeStats> {
        self.stats.as_ref().map(|x| x.runtime_stats())
    }

    pub fn clear(&mut self) {
        self.host_module = None;
        self.python_state = None;
        self.stats = None;
    }

    pub fn run(
        &mut self,
        now: usize,
        sample_rate: f32,
        buffer: &mut Buffer,
        events: Vec<PyO3NoteEvent>,
        source: &Source,
    ) -> Result<Vec<PyO3NoteEvent>, EvalError> {
        assert!(sample_rate > 0.0);
        #[derive(FromPyObject)]
        struct PythonProcessResult(Py<PyAny>, Vec<Vec<f32>>, Vec<PyO3NoteEvent>);

        let mut frame_stats = FrameStats {
            now: now,
            sample_rate: sample_rate,
            d: Duration::from_secs(0),
            events_to_pyo3: events.len(),
            events_from_pyo3: 0,
        };

        let buf = buffer.as_slice();
        let result = Python::with_gil(|py| -> Result<(PythonProcessResult, Duration), PyErr> {
            let tmp = buf.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
            let pybuf: Py<PyAny> = PyList::new(py, tmp).into_py(py);
            let events: Py<PyAny> = PyList::new(py, events).into_py(py);
            let state = self
                .python_state
                .take()
                .unwrap_or(PyNone::get(py).to_object(py));
            let hm = self
                .host_module
                .take()
                .unwrap_or(create_host_module(py)?.to_object(py));
            let host_module = hm.downcast::<PyModule>(py)?;
            let globals = [("host", host_module)].into_py_dict(py);
            let locals = [("state", state), ("buffer", pybuf), ("events", events)].into_py_dict(py);

            let time = std::time::Instant::now();
            py.run(source.text.as_str(), Some(globals), Some(locals))?;
            let result: &PyAny = py.eval(
                "process(state, buffer, events)",
                Some(globals),
                Some(locals),
            )?;
            let d = time.elapsed();
            self.host_module = Some(host_module.into());

            let result: Result<PythonProcessResult, PyErr> = result.extract();

            Ok((result?, d))
        });
        match result {
            Ok((PythonProcessResult(new_state, in_buffer, events), d)) => {
                frame_stats.d = d;
                frame_stats.events_from_pyo3 = events.len();

                if self.stats.is_none() {
                    self.stats = Some(Stats::default());
                }
                let stats = self.stats.as_mut().unwrap();
                stats.record(frame_stats);

                self.copyback_buffer(buffer, &in_buffer)?;
                self.python_state = Some(new_state);
                Ok(events)
            }
            Err(e) => Err(EvalError::PythonError(e.to_string())),
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
