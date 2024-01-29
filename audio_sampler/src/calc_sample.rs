use crate::loop_mode::LoopMode;
use crate::utils;
use crate::utils::{normalize_offset, ping_pong, Bound, Offset};

pub struct CalcSampleResult {
    index: usize,
    new_offset: f32,
    speed_change: f32,
}

pub struct LoopConfig {
    pub loop_mode: LoopMode,
    pub loop_start_percent: f32,
    pub loop_length_percent: f32,
    pub offset: f32,
    pub speed: f32,
    pub data_len: usize,
}

impl LoopConfig {
    pub fn to_result(&self) -> CalcSampleResult {
        calc_sample(
            self.loop_mode,
            self.loop_start_percent,
            self.loop_length_percent,
            self.offset,
            self.speed,
            self.data_len,
        )
    }

    pub fn data_len_f32(&self) -> f32 {
        self.data_len as f32
    }

    pub fn start(&self) -> f32 {
        self.loop_start_percent * self.data_len_f32()
    }

    pub fn end(&self) -> f32 {
        (self.loop_start_percent + self.loop_length_percent) * self.data_len_f32()
            % self.data_len_f32()
    }

    pub fn length(&self) -> f32 {
        self.loop_length_percent * self.data_len_f32()
    }

    pub fn buffer_to_loop(&self, x: f32) -> f32 {
        normalize_offset(x - self.start(), self.data_len_f32())
    }

    pub fn loop_to_buffer(&self, x: f32) -> f32 {
        normalize_offset(x + self.start(), self.data_len_f32())
    }

    pub fn loop_contains(&self, x: f32) -> bool {
        let s = self.start();
        let e = self.end();
        if s < e {
            s <= x && x < e
        } else {
            (s <= x && x < self.data_len_f32()) || (0.0 <= x && x < e)
        }
    }

    pub fn iter_indices(&self) -> impl Iterator<Item = usize> {
        let start = self.start() as usize;
        let end = self.start() as usize + self.length() as usize;
        struct Iter {
            end: usize,
            data_len: usize,
            current: usize,
        }
        impl Iterator for Iter {
            type Item = usize;
            fn next(&mut self) -> Option<Self::Item> {
                if self.current < self.end {
                    let result = self.current;
                    self.current += 1;
                    Some(result % self.data_len)
                } else {
                    None
                }
            }
        }

        Iter {
            end,
            data_len: self.data_len,
            current: start,
        }
    }
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
            x as usize
        }
        LoopMode::PingPong => {
            // adjust offset to face the direction of speed
            // subtracting loop_length in addition to 1.0
            let x = offset + if speed < 0.0 { -1.0 - loop_length } else { 0.0 };
            // normalize offset to be within 0..2*loop_length
            let x = utils::normalize_offset(x, 2.0 * loop_length);
            // undo the mirroring effectc
            let x = if x < loop_length {
                x
            } else {
                2.0 * loop_length - x - 1.0
            };
            let x = (start + x).round() % len_f32;
            x as usize
        }
        LoopMode::PlayOnce => {
            // play once does not bound the offset by loop length
            let x = offset + if speed < 0.0 { -1.0 } else { 0.0 };
            let x = if x < 0.0 { x + loop_length } else { x };
            let x = (start + x).round() % len_f32;
            x as usize
        }
    }
}

#[cfg(test)]
mod test1 {
    use crate::utils::ping_pong;

    fn float_eq_approx(a: f32, b: f32) -> bool {
        let diff = (a - b).abs();
        diff < 0.0001
    }

    fn assert_float_eq_approx(a: f32, b: f32) {
        let diff = (a - b).abs();
        assert!(diff < 0.0001, "a = {:?}, b = {:?}, diff = {:?}", a, b, diff);
    }

    #[test]
    fn test_calc_ping_pong() {
        let indices: Vec<_> = (0..10).map(|x| x as f32).collect();
        let rindices = indices.iter().copied().rev().collect::<Vec<_>>();
        let ones = vec![1.0f32; indices.len()];
        let minus_ones = vec![-1.0f32; indices.len()];
        let aa = indices
            .iter()
            .copied()
            .zip(ones.iter().copied())
            .collect::<Vec<_>>();
        let bb = rindices
            .iter()
            .copied()
            .zip(minus_ones.iter().copied())
            .collect::<Vec<_>>();

        let mut expected = vec![&aa, &bb, &aa, &bb, &aa, &bb, &aa]
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
            .concat();
        let mut tmp = vec![];
        for i in -40..30 {
            //eprintln!(
            //    "calc_ping_pong({})={:?}",
            //    i,
            //    calc_ping_pong(0.0, 10.0, i as f32)
            //);
            tmp.push(ping_pong(0.0, 10.0, i as f32));
        }
        assert_eq!(tmp, expected);
    }
}

