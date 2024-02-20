#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Mode {
    Loop,
    PingPong,
}

#[derive(Debug, Clone)]
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

    pub fn update_length(&mut self, now: usize, length: f32) {
        if length == self.length {
            return;
        }
        let offset = self.offset(now);
        if offset < length {
            eprintln!("A");
            self.shift = offset - self.start;
            self.length = length;
            self.since = now;
        } else {
            eprintln!("B");
            self.length = length;
            self.since = now;
        }
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

    pub fn offset(&self, now: usize) -> f32 {
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
            start: 5.0,
            speed: -2.0,
            length: 10.0,
            data_length: 100.0,
            mode: crate::clip::Mode::PingPong,
            shift: 0.0,
        };
        let k = 100;
        for i in 0..(k - 1) {
            match i {
                _ => (),
            };
            let x = clip.offset(i);
            eprintln!(
                "{:<4} {:<4} dt={:<4} sh={:<4}",
                i,
                x,
                clip.elapsed(i),
                clip.shift
            );
        }
    }
}
