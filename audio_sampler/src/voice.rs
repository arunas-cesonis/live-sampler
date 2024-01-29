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
    pub ping_pong_speed: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct CalcSampleIndexParams {
    pub loop_mode: LoopMode,
    pub offset: f32,
    pub speed: f32,
    pub loop_start_percent: f32,
    pub loop_length_percent: f32,
    pub data_len: usize,
}

impl CalcSampleIndexParams {
    pub fn with_speed(self, speed: f32) -> Self {
        Self { speed, ..self }
    }

    pub fn with_offset(self, offset: f32) -> Self {
        Self { offset, ..self }
    }

    pub fn with_loop_start_percent(self, loop_start_percent: f32) -> Self {
        Self {
            loop_start_percent,
            ..self
        }
    }

    pub fn with_loop_length_percent(self, loop_length_percent: f32) -> Self {
        Self {
            loop_length_percent,
            ..self
        }
    }

    pub fn with_data_len(self, data_len: usize) -> Self {
        Self { data_len, ..self }
    }

    pub fn with_loop_mode(self, loop_mode: LoopMode) -> Self {
        Self { loop_mode, ..self }
    }

    pub fn to_result(&self) -> (usize, f32) {
        calc_sample_index(
            self.loop_mode,
            self.offset,
            self.speed,
            self.loop_start_percent,
            self.loop_length_percent,
            self.data_len,
        )
    }
}

pub fn calc_sample_index(
    loop_mode: LoopMode,
    offset: f32,
    speed: f32,
    loop_start_percent: f32,
    loop_length_percent: f32,
    data_len: usize,
) -> (usize, f32) {
    assert!(loop_length_percent > 0.0 && loop_length_percent <= 1.0);
    assert!(loop_start_percent >= 0.0 && loop_start_percent < 1.0);
    assert!(data_len > 0);
    let len_f32 = data_len as f32;
    let loop_length = loop_length_percent * len_f32;
    let start = loop_start_percent * len_f32;
    match loop_mode {
        LoopMode::Loop => {
            // adjust offset to face the direction of speed
            let x = offset + if speed < 0.0 { -1.0 } else { 0.0 };
            // wrap it to be a positive value within loop's length
            let x = utils::normalize_offset(x, loop_length);
            // add start to get the absolute offset
            let x = (start + x).round() % len_f32;
            (x as usize, 1.0)
        }
        LoopMode::PingPong => {
            // adjust offset to face the direction of speed
            // subtracting loop_length in addition to 1.0
            let x = offset + if speed < 0.0 { -1.0 - loop_length } else { 0.0 };
            // normalize offset to be within 0..2*loop_length
            let x = utils::normalize_offset(x, 2.0 * loop_length);
            // undo the mirroring effectc
            let (x, change) = if x < loop_length {
                (x, -1.0)
            } else {
                (2.0 * loop_length - x - 1.0, 1.0)
            };
            let x = (start + x).round() % len_f32;
            (x as usize, change)
        }
        LoopMode::PlayOnce => {
            // play once does not bound the offset by loop length
            let x = offset + if speed < 0.0 { -1.0 } else { 0.0 };
            let x = if x < 0.0 { x + loop_length } else { x };
            let x = (start + x).round() % len_f32;
            (x as usize, 1.0)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calc_sample_index_loop() {
        let params = CalcSampleIndexParams {
            loop_mode: LoopMode::Loop,
            offset: 0.0,
            speed: 1.0,
            loop_start_percent: 0.0,
            loop_length_percent: 1.0,
            data_len: 100,
        };
        assert_eq!(params.to_result().0, 0);
        assert_eq!(params.with_speed(-1.0).to_result().0, 99);
        assert_eq!(params.with_loop_length_percent(0.75).to_result().0, 0);
        assert_eq!(
            params
                .with_speed(-1.0)
                .with_loop_length_percent(0.75)
                .to_result()
                .0,
            74
        );
        assert_eq!(params.with_offset(25.0).to_result().0, 25);
        assert_eq!(params.with_offset(25.0).with_speed(-1.0).to_result().0, 24);
        assert_eq!(
            params
                .with_offset(30.0)
                .with_loop_start_percent(0.75)
                .with_loop_length_percent(0.5)
                .to_result()
                .0,
            5
        );
        assert_eq!(
            params
                .with_offset(30.0)
                .with_loop_start_percent(0.75)
                .with_loop_length_percent(0.5)
                .to_result()
                .0,
            5
        );
        assert_eq!(
            params
                .with_offset(30.0)
                .with_loop_start_percent(0.75)
                .with_loop_length_percent(0.5)
                .to_result()
                .0,
            5
        );
    }
}
