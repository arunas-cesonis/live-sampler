use crate::event::PyO3NoteEvent;
use crate::source_state::Source;
use crate::utils::EvalError;

pub struct RunParams {
    pub now: usize,
    pub buffer: Vec<Vec<f32>>,
    pub events: Vec<PyO3NoteEvent>,
    pub transport: crate::transport::Transport,
    pub source: Source,
}

pub type RunResult = Result<(Vec<Vec<f32>>, Vec<PyO3NoteEvent>), EvalError>;
