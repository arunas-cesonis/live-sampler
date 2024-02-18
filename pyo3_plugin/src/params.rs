use crate::editor_vizia;
use crate::source_path::SourcePath;
use nih_plug::params::{BoolParam, EnumParam, FloatParam, Params};
use nih_plug::prelude::{Enum, FloatRange};
use nih_plug_vizia::ViziaState;
use std::sync::Arc;

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
    #[id = "bypass"]
    pub auto_passthru: BoolParam,
    #[id = "show_debug_data"]
    pub show_debug_data: BoolParam,
    #[id = "speed"]
    pub speed: FloatParam,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    // #[id = "loop_mode"]
    // pub loop_mode: EnumParam<LoopModeParam>,
    #[id = "loop_length"]
    pub loop_length: FloatParam,
    // pub loop_length_unit: EnumParam<TimeOrRatioUnit>,
    // #[id = "loop_length_unit"]
    #[id = "start_offset"]
    pub start_offset: FloatParam,
    //    #[id = "recording_mode"]
    // pub recording_mode: EnumParam<RecordingMode>,
    #[id = "mode"]
    pub mode: EnumParam<ModeParam>,
    // The editor state, saved together with the parameter state so the custom scaling can be
    // restored.
    // #[persist = "editor-state"]
    // editor_state: Arc<ViziaState>,
    #[persist = "editor-state"]
    pub(crate) editor_state: Arc<ViziaState>,
    #[persist = "source-path"]
    pub(crate) source_path: SourcePath,
}

impl Default for PyO3PluginParams2 {
    fn default() -> Self {
        Self {
            auto_passthru: BoolParam::new("Auto passthru", true),
            show_debug_data: BoolParam::new("Show debug data", false),
            speed: FloatParam::new("Speed", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 }),
            attack: FloatParam::new("Attack", 0.0, FloatRange::Linear { min: 0.0, max: 2.0 }),
            editor_state: editor_vizia::default_state(),
            decay: FloatParam::new("Decay", 0.0, FloatRange::Linear { min: 0.0, max: 2.0 }),
            mode: EnumParam::new("Mode", ModeParam::default()),
            // loop_mode: EnumParam::new("Loop mode", LoopModeParam::default()),
            loop_length: FloatParam::new(
                "Loop length",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            ),
            start_offset: FloatParam::new(
                "Start offset",
                0.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            ),
            // recording_mode: EnumParam::new("Recording mode", RecordingMode::default()),
            // midi_channel: EnumParam::new("MIDI channel", MIDIChannelParam::default()),
            // editor_state: default_state(),
            source_path: SourcePath::default(),
        }
    }
}
