#[derive(Clone, Default, Debug, PartialEq)]
pub struct VolumeEnv {
    start: usize,
    duration: usize,
    initial: f32,
    target: f32,
}

impl VolumeEnv {
    pub fn new(initial: f32) -> Self {
        Self {
            start: 0,
            duration: 0,
            initial,
            target: initial,
        }
    }
    /// Cancel current envelope and continue from current value to new target
    pub fn retrigger(&mut self, now: usize, start: usize, duration: usize, target: f32) {
        self.initial = self.value(now);
        self.start = start;
        self.duration = duration;
        self.target = target;
    }
    pub fn value(&self, now: usize) -> f32 {
        let end = self.start + self.duration;
        if now > end {
            self.target
        } else if now <= self.start {
            self.initial
        } else {
            let t = ((now - self.start) as f32 / self.duration as f32).clamp(0.0, 1.0);
            //eprintln!("t={}", t);
            //eprintln!("d={}", self.target - self.initial);
            //eprintln!("d={}", self.initial);
            let y = self.initial + (self.target - self.initial) * t;
            y
        }
    }
}

#[cfg(test)]
mod test {
    use super::VolumeEnv;

    #[test]
    fn test_retrigger() {
        let mut vol = VolumeEnv::new(1.0);
        assert_eq!(1.0, vol.value(0));
        assert_eq!(1.0, vol.value(100));
        assert_eq!(1.0, vol.value(151));
        // start envelope from 1.0 to 0.0 over 100 samples
        vol.retrigger(0, 50, 100, 0.0);
        assert_eq!(1.0, vol.value(0));
        assert_eq!(0.5, vol.value(100));
        assert_eq!(0.0, vol.value(151));
        // re-trigger envelope from current value to 1.0 over 10 samples from 100th sample
        vol.retrigger(100, 100, 10, 1.0);
        assert_eq!(0.5, vol.value(100));
        assert_eq!(1.0, vol.value(110));
        assert_eq!(1.0, vol.value(151));
    }
}
