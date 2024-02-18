use std::sync::Arc;

use nih_plug::params::{BoolParam, EnumParam, FloatParam, Params};
use nih_plug::prelude::{Enum, FloatRange};
use nih_plug_vizia::ViziaState;

use crate::editor_vizia;
use crate::source_path::SourcePath;

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum ModeParam {
    Run,
    Pause,
    Bypass,
}
impl Default for ModeParam {
    fn default() -> Self {
        ModeParam::Run
    }
}

#[derive(Params)]
pub struct PyO3PluginParams2 {
    #[id = "watch-file"]
    pub(crate) watch_source_path: BoolParam,
    #[id = "mode"]
    pub mode: EnumParam<ModeParam>,
    #[persist = "editor-state"]
    pub(crate) editor_state: Arc<ViziaState>,
    #[persist = "source-path"]
    pub(crate) source_path: SourcePath,
}

impl PyO3PluginParams2 {
    pub fn source_path(&self) -> &SourcePath {
        &self.source_path
    }
}

impl Default for PyO3PluginParams2 {
    fn default() -> Self {
        Self {
            watch_source_path: BoolParam::new("Watch file", false),
            editor_state: editor_vizia::default_state(),
            mode: EnumParam::new("Mode", ModeParam::default()),
            source_path: SourcePath::default(),
        }
    }
}
