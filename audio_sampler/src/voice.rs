use crate::clip::Clip;
use crate::common_types::Params;
use crate::sampler::LoopMode;
use crate::utils;
use crate::utils::normalize_offset;
use crate::volume::Volume;
use smallvec::SmallVec;
use std::ops::Index;

#[derive(Clone, Debug)]
pub struct Voice {
    pub note: u8,
    pub loop_start_percent: f32,
    pub played: f32,
    pub volume: Volume,
    pub finished: bool,
    pub ping_pong_speed: f32,
    pub clip: Clip,
    // this is only used by the UI to show loop points
    // its hack/workaround for not having loop information easily available
    pub last_sample_index: usize,
}
