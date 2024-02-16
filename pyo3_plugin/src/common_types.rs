#[derive(Clone, Debug)]
pub enum FileStatus {
    Loaded(String, usize),
    Unloaded,
    Error(String),
}

#[derive(Clone, Debug)]
pub enum EvalError {
    PythonError(String),
    OtherError(String),
}

#[derive(Clone, Debug)]
pub enum EvalStatus {
    Ok,
    NotExecuted,
    Error(EvalError),
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub file_status: FileStatus,
    pub eval_status: EvalStatus,
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
