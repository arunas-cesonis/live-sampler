use crate::clip::Clip;
use crate::common_types::Note;
use crate::volume::Volume;

#[derive(Clone, Debug)]
pub struct Voice {
    pub note: Note,
    pub loop_start_percent: f32,
    pub played: f32,
    pub clip2: Clip,
    pub volume: Volume,
    pub finished: bool,
    pub finished_at: usize,
    pub is_at_zero_crossing: bool,
    pub last_sample_value: f32,
    pub speed: f32,

    // this is only used by the UI to show loop points
    // its hack/workaround for not having loop information easily available
    pub last_sample_index: usize,
}
