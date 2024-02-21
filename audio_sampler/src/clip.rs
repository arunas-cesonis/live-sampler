use nih_plug::nih_warn;

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Mode {
    Loop,
    PingPong,
}

#[derive(Copy, Debug, Clone)]
pub struct Clip {
    pub since: usize,
    pub start: f32,
    pub speed: f32,
    pub length: f32,
    pub data_length: f32,
    pub mode: Mode,
    pub shift: f32,
}

impl Clip {
    pub fn new(
        since: usize,
        start: f32,
        speed: f32,
        length: f32,
        data_length: f32,
        mode: Mode,
    ) -> Self {
        Self {
            since,
            start,
            speed,
            length,
            data_length,
            mode,
            shift: 0.0,
        }
    }

    pub fn clip_to_data(&self, x: f32) -> f32 {
        assert!(0.0 <= x && x < self.length);
        let x = (self.start + x) % self.data_length;
        x
    }

    pub fn data_to_clip(&self, x: f32) -> Option<f32> {
        assert!(0.0 <= x && x < self.data_length);
        if x >= self.start {
            let x = x - self.start;
            if x < self.length {
                Some(x)
            } else {
                None
            }
        } else {
            let x = x + self.data_length - self.start;
            if x < self.length {
                Some(x)
            } else {
                None
            }
        }
    }

    fn is_pingpong_reversing(&self, now: usize) -> bool {
        let l = self.length;
        let s = self.speed;
        let r = if s < 0.0 { 1.0 } else { 0.0 };
        let dt = self.elapsed(now) as f32;
        let x = self.shift + (dt + r) * s;
        let x = x.abs() % (2.0 * l);
        x >= l
    }

    pub fn update_speed(&mut self, now: usize, speed: f32) {
        if speed == self.speed {
            return;
        }
        // many duplicated calculations here!
        nih_warn!("now={:<8} SPEED: {:<4} -> {:<4}", now, self.speed, speed);
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

    pub fn update_length(&mut self, now: usize, length: f32) {
        if length == self.length {
            return;
        }
        // many duplicated calculations here!
        let offset = self.offset(now);
        self.length = length;
        if let Some(shift) = self.data_to_clip(offset) {
            if self.is_pingpong_reversing(now) {
                self.shift = 2.0 * self.length - shift - 1.0;
            } else {
                self.shift = shift;
            }
        } else {
            self.shift = 0.0;
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

    pub fn clip_offset(&self, now: usize) -> f32 {
        let l = self.length;
        let s = self.speed;
        let r = if s < 0.0 { 1.0 } else { 0.0 };
        let dt = self.elapsed(now) as f32;
        let x = self.shift + (dt + r) * s;
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

    pub fn offset(&self, now: usize) -> f32 {
        let x = self.clip_offset(now);
        let x = (self.start + x) % self.data_length;
        x
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_update() {
        let mut clip = crate::clip::Clip {
            since: 0,
            start: 40.0,
            speed: 0.5,
            length: 100.0,
            data_length: 1000.0,
            mode: crate::clip::Mode::PingPong,
            shift: 0.0,
        };
        let mut prev = 0.0;
        for i in 0..4410 {
            if i >= 200 && i % 25 == 0 {
                clip.update_speed(i, -clip.speed);
            }
            let x = clip.offset(i);
            let xx = clip.clip_offset(i);
            let dx = x - prev;
            eprintln!(
                "i={:<5} x={:<5} xx={:<5} dx={:<5} length={:<5} speed={:<5} shift={:<5}",
                i, x, xx, dx, clip.length, clip.speed, clip.shift
            );
            prev = x;
        }
    }
}
