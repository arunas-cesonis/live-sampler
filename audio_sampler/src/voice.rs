use crate::common_types::Params;
use crate::sampler::LoopMode;
use crate::utils;
use crate::utils::normalize_offset;
use crate::volume::Volume;
use nih_plug::wrapper::vst3::vst3_sys::vst::EventTypes::kNoteOffEvent;
use nih_plug_vizia::vizia::views::virtual_list_derived_lenses::offset;
use smallvec::SmallVec;
use std::ops::Index;

#[derive(Clone, Debug)]
pub struct Voice {
    pub note: u8,
    pub loop_start_percent: f32,
    pub offset: f32,
    pub played: f32,
    pub volume: Volume,
    pub finished: bool,
    // this is only used by the UI to show loop points
    // its hack/workaround for not having loop information easily available
    pub last_sample_index: usize,
}

impl Voice {
    pub fn process_sample(&mut self, data: &[f32], params: &Params) -> f32 {
        let len_f32 = data.len() as f32;
        let s = self.loop_start_percent * len_f32;
        let l = params.loop_length_percent * len_f32;
        let e = (s + l) % len_f32;
        0.0
    }
}

pub fn advance_loop(seq: &mut Sequence, amount: f32) {
    debug_assert!(seq.duration() > 0.0);
    let mut remaining = amount;
    while remaining > 0.0 {
        remaining = seq.advance(remaining);
        if remaining > 0.0 {
            assert!(seq.is_at_end());
            seq.reset();
        } else if remaining < 0.0 {
            assert!(seq.is_at_start());
            seq.reset_to_end();
        }
    }
    eprintln!("remaining={}", remaining)
}

#[derive(Debug, Clone)]
pub struct Position {
    position: f32,
    index: usize,
}

impl Position {
    pub fn start(seq: &Sequence) -> Self {
        Self {
            position: seq.v[0].start,
            index: 0,
        }
    }

    pub fn end(seq: &Sequence) -> Self {
        Self {
            position: seq.v[seq.v.len() - 1].end,
            index: seq.v.len() - 1,
        }
    }

    pub fn is_at_end(&self, seq: &crate::voice::Sequence) -> bool {
        self.index + 1 == seq.v.len() && self.position == seq.v[self.index].end
    }

    pub fn is_at_start(&self, seq: &crate::voice::Sequence) -> bool {
        self.index == 0 && self.position == seq.v[0].start
    }

    pub fn position(&self) -> f32 {
        self.position
    }

    pub fn advance(&mut self, seq: &Sequence, amount: f32) -> f32 {
        let mut remaining = amount;
        let mut c = 0;
        while remaining != 0.0 {
            c += 1;
            if c > 100 {
                eprintln!(
                    "c={} index={} position={} remaining={}",
                    c, self.index, self.position, remaining
                );
                panic!("c={}", c);
            }
            let clip = &seq.v[self.index];
            let new_position = self.position + remaining;
            if new_position >= clip.end {
                remaining = new_position - clip.end;
                if self.index + 1 < seq.v.len() {
                    self.index += 1;
                    self.position = seq.v[self.index].start;
                } else {
                    self.position = seq.v[self.index].end;
                    return remaining;
                }
            } else if new_position <= clip.start {
                remaining = clip.start - new_position;
                if self.index > 0 {
                    self.index -= 1;
                    self.position = seq.v[self.index].end;
                } else {
                    self.position = seq.v[self.index].start;
                    return remaining;
                }
            } else {
                remaining = 0.0;
                self.position = new_position;
                break;
            };
        }
        remaining
    }
}

#[derive(Debug, Clone)]
pub struct Sequence {
    v: SmallVec<[Clip; 4]>,
    index: usize,
}

impl Sequence {
    pub fn new(v: SmallVec<[Clip; 4]>) -> Self {
        assert!(!v.is_empty());
        Self { v, index: 0 }
    }

