use nih_plug::prelude::Enum;

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopMode {
    PlayOnce,
    PingPong,
    Loop,
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopModeParam {
    PlayOnce,
    Loop,
}

impl LoopMode {
    pub fn from_param(param: LoopModeParam) -> Self {
        match param {
            LoopModeParam::PlayOnce => LoopMode::PlayOnce,
            LoopModeParam::Loop => LoopMode::Loop,
        }
    }
}
