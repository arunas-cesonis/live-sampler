use crate::clip::Clip;
use crate::volume::Volume;

#[repr(transparent)]
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceId(pub usize);

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