    fn next_clip(&mut self) -> bool {
        if self.index + 1 < self.v.len() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    pub fn advance(&mut self, amount: f32) -> f32 {
        let mut remaining = amount;
        let mut c = 0;
        while remaining != 0.0 {
            let clip = &mut self.v[self.index];
            remaining = clip.advance(remaining);

            if remaining > 0.0 {
                if self.index + 1 == self.v.len() {
                    break;
                }
                self.index += 1;
                self.v[self.index].reset();
            } else if remaining < 0.0 {
                if self.index == 0 {
                    break;
                }
                self.index -= 1;
                self.v[self.index].reset_to_end();
            } else {
                //while self.v[self.index].is_at_end()
                break;
            }
        }
        remaining
    }

    pub fn duration(&self) -> f32 {
        self.v.iter().map(|clip| clip.duration()).sum()
    }

    pub fn seek_position(&mut self, position: f32) -> bool {
        let mut i = 0;
        while !self.v[i].seek_position(position) {
            i += 1;
            if i == self.v.len() {
                return false;
            }
        }
        self.index = i;
        for j in 0..i {
            self.v[j].reset_to_end();
        }
        for j in (i + 1)..self.v.len() {
            self.v[j].reset();
        }
        true
    }

    pub fn remaining(&self) -> f32 {
        self.v[self.index].remaining()
            + self.v[(self.index + 1)..]
                .iter()
                .map(|clip| clip.duration())
                .sum::<f32>()
    }

    pub fn remaining_to_start(&self) -> f32 {
        self.v[self.index].remaining_to_start()
            + self.v[..self.index]
                .iter()
                .map(|clip| clip.duration())
                .sum::<f32>()
    }

    pub fn is_at_start(&self) -> bool {
        self.index == 0 && self.v[0].is_at_start()
    }

    pub fn is_at_end(&self) -> bool {
        self.index == self.v.len() - 1 && self.v[self.index].is_at_end()
    }

    pub fn reset(&mut self) {
        self.index = 0;
        for clip in &mut self.v {
            clip.reset();
        }
    }

    pub fn reset_to_end(&mut self) {
        self.index = self.v.len() - 1;
        for clip in &mut self.v {
            clip.reset_to_end();
        }
    }

    pub fn position(&self) -> f32 {
        self.v[self.index].position()
    }

    pub fn show(&self) -> String {
        self.v
            .iter()
            .enumerate()
            .map(|(index, clip)| {
                let completion = (clip.position() - clip.start) / clip.duration();
                if index == self.index {
                    format!(
                        "[{:>7.2}% / {} {} {}]",
                        completion * 100.0,
                        clip.start,
                        clip.position,
                        clip.end
                    )
                } else {
                    format!(
                        " {:>7.2}% / {} {} {}  ",
                        completion * 100.0,
                        clip.start,
                        clip.position,
                        clip.end
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct Clip {
    start: f32,
    end: f32,
    position: f32,
}

impl Clip {
    pub fn new(start: f32, end: f32) -> Self {
        assert!(start <= end);
        Self {
            start,
            end,
            position: start,
        }
    }

    pub fn remaining(&self) -> f32 {
        self.end - self.position
    }

    pub fn remaining_to_start(&self) -> f32 {
        self.position - self.start
    }

    pub fn is_at_start(&self) -> bool {
        self.position == self.start
    }

    pub fn is_at_end(&self) -> bool {
        self.position == self.end
    }

    pub fn seek_position(&mut self, position: f32) -> bool {
        if self.start <= position && position <= self.end {
            self.position = position;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.position = self.start
    }

    pub fn reset_to_end(&mut self) {
        self.position = self.end
    }

    pub fn duration(&self) -> f32 {
        self.end - self.start
    }

    pub fn advance(&mut self, amount: f32) -> f32 {
        let new_position = self.position + amount;
        if new_position > self.end {
            self.position = self.end;
            new_position - self.end
        } else if new_position < self.start {
            self.position = self.start;
            //eprintln!("new_position={} start={}", new_position, self.start);
            new_position - self.start
        } else {
            self.position = new_position;
            0.0
        }
    }

    pub fn position(&self) -> f32 {
        self.position
    }
}

#[cfg(test)]
mod test {
    use crate::voice::{advance_loop, Clip, Position, Sequence};
    use serde::de::Unexpected::Seq;
    use smallvec::smallvec;

    #[test]
    fn test_position() {
        let seq = Sequence::new(smallvec![Clip::new(0.0, 3.0), Clip::new(5.0, 7.00)]);
        let mut pos = Position::start(&seq);
        // let mut seq = Sequence::new(smallvec![Clip::new(0.0, 3.0)]);
        while !pos.is_at_end(&seq) {
            eprintln!(
                "p={:?} s={} e={}",
                pos.position,
                pos.is_at_start(&seq),
                pos.is_at_end(&seq)
            );
            pos.advance(&seq, 1.0);
        }
        while !pos.is_at_start(&seq) {
            eprintln!(
                "p={:?} s={} e={}",
                pos.position,
                pos.is_at_start(&seq),
                pos.is_at_end(&seq)
            );
            pos.advance(&seq, -1.0);
        }
    }
}
