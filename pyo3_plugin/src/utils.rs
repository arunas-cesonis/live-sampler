use nih_plug::midi::NoteEvent;
use std::time::{Duration, Instant};

#[derive(PartialEq, Clone, Debug)]
pub enum FileStatus {
    Loaded(String, usize, Instant),
    NotLoaded,
    Error(String),
}

impl FileStatus {
    pub fn is_loaded(&self) -> bool {
        matches!(self, FileStatus::Loaded(_, _, _))
    }
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
        matches!(self, EvalStatus::Error(_))
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
    pub events_to_pyo3: usize,
    pub events_from_pyo3: usize,
    pub window_size: usize,
    pub sample_rate: f32,
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

pub fn note_event_timing<S>(note_event: &NoteEvent<S>) -> Option<u32> {
    Some(match note_event {
        NoteEvent::NoteOn { timing, .. } => *timing,
        NoteEvent::NoteOff { timing, .. } => *timing,
        NoteEvent::Choke { timing, .. } => *timing,
        NoteEvent::VoiceTerminated { timing, .. } => *timing,
        NoteEvent::PolyModulation { timing, .. } => *timing,
        NoteEvent::MonoAutomation { timing, .. } => *timing,
        NoteEvent::PolyPressure { timing, .. } => *timing,
        NoteEvent::PolyVolume { timing, .. } => *timing,
        NoteEvent::PolyPan { timing, .. } => *timing,
        NoteEvent::PolyTuning { timing, .. } => *timing,
        NoteEvent::PolyVibrato { timing, .. } => *timing,
        NoteEvent::PolyExpression { timing, .. } => *timing,
        NoteEvent::PolyBrightness { timing, .. } => *timing,
        NoteEvent::MidiChannelPressure { timing, .. } => *timing,
        NoteEvent::MidiPitchBend { timing, .. } => *timing,
        NoteEvent::MidiCC { timing, .. } => *timing,
        NoteEvent::MidiProgramChange { timing, .. } => *timing,
        NoteEvent::MidiSysEx { timing, .. } => *timing,
        _ => return None,
    })
}
