use crate::sampler::LoopMode;
use crate::utils::ping_pong2;
use log::log_enabled;

#[derive(Debug, Clone)]
pub struct Clip {
    offset: f32,
    length: f32,
    ping_pong: f32,
}

impl Clip {
    pub fn advance(&mut self, loop_mode: LoopMode, amount: f32) -> f32 {
        match loop_mode {
            LoopMode::PlayOnce => {
                let new_offset = self.offset + amount * self.ping_pong;
                if new_offset >= self.length {
                    self.offset = self.length - 1.0;
                    new_offset - self.offset
                } else if new_offset < 0.0 {
                    self.offset = 0.0;
                    -new_offset
                } else {
                    self.offset = new_offset;
                    0.0
                }
            }
            LoopMode::Loop => {
                let new_offset = self.offset + amount * self.ping_pong;
                if new_offset >= self.length {
                    self.offset = new_offset - self.length;
                    0.0
                } else if new_offset < 0.0 {
                    self.offset = self.length + new_offset;
                    0.0
                } else {
                    self.offset = new_offset;
                    0.0
                }
            }
            LoopMode::PingPong => {
                let (new_offset, speed_change) =
                    ping_pong2(self.offset + amount * self.ping_pong, self.length);
                self.offset = new_offset;
                self.ping_pong *= speed_change;
                0.0

                //let speed = amount * self.ping_pong;
                //let new_offset = self.offset + speed;
                //if new_offset >= self.length - 1.0 {
                //    self.offset = self.length - (new_offset - self.length) - 2.0;
                //    self.ping_pong = -self.ping_pong;
                //    0.0
                //} else if new_offset < 0.0 {
                //    self.offset = -new_offset;
                //    self.ping_pong = -self.ping_pong;
                //    0.0
                //} else {
                //    self.offset = new_offset;
                //    0.0
                //}
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::ping_pong2;
    use std::time::Instant;

    fn advance_n(
        clip: &mut Clip,
        loop_mode: LoopMode,
        times: usize,
        amount: f32,
    ) -> Vec<(f32, Clip)> {
        std::iter::repeat_with(|| {
            let tmp = clip.clone();
            let r = clip.advance(loop_mode, amount);
            (r, tmp)
        })
        .take(times)
        .collect()
    }

    fn print_lines<A>(v: Vec<A>, per_line: usize) -> String
    where
        A: std::fmt::Debug,
    {
        let mut i = 0;
        let mut out = String::new();

        while i < v.len() {
            out.push_str(format!("{:<4}: {:?}\n", i, &v[i..(i + per_line).min(v.len())]).as_str());
            i += per_line;
        }
        out + "\n"
    }

    #[test]
    fn test_clip() {
        let mut clip = Clip {
            length: 5.0,
            offset: 0.0,
            ping_pong: 1.0,
        };
        let mut speed = 1.0;
        let mut out = vec![];
        for i in 0..100 {
            out.push(clip.offset);
            clip.advance(LoopMode::PingPong, 1.0);
        }
        eprintln!("{}", print_lines(out, 5));
    }
}
