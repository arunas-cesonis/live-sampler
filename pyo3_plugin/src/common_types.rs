use crate::params::ModeParam;
use crate::source_path::SourcePath;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{Enum, Params};
use pyo3::ffi::PyWideStringList;
use std::sync::atomic::{AtomicU8, AtomicUsize};
use std::time::{Duration, Instant};

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

#[derive(PartialEq, Clone, Debug)]
pub struct RuntimeStats {
    pub total_duration: Duration,
    pub last_duration: Duration,
    pub last_rolling_avg: Duration,
    pub iterations: usize,
    pub source_loaded: Instant,
    pub window_size: usize,
    pub sample_rate: f32,
}

impl RuntimeStats {
    pub fn new() -> Self {
        Self {
            total_duration: Duration::from_secs(0),
            last_duration: Duration::from_secs(0),
            last_rolling_avg: Duration::from_secs(0),
            iterations: 0,
            source_loaded: Instant::now(),
            window_size: 0,
            sample_rate: 0.0,
        }
    }
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct Status {
    pub file_status: FileStatus,
    pub eval_status: EvalStatus,
    pub paused_on_error: bool,
    pub runtime_stats: Option<RuntimeStats>,
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