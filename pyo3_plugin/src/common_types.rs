use nih_plug::prelude::Params;
use pyo3::ffi::PyWideStringList;
use std::time::Duration;

#[derive(PartialEq, Clone, Debug)]
pub enum FileStatus {
    Loaded(String, usize),
    Unloaded,
    Error(String),
}

#[derive(PartialEq, Clone, Debug)]
pub enum EvalError {
    PythonError(String),
    OtherError(String),
}

#[derive(PartialEq, Clone, Debug)]
pub enum EvalStatus {
    Ok,
    NotExecuted,
    Error(EvalError),
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct Stats {
    pub duration: Duration,
    pub last_duration: Duration,
    pub last_rolling_avg: Duration,
    pub iterations: usize,
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct Status {
    pub file_status: FileStatus,
    pub eval_status: EvalStatus,
    pub stats: Stats,
}

impl Default for FileStatus {
    fn default() -> Self {
        Self::Unloaded
    }
}

impl Default for EvalStatus {
    fn default() -> Self {
        Self::NotExecuted
    }
}
