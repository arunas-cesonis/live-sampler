use crate::sampler::LoopMode;
use crate::utils;
use crate::utils::normalize_offset;
use crate::volume::Volume;

#[derive(Clone, Debug)]
pub struct Voice {
    pub note: u8,
    pub loop_start_percent: f32,
    pub offset: f32,
    pub played: f32,
    pub volume: Volume,
    pub finished: bool,
    // this is only used by the UI to show loop points
    // its hack/workaround for not having loop information easily available
    pub last_sample_index: usize,
}
