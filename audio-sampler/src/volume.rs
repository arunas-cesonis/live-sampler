use std::ops::Range;

#[derive(Clone, Debug)]
pub enum Volume {
    Static(f32),
    Linear {
        time: Range<usize>,
        value: Range<f32>,
    },
}
impl Default for Volume {
    fn default() -> Self {
        Volume::Static(1.0)
    }
}

impl Volume {
    pub fn new(value: f32) -> Self {
        Volume::Static(value)
    }
    #[allow(unused)]
    pub fn is_static_and_mute(&self) -> bool {
        match self {
            Volume::Static(x) => *x == 0.0,
            _ => false,
        }
    }
    #[allow(unused)]
    pub fn is_static(&self) -> bool {
        match self {
            Volume::Static(_) => true,
            Volume::Linear { .. } => false,
        }
    }
    pub fn value(&self, now: usize) -> f32 {
        match self {
            Volume::Linear { time, value } => {
                let t = (now - time.start) as f32 / (time.end - time.start) as f32;
                assert!(t >= 0.0);
                assert!(t <= 1.0);
                value.start + (value.end - value.start) * t
            }
            Volume::Static(value) => *value,
        }
    }
    #[allow(unused)]
    pub fn set(&mut self, value: f32) {
        *self = Volume::Static(value)
    }
    pub fn to(&mut self, now: usize, duration: usize, target: f32) {
        if duration == 0 {
            *self = Volume::Static(target);
        } else if duration > 0 {
            let initial = self.value(now);
            *self = Volume::Linear {
                time: now..(now + duration),
                value: initial..target,
            }
        } else {
            panic!("duration={duration}")
        }
    }
    pub fn step(&mut self, now: usize) {
        match self {
            Volume::Linear { time, value } => {
                assert!(now >= time.start);
                if now >= time.end {
                    *self = Volume::Static(value.end)
                }
            }
            Volume::Static(_) => (),
        }
    }
}
