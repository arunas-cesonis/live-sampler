use std::time::{Duration, Instant};

#[derive(PartialEq, Clone, Debug)]
pub enum FileStatus {
    Loaded(String, usize, Instant),
    NotLoaded,
    Error(String),
}

impl FileStatus {
    pub fn is_loaded(&self) -> bool {
        match self {
            FileStatus::Loaded(p, _, _) => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum FileError {
    PythonError(String),
    OtherError(String),
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

impl EvalStatus {
    pub fn is_error(&self) -> bool {
        match self {
            EvalStatus::Error(_) => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum UICommand {
    Reload,
    Reset,
}

#[derive(Default, Clone, Debug)]
pub struct RuntimeStats {
    pub total_duration: Duration,
    pub last_duration: Duration,
    pub last_rolling_avg: Duration,
    pub iterations: usize,
    pub source_loaded: Option<Instant>,
    pub events_to_pyo3: usize,
    pub events_from_pyo3: usize,
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
            source_loaded: None,
            events_to_pyo3: 0,
            events_from_pyo3: 0,
            window_size: 0,
            sample_rate: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub file_status: FileStatus,
    pub eval_status: EvalStatus,
}

impl Default for FileStatus {
    fn default() -> Self {
        Self::NotLoaded
    }
}

impl Default for EvalStatus {
    fn default() -> Self {
        Self::NotExecuted
    }
}
