use crate::clip2::Clip2;
use crate::common_types::{LoopMode, Note};
use crate::volume::Volume;
#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub mode: LoopMode,
    pub offset: usize,
    pub length: usize,
}

#[derive(Clone, Debug)]
pub struct Voice {
    pub note: Note,
    pub loop_start_percent: f32,
    pub played: f32,
    pub clip: Clip2,
    pub volume: Volume,
    pub finished: bool,
    // this is only used by the UI to show loop points
    // its hack/workaround for not having loop information easily available
    pub last_sample_index: usize,
}
