use std::ops::Range;
/*
#[derive(Default)]
pub struct Channel {
    pub tape: Tape,
    pub voices: Vec<Voice>
}

#[derive(Default)]
pub struct Tape {
    data : Vec<f32>,
    write: usize
}

impl Tape {
    pub fn write(&mut self, sample: f32) {
        if self.write  < self.data.len() {
            self.data[self.write] = sample;
        } else {
            self.data.push(sample);
        }
    }
    pub fn truncate_write(&mut self) {
        self.data.truncate(self.write);
        self.write = 0;
    }
    pub fn calc_sample_pos(&self, read: f32) -> usize {
        let len_f32 = (self.data.len() as f32);
        let i = read % len_f32;
        let i = if i < 0.0 { i + len_f32 } else { i };
        let i = i as usize;
        i
    }
    pub fn read_sample(&self, read: usize) -> f32 {
        self.data[read]
    }
}

pub struct Voice {
    read: f32,
    speed: f32,
    volume: Volume,
    finished: bool
}

impl Voice {
    pub fn new(read: f32, initial_volume: f32) -> Self {
        Self {
            read,
            speed: 1.0,
            finished: false,
            volume: Volume::new(initial_volume)
        }
    }
    pub fn get_read(&self) -> f32 {
        self.read
    }
    pub fn get_volume(&self, now: usize) -> f32 {
        self.volume.value(now)
    }
    pub fn set_finished(&mut self) {
        self.finished = true;
    }
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
    pub fn is_mute_and_finished(&self, now: usize) -> bool {
        self.finished && self.volume.is_static_and_mute()
    }
    pub fn reverse(&mut self) {
        self.speed = -self.speed;
    }
    pub fn fade(&mut self, now: usize, duration: usize, target: f32) {
        self.volume.to(now, duration, target);
    }
    pub fn advance(&mut self, now: usize) {
        self.read += self.speed;
        self.volume.step(now);
    }
}

pub fn play_voice(tape: &Tape, voice: &mut Voice, now: usize) -> f32 {
    let sample = tape.data[tape.calc_sample_pos(voice.read)] * voice.get_volume(now);
    voice.advance(now);
    sample
}

 */
#[derive(Clone, Debug)]
pub enum Volume {
    Static(f32),
    Linear {
        time: Range<usize>,
        value: Range<f32>,
    },
}

impl Volume {
    pub fn new(value: f32) -> Self {
        Volume::Static(value)
    }
    pub fn is_static_and_mute(&self) -> bool {
        match self {
            Volume::Static(x) => *x == 0.0,
            _ => false
        }
    }
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
