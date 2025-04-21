use std::sync::Arc;

use nih_plug::params::{BoolParam, EnumParam, Params};
use nih_plug::prelude::{Enum, FloatParam, FloatRange};
use nih_plug_vizia::ViziaState;

use crate::editor_vizia;
use crate::source_path::PersistedSourcePath;

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
    #[id = "param1"]
    pub(crate) param1: FloatParam,
    #[id = "param2"]
    pub(crate) param2: FloatParam,
    #[id = "param3"]
    pub(crate) param3: FloatParam,
    #[id = "param4"]
    pub(crate) param4: FloatParam,
    #[id = "watch-file"]
    pub(crate) watch_source_path: BoolParam,
    #[id = "mode"]
    pub mode: EnumParam<ModeParam>,
    #[persist = "editor-state"]
    pub(crate) editor_state: Arc<ViziaState>,
    #[persist = "source-path"]
    pub(crate) source_path: PersistedSourcePath,
}

impl PyO3PluginParams2 {
    pub fn source_path(&self) -> &PersistedSourcePath {
        &self.source_path
    }
}

impl Default for PyO3PluginParams2 {
    fn default() -> Self {
        Self {
            param1: FloatParam::new("Param 1", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            param2: FloatParam::new("Param 2", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            param3: FloatParam::new("Param 3", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            param4: FloatParam::new("Param 4", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            watch_source_path: BoolParam::new("Watch file", false),
            editor_state: editor_vizia::default_state(),
            mode: EnumParam::new("Mode", ModeParam::default()),
            source_path: PersistedSourcePath::default(),
        }
    }
}
