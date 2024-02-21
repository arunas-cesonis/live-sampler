use nih_plug::nih_warn;
use num_traits::One;
use crate::sampler::LoopMode;

pub type T = f32;

pub const ZERO: T = 0.0;
pub const ONE: T = 1.0;
pub const TWO: T = 2.0;

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Mode {
    Loop,
    PingPong,
}

#[derive(Copy, Debug, Clone)]
pub struct Clip2 {
    pub since: usize,
    pub start: T,
    pub speed: f32,
    pub length: T,
    pub data_length: T,
    pub mode: Mode,
    pub shift: T,
}

impl Clip2 {
    pub fn new(
        since: usize,
        start: T,
        speed: f32,
        length: T,
        data_length: T,
        mode: Mode,
    ) -> Self {
        let length = length.max(ONE);
        Self {
            since,
            start,
            speed,
            length,
            data_length,
            mode,
            shift: ZERO,
        }
    }

    pub fn clip_to_data(&self, x: T) -> T {
        assert!(ZERO <= x && x < self.length);
        let x = (self.start + x) % (self.data_length as T);
        x
    }

    pub fn data_to_clip(&self, x: T) -> Option<T> {
        assert!(ZERO <= x && x < self.data_length as T);
        if x >= self.start {
            let x = x - self.start;
            if x < self.length {
                Some(x)
            } else {
                None
            }
        } else {
            let x = x + (self.data_length as T) - self.start;
            if x < self.length {
                Some(x)
            } else {
                None
            }
        }
    }

    fn is_pingpong_reversing(&self, now: usize) -> bool {
        if self.mode != Mode::PingPong {
            return false;
        }
        let l = self.length;
        let s = self.speed;
        let r = if s < ZERO { ONE } else { ZERO };
        let dt = self.elapsed(now) as T;
        let x = self.shift as T + ((dt + r) as f32) * s;
        let x = (x.abs() % ((TWO * l) as T));
        x >= l
    }

    pub fn update_speed(&mut self, now: usize, speed: T) {
        if speed == self.speed {
            return;
        }
        // many duplicated calculations here!
        let offset = self.offset(now);
        self.speed = speed;
        // FIXME: avoid local -> global -> local conversion
        if let Some(shift) = self.data_to_clip(offset) {
            self.shift = shift;
        } else {
            panic!("self.offset returned unreachable offset")
        }
        self.since = now;
    }

    pub fn update_length(&mut self, now: usize, length: T) {
        let length = length.max(ONE);
        if length == self.length {
            return;
        }
        // many duplicated calculations here!
        let offset = self.offset(now);
        self.length = length;
        if let Some(shift) = self.data_to_clip(offset) {
            if self.is_pingpong_reversing(now) {
                self.shift = TWO * self.length - shift - ONE;
            } else {
                self.shift = shift;
            }
        } else {
            self.shift = ZERO;
        }
        self.since = now;
    }

    pub fn shift(&self, now: usize) -> Self {
        let mut tmp = self.clone();
        tmp.shift = tmp.offset(now);
        tmp.since = now;
        tmp
    }

    pub fn elapsed(&self, now: usize) -> usize {
        now - self.since
    }

    pub fn clip_offset(&self, now: usize) -> T {
        let l = self.length;
        let s = self.speed;
        let r = if s < ZERO { ONE } else { ZERO };
        let dt = self.elapsed(now) as T;
        let x = (self.shift as f32 + (dt + r) * s);
        let x = match self.mode {
            Mode::Loop => {
                let x = x % l;
                if x >= 0.0 {
                    x
                } else {
                    x + l
                }
            }
            Mode::PingPong => {
                let x = x.abs() % (2.0 * l);
                if x < l {
                    x
                } else {
                    (2.0 * l - x - 1.0).max(0.0)
                }
            }
        };
        assert!(
            x >= 0.0 && x < l,
            "x={} speed={} length={} elapsed={}",
            x,
            s,
            l,
            dt
        );
        x
    }

    pub fn offset(&self, now: usize) -> T {
        let x = self.clip_offset(now);
        let x = (self.start + x) % (self.data_length as T);
        x
    }
}

#[cfg(test)]
mod test {
    use nih_plug_vizia::vizia::input::Code::MediaPlayPause;
    use rand::prelude::SmallRng;
    use rand::{RngCore, SeedableRng};
    use crate::clip2::Clip2;
    use crate::clip2::Mode::Loop;
    use crate::clip::Clip;

    fn rand_f32(rng: &mut SmallRng) -> f32 {
        let x = rng.next_u32() as f32 / u32::MAX as f32;
        x
    }

    const DEFAULT_PRECISION: f32 = 0.0001;

    fn same(x: f32, y: f32) -> bool {
        (x - y).abs() < DEFAULT_PRECISION
    }

    #[test]
    fn test2_() {

        // index1=21535 index2=21652 d=false voice=Voice { note: Note { note: 16, channel: 1 }, loop_start_percent: 0.25, played: 543.0, clip2: Clip2 { since: 239232, start: 21512, speed: 1.0, length: 164, data_length: 86049, mode: Loop, shift: 304 }, clip: Clip { offset: 21512, length: 164, local_adjustment: 23, updated_at: 239232, speed: 1.0 }, volume: Static(1.0), finished: false, last_sample_index: 21534 }
        // Clip { offset: 21512, length: 164, local_adjustment: 23, updated_at: 239232, speed: 1.0 }
        // Clip2 { since: 239232, start: 21512, speed: 1.0, length: 164, data_length: 86049, mode: Loop, shift: 304 }
        let clip1 = Clip {
            offset: 21512,
            length: 164,
            local_adjustment: 23,
            updated_at: 239232,
            speed: 1.0,
        };
        //let mut clip22 = Clip2 { since: 238848, start: 21512, speed: 1.0, length: 163, data_length: 86049, mode: Loop, shift: 128 };
        //let mut clip2 = Clip2 { since: 239232, start: 21512, speed: 1.0, length: 164, data_length: 86049, mode: Loop, shift: 304 };
        //let x = clip1.sample_index(239232, 86049);
        //let y = clip2.offset(239232);
        //clip22.update_length(239232, 164);
        //eprintln!("{:?}", clip22);
        //eprintln!("x={} y={}", x, y);
    }
    /*
        #[test]
        fn test_basic() {
            let mut rng = SmallRng::from_seed([0; 32]);
            let mut clip = crate::clip2::Clip2 {
                since: 0,
                start: 4,
                speed: 0.5,
                length: 100,
                data_length: 1000,
                mode: crate::clip2::Mode::PingPong,
                shift: 0,
            };
            let mut prev = clip.start;
            #[derive(Default, Debug)]
            struct Count {
                max_more: f32,
                max_less: f32,
                more: usize,
                less: usize,
                same: usize,
                changes: usize,
            }
            let mut count = Count::default();
            for i in 0..4410 {
                if rng.next_u32() % 25 == 0 {
                    let speed = rand_f32(&mut rng) * 2.0 - 1.0;
                    let speed = speed * 2.0;
                    count.changes += 1;
                    clip.update_speed(if i > 0 { i - 1 } else { i }, speed);
                }
                let x = clip.offset(i);
                let clip_x = clip.clip_offset(i);
                eprintln!("i={:<5} x={:<5} clip_x={:<5}", i, x, clip_x);
            }
            eprintln!("count={:?}", count);
        }
        */
}
