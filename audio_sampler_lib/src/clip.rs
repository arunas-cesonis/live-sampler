pub type T = f32;

const ZERO: T = 0.0;
const ONE: T = 1.0;
const TWO: T = 2.0;

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Mode {
    Loop,
    PingPong,
}

#[derive(Copy, Debug, Clone)]
pub struct Clip {
    pub since: usize,
    pub start: T,
    pub speed: f32,
    pub length: T,
    pub data_length: T,
    pub mode: Mode,
    pub shift: T,
}

impl Clip {
    pub fn new(since: usize, start: T, speed: f32, length: T, data_length: T, mode: Mode) -> Self {
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

    pub fn is_pingpong_reversing(&self, now: usize) -> bool {
        if self.mode != Mode::PingPong {
            return false;
        }
        let l = self.length;
        let s = self.speed;
        let r = if s < ZERO { ONE } else { ZERO };
        let dt = self.elapsed(now) as T;
        let x = self.shift as T + ((dt + r) as f32) * s;
        let x = x.abs() % ((TWO * l) as T);
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
            if self.is_pingpong_reversing(now) {
                self.shift = TWO * self.length - shift - ONE;
            } else {
                self.shift = shift;
            }
        } else {
            panic!("self.offset returned unreachable offset")
        }
        self.since = now;
    }

    pub fn update_mode(&mut self, now: usize, mode: Mode) {
        if mode == self.mode {
            return;
        }
        // many duplicated calculations here!
        let offset = self.offset(now);
        self.mode = mode;
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

    pub fn update_length(&mut self, now: usize, length: T) {
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

    pub fn update_data_length(&mut self, now: usize, data_length: T) {
        if data_length == self.data_length {
            return;
        }
        // many duplicated calculations here!
        let offset = self.offset(now);
        self.data_length = data_length;
        if offset >= self.data_length {
            self.shift = ZERO;
            self.since = now;
        }
    }

    pub fn elapsed(&self, now: usize) -> usize {
        now - self.since
    }

    pub fn clip_offset(&self, now: usize) -> T {
        let l = self.length;
        let s = self.speed;
        let r = if s < ZERO { ONE } else { ZERO };
        let dt = self.elapsed(now) as T;
        let x = self.shift as f32 + (dt + r) * s;

        // eprintln!("{:?}", -0.041839838_f32 + 2314184.0_f32);

        let x1 = match self.mode {
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

        // need to clamp x1 due to possible floating poing error, e.g.
        // -0.041839838_f32 + 2314184.0_f32 = 2314184.0_f32
        let x1 = x1.min(l - 1.0);
        assert!(
            x1 >= 0.0 && x1 < l,
            "x={} speed={} length={} elapsed={} {:#?}",
            x1,
            s,
            l,
            dt,
            self
        );
        x1
    }

    pub fn offset(&self, now: usize) -> T {
        let x = self.clip_offset(now);
        let x = (self.start + x) % (self.data_length as T);
        x
    }
}
