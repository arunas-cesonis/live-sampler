#[derive(Clone, Debug)]
pub enum FileStatus {
    Loaded(String, usize),
    Unloaded,
    Error(String),
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub file_status: FileStatus,
}

impl Default for FileStatus {
    fn default() -> Self {
        Self::Unloaded
    }
}
