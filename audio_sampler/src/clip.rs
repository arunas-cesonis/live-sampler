use std::fs::read;

struct Clip {
    start: f32,
    end: f32,
}

impl Clip {
    pub fn new(start: f32, end: f32) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    pub fn duration(&self) -> f32 {
        self.end - self.start
    }
}

#[derive(Default, Debug, Clone)]
struct Position {
    index: usize,
    offset: f32,
}

struct Clips {
    clips: Vec<Clip>,
}

impl Position {
    pub fn advance(&mut self, clips: &Clips, speed: f32) {
        let mut remaining = speed;
        while (remaining > 0.0) {
            let clip = &clips.clips[self.index];
            let clip_remaining = clip.duration() - self.offset;
        }
    }
}
