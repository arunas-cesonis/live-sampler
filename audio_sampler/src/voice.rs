use crate::sampler::LoopMode;
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

impl Voice {
    pub fn new(note: u8, loop_start_percent: f32) -> Self {
        Self {
            note,
            loop_start_percent,
            offset: 0.0,
            played: 0.0,
            volume: Volume::new(0.0),
            finished: false,
            last_sample_index: 0,
        }
    }
}

fn ensure_range(x: f32, n: f32) -> f32 {
    let x = x % n;
    if x < 0.0 {
        x + n
    } else {
        x
    }
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

    pub fn to_result(&self) -> usize {
        calc_sample_index1(&self)
    }
}

pub fn calc_sample_index1(params: &CalcSampleIndexParams) -> usize {
    calc_sample_index(
        params.loop_mode,
        params.offset,
        params.speed,
        params.loop_start_percent,
        params.loop_length_percent,
        params.data_len,
    )
}

pub fn calc_sample_index(
    loop_mode: LoopMode,
    offset: f32,
    speed: f32,
    loop_start_percent: f32,
    loop_length_percent: f32,
    data_len: usize,
) -> usize {
    assert!(loop_length_percent > 0.0 && loop_length_percent <= 1.0);
    assert!(loop_start_percent >= 0.0 && loop_start_percent < 1.0);
    assert!(data_len > 0);
    assert!(data_len > 0);
    let len_f32 = data_len as f32;
    let loop_length = loop_length_percent * len_f32;
    let start = loop_start_percent * len_f32;
    match loop_mode {
        LoopMode::Loop => {
            let x = offset + if speed < 0.0 { -1.0 } else { 0.0 };
            let x = x % loop_length;
            let x = if x < 0.0 { x + loop_length } else { x };
            let x = (start + x).round() % len_f32;
            let x = if x < 0.0 { x + loop_length } else { x };
            x as usize
        }
        LoopMode::PingPong => {
            let x = offset + if speed < 0.0 { -1.0 } else { 0.0 };
            let x = x % (2.0 * loop_length);
            let x = if x < 0.0 { x + 2.0 * loop_length } else { x };
            let x = if x < loop_length {
                x
            } else {
                2.0 * loop_length - x - 1.0
            };
            let x = (start + x).round() % len_f32;
            let x = if x < 0.0 { x + loop_length } else { x };
            x as usize
        }
        LoopMode::PlayOnce => {
            let x = offset + if speed < 0.0 { -1.0 } else { 0.0 };
            // let x = x % loop_length;
            let x = if x < 0.0 { x + loop_length } else { x };
            let x = (start + x).round() % len_f32;
            let x = if x < 0.0 { x + loop_length } else { x };
            x as usize
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calc_sample_index_loop() {
        let mut params = CalcSampleIndexParams {
            loop_mode: LoopMode::Loop,
            offset: 0.0,
            speed: 1.0,
            loop_start_percent: 0.0,
            loop_length_percent: 1.0,
            data_len: 100,
        };
        assert_eq!(params.to_result(), 0);
        assert_eq!(params.with_speed(-1.0).to_result(), 99);
        assert_eq!(params.with_loop_length_percent(0.75).to_result(), 0);
        assert_eq!(
            params
                .with_speed(-1.0)
                .with_loop_length_percent(0.75)
                .to_result(),
            74
        );
        assert_eq!(params.with_offset(25.0).to_result(), 25);
        assert_eq!(params.with_offset(25.0).with_speed(-1.0).to_result(), 24);
        assert_eq!(
            params
                .with_offset(30.0)
                .with_loop_start_percent(0.75)
                .with_loop_length_percent(0.5)
                .to_result(),
            5
        );
        assert_eq!(
            params
                .with_offset(30.0)
                .with_loop_start_percent(0.75)
                .with_loop_length_percent(0.5)
                .to_result(),
            5
        );
        assert_eq!(
            params
                .with_offset(30.0)
                .with_loop_start_percent(0.75)
                .with_loop_length_percent(0.5)
                .to_result(),
            5
        );
    }

    fn test_calc_sample_index_ping_pong() {
        let voice = Voice::new(0, 0.0);
        let mode = LoopMode::PingPong;
        let data = (0..5).collect::<Vec<_>>();
    }
}
