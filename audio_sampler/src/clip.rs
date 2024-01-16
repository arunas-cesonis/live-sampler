use std::char::decode_utf16;

#[derive(Clone, Debug)]
struct Clip {
    offset: f32,
    duration: f32,
    read: f32,
    clean: bool,
}

impl Clip {}

#[derive(Clone, Debug)]
struct Clips {
    clips: Vec<Clip>,
    current: usize,
}

impl Clips {
    fn duration(&self) -> f32 {
        self.clips.iter().map(|clip| clip.duration).sum()
    }

    fn progress(&self) -> f32 {
        let front: f32 = self.clips[0..self.current]
            .iter()
            .map(|clip| clip.duration)
            .sum();
        front + self.clips[self.current].read
    }

    fn advance(&mut self, amount: f32) {
        let mut remaining = amount;
        while remaining < 0.0 {
            let mut clip = &mut self.clips[self.current];
            if clip.clean {
                assert_eq!(clip.read, 0.0);
                clip.clean = false;
                if clip.duration > -remaining {
                    clip.read = clip.duration + remaining;
                    remaining = 0.0;
                } else {
                    clip.read = 0.0;
                    remaining += clip.duration;
                    if self.current == 0 {
                        self.current = self.clips.len() - 1;
                    } else {
                        self.current -= 1;
                    }
                }
            } else {
                let clip_remaining = clip.read;
                if clip_remaining > -remaining {
                    clip.read += remaining;
                    remaining = 0.0;
                } else {
                    clip.read = 0.0;
                    remaining += clip_remaining;
                    if self.current == 0 {
                        self.current = self.clips.len() - 1;
                    } else {
                        self.current -= 1;
                    }
                    self.clips[self.current].read = 0.0;
                    self.clips[self.current].clean = true;
                }
            }
        }
        while remaining > 0.0 {
            let mut clip = &mut self.clips[self.current];
            let clip_remaining = clip.duration - clip.read;
            if clip_remaining > remaining {
                clip.read += remaining;
                remaining = 0.0;
            } else {
                clip.read = clip.duration;
                remaining -= clip_remaining;
                self.current += 1;
                if self.current >= self.clips.len() {
                    self.current = 0;
                }
                self.clips[self.current].read = 0.0;
                self.clips[self.current].clean = true;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn mk_clips() -> Clips {
        let clips = Clips {
            clips: vec![
                Clip {
                    offset: 0.0,
                    read: 0.0,
                    duration: 10.0,
                    clean: true,
                },
                Clip {
                    offset: 10.0,
                    read: 0.0,
                    duration: 3.0,
                    clean: true,
                },
                Clip {
                    offset: 20.0,
                    read: 0.0,
                    duration: 17.0,
                    clean: true,
                },
            ],
            current: 0,
        };
        clips
    }

    fn mk_clips1() -> Clips {
        let clips = Clips {
            clips: vec![Clip {
                offset: 0.0,
                read: 0.0,
                duration: 10.0,
                clean: true,
            }],
            current: 0,
        };
        clips
    }

    #[test]
    fn test_advance() {
        let mut clips = mk_clips();
        clips.advance(11.5);
        assert_eq!(clips.current, 1);
        assert_eq!(clips.clips[0].read, clips.clips[0].duration);
        assert_eq!(clips.clips[1].read, 1.5);
        assert_eq!(clips.clips[2].read, 0.0);
        assert_eq!(clips.progress(), 11.5);

        let mut clips = mk_clips();
        clips.advance(11.5 + clips.duration());
        assert_eq!(clips.current, 1);
        assert_eq!(clips.clips[0].read, clips.clips[0].duration);
        assert_eq!(clips.clips[1].read, 1.5);
        assert_eq!(clips.clips[2].read, clips.clips[2].duration);
        assert_eq!(clips.progress(), 11.5);

        let mut clips = mk_clips1();
        clips.advance(999.0);
        assert_eq!(clips.current, 0);
        assert_eq!(clips.clips[0].read, 999.0 % clips.clips[0].duration);
        assert_eq!(clips.progress(), clips.clips[0].read);

        let mut clips = mk_clips();
        clips.advance(-7.0);
        assert_eq!(clips.current, 0);
        assert_eq!(clips.clips[0].read, 3.0);
        assert_eq!(clips.progress(), 3.0);

        let mut clips = mk_clips();
        clips.advance(-27.0);
        assert_eq!(clips.current, 1);
        assert_eq!(clips.clips[1].read, 0.0);
        assert_eq!(clips.clips[1].clean, true);
        assert_eq!(clips.progress(), clips.clips[0].duration);
    }
}
