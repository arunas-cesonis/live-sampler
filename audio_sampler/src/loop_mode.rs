use nih_plug::prelude::Enum;

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopMode {
    PlayOnce,
    PingPong,
    Loop,
}