pub fn calc_sample(
    loop_mode: LoopMode,
    loop_start_percent: f32,
    loop_length_percent: f32,
    offset: f32,
    speed: f32,
    data_len: usize,
) -> CalcSampleResult {
    let len_f32 = data_len as f32;

    // start offset, length, end offset
    let s = loop_start_percent * len_f32;
    let l = loop_length_percent * len_f32;
    let e = (s + l) % len_f32;

    // snap back to start if the position offset was pointing at
    // became out of bounds (e.g. due to parameter change)
    let clamped_offset = if s < e {
        if (s <= offset && offset < e) {
            offset
        } else {
            s
        }
    } else if e < s {
        if (0.0 <= offset && offset < e) || (s <= offset && offset < len_f32) {
            offset
        } else {
            s
        }
    } else {
        offset
    };

    let loop_offset = normalize_offset(clamped_offset - s, len_f32);

    //
    let (new_loop_offset, speed_change) = match loop_mode {
        LoopMode::Loop | LoopMode::PlayOnce => (normalize_offset(loop_offset + speed, l), 1.0),
        LoopMode::PingPong => ping_pong(loop_offset, l, speed),
    };

    let index_offset = if speed < 0.0 {
        // the sample to emit is one behind current offset if its playing backwards
        // this remaps the offset to 'loop' space, shifts it back by 1.0
        // and maps it back to 'buffer' space`
        //let local_offset = (offset + len_f32 - s) % len_f32;
        normalize_offset(normalize_offset(loop_offset - 1.0, l) + s, len_f32)
    } else {
        clamped_offset
    };

    let index = index_offset.round() as usize;

    let new_offset = normalize_offset(new_loop_offset + s, len_f32);
    eprintln!("s={:?}", s);
    eprintln!("e={:?}", e);
    eprintln!("l={:?}", l);
    eprintln!("offset={:?}", offset);
    eprintln!("clamped_offset={:?}", clamped_offset);
    eprintln!("index_offset={:?}", index_offset);
    eprintln!("index={:?}", index);
    eprintln!("loop_offset={:?}", loop_offset);
    eprintln!("new_loop_offset={:?}", new_loop_offset);
    eprintln!("new_offset={:?}", new_offset);
    eprintln!("speed_change={:?}", speed_change);

    CalcSampleResult {
        index,
        new_offset,
        speed_change,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calc_sample() {
        let loop_mode: LoopMode = LoopMode::Loop;
        let loop_start_percent = 0.8;
        let loop_length_percent = 0.7;
        let offset = 0.0;
        let speed = 1.0;
        let data_len = 100;
        let mut params = LoopConfig {
            loop_mode,
            loop_start_percent,
            loop_length_percent,
            offset,
            speed,
            data_len,
        };
        //
        let result = params.to_result();
        assert_eq!(result.index, 0);
        assert_eq!(result.new_offset, 1.0);
        assert_eq!(result.speed_change, 1.0);

        //
        params.speed = -1.0;
        let result = params.to_result();
        assert_eq!(result.index, 99);
        assert_eq!(result.new_offset, 99.0);
        assert_eq!(result.speed_change, 1.0);

        //
        params.speed = -1.0;
        params.offset = 80.0;
        let result = params.to_result();
        assert_eq!(result.index, 49);
        assert_eq!(result.new_offset, 49.0);
        assert_eq!(result.speed_change, 1.0);

        //
        params.speed -= params.loop_length_percent * params.data_len as f32;
        let result = params.to_result();
        assert_eq!(result.index, 49);
        assert_eq!(result.new_offset, 49.0);
        assert_eq!(result.speed_change, 1.0);
    }

    #[test]
    fn test_buffer_indices() {
        let loop_mode: LoopMode = LoopMode::PingPong;
        let loop_start_percent = 0.8;
        let loop_length_percent = 0.7;
        let offset = 0.0;
        let speed = 1.0;
        let data_len = 10;
        let mut params = LoopConfig {
            loop_mode,
            loop_start_percent,
            loop_length_percent,
            offset,
            speed,
            data_len,
        };
        let indices: Vec<_> = params.iter_indices().map(|x| x as f32).collect();
        let indices1 = indices
            .iter()
            .map(|i| params.buffer_to_loop(*i))
            .collect::<Vec<_>>();
        assert_eq!(
            indices1,
            (0..params.length() as usize)
                .map(|i| i as f32)
                .collect::<Vec<_>>()
        );
        let all = (0..params.data_len).map(|x| x as f32).collect::<Vec<_>>();
        let loop_indices = (0..params.length() as usize)
            .map(|x| x as f32)
            .collect::<Vec<_>>();
        assert_eq!(loop_indices, indices1);
        eprintln!("indices1={:?}", indices1);
        eprintln!("indices={:?}", indices);
        for x in &indices {
            assert!(params.loop_contains(*x));
        }
        for x in all {
            if !indices.contains(&x) {
                assert!(!params.loop_contains(x));
            }
        }
    }

    #[test]
    fn test_calc_sample_ping_pong() {
        let loop_mode: LoopMode = LoopMode::PingPong;
        let loop_start_percent = 0.8;
        let loop_length_percent = 0.7;
        let offset = 0.0;
        let speed = 1.0;
        let data_len = 10;
        let mut params = LoopConfig {
            loop_mode,
            loop_start_percent,
            loop_length_percent,
            offset,
            speed,
            data_len,
        };
        //
        let result = params.to_result();
        assert_eq!(result.index, 0);
        assert_eq!(result.new_offset, 1.0);
        assert_eq!(result.speed_change, 1.0);

        //
        params.speed = -1.0;
        let result = params.to_result();
        assert_eq!(result.index, 9);
        assert_eq!(result.new_offset, 9.0);
        assert_eq!(result.speed_change, 1.0);

        params.speed = 1.0;
        params.offset = 8.0;
        let result = params.to_result();
        assert_eq!(result.index, 8);
        assert_eq!(result.new_offset, 9.0);
        assert_eq!(result.speed_change, 1.0);

        params.speed = 1.0;
        params.offset = 3.0;
        let result = params.to_result();
        assert_eq!(result.index, 3);
        assert_eq!(result.new_offset, 4.0);
        assert_eq!(result.speed_change, 1.0);

        params.speed = 1.0;
        params.offset = 4.0;
        let result = params.to_result();
        assert_eq!(result.index, 4);
        assert_eq!(result.new_offset, 4.0);
        assert_eq!(result.speed_change, -1.0);
    }
}
