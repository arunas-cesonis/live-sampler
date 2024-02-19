use crate::common_types::{LoopMode, Note};
use crate::volume::Volume;

#[repr(transparent)]
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceId(pub usize);

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub offset: f32,
    pub length: f32,
}

#[derive(Clone, Debug)]
pub struct Voice {
    pub note: Note,
    pub loop_start_percent: f32,
    pub played: f32,
    pub player: Player,
    pub player_updated: usize,
    pub volume: Volume,
    pub finished: bool,
    // this is only used by the UI to show loop points
    // its hack/workaround for not having loop information easily available
    pub last_sample_index: usize,
}
