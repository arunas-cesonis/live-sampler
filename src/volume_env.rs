#[derive(Clone, Default, Debug, PartialEq)]
pub struct VolumeEnv {
    start: usize,
    duration: usize,
    initial: f32,
    target: f32,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct PolyVolumeEnv {
    channels: Vec<VolumeEnv>,
}
impl PolyVolumeEnv {
    pub fn new(initial: Vec<f32>) -> Self {
        Self {
            channels: initial.into_iter().map(VolumeEnv::new).collect(),
        }
    }
    pub fn retrigger(&mut self, now: &[usize], start: &[usize], duration: usize, target: f32) {
        self.channels
            .iter_mut()
            .enumerate()
            .for_each(|(i, env)| env.retrigger(now[i], start[i], duration, target))
    }
    pub fn value(&self, channel: usize, now: usize) -> f32 {
        self.channels[channel].value(now)
    }
    pub fn channels(&self) -> &[VolumeEnv] {
        &self.channels
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    NotStarted,
    InProgress,
    Finished,
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
    pub fn end(&self) -> usize {
        self.start + self.duration
    }
    pub fn status(&self, now: usize) -> Status {
        if now >= self.end() {
            Status::Finished
        } else if now <= self.start {
            Status::NotStarted
        } else {
            Status::InProgress
        }
    }
    pub fn value(&self, now: usize) -> f32 {
        match self.status(now) {
            Status::Finished => self.target,
            Status::NotStarted => self.initial,
            Status::InProgress => {
                let t = ((now - self.start) as f32 / self.duration as f32).clamp(0.0, 1.0);
                //eprintln!("t={}", t);
                //eprintln!("d={}", self.target - self.initial);
                //eprintln!("d={}", self.initial);
                let y = self.initial + (self.target - self.initial) * t;
                y
            }
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
