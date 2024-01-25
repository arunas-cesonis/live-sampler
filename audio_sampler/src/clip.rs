use crate::sampler::LoopMode;
use smallvec::SmallVec;
use std::cmp::Ordering;

#[derive(Default, Debug, Clone, Copy)]
pub struct Position {
    index: usize,
    offset: f32,
    direction: f32,
}

impl Position {
    pub fn start(v: &[Interval]) -> Self {
        Position {
            index: 0,
            offset: v[0].start,
            direction: 1.0,
        }
    }

    fn get_valid_index_offset(&self, v: &[Interval]) -> (usize, f32) {
        if self.index < v.len() && v[self.index].contains(self.offset) {
            (self.index, self.offset)
        } else {
            if let Some(index) = v.iter().position(|x| x.contains(self.offset)) {
                (index, self.offset)
            } else {
                (0, v[0].start)
            }
        }
    }

    pub fn make_valid(&mut self, v: &[Interval]) {
        let (mut index, mut offset) = self.get_valid_index_offset(v);
        self.index = index;
        self.offset = offset;
    }

    pub fn to_data_index(
        &self,
        v: &[Interval],
        speed: f32,
        data_len: usize,
        loop_mode: LoopMode,
    ) -> usize {
        let offset = if self.direction * speed >= 0.0 {
            self.offset
        } else {
            let mut tmp = self.clone();
            tmp.advance(&v, -1.0, loop_mode);
            tmp.offset
        };
        (offset.round() as usize) % data_len
    }

    fn step(&mut self, v: &[Interval], mut amount: f32, loop_mode: LoopMode) -> f32 {
        if amount < 0.0 {
            assert!(v[self.index].contains(self.offset) || v[self.index].at_the_end(self.offset));
            let rem = self.offset - v[self.index].start;
            if rem == 0.0 {
                self.index = (self.index + v.len() - 1) % v.len();
                self.offset = v[self.index].end;
                return amount;
            }
            if rem > -amount {
                self.offset += amount;
                amount = 0.0;
            } else {
                amount += rem;
                self.index = (self.index + v.len() - 1) % v.len();
                self.offset = v[self.index].end;
            }
        } else if amount > 0.0 {
            assert!(v[self.index].contains(self.offset));
            let rem = v[self.index].end - self.offset;
            if rem > amount {
                self.offset += amount;
                amount = 0.0;
            } else {
                amount -= rem;
                self.index = (self.index + 1) % v.len();
                self.offset = v[self.index].start;
            }
        }
        amount
    }

    pub fn advance(&mut self, v: &[Interval], mut amount: f32, loop_mode: LoopMode) {
        let (mut index, mut offset) = self.get_valid_index_offset(v);
        self.index = index;
        self.offset = offset;
        loop {
            amount = self.step(v, amount, loop_mode);
            if !(amount.abs() > 0.0) {
                break;
            }
        }
        if v[self.index].at_the_end(self.offset) {
            self.index = (self.index + 1) % v.len();
            self.offset = v[self.index].start;
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Interval {
    start: f32,
    end: f32,
}

impl Interval {
    pub fn contains(&self, offset: f32) -> bool {
        self.start <= offset && offset < self.end
    }

    pub fn at_the_end(&self, offset: f32) -> bool {
        offset == self.end
    }

    pub fn duration(&self) -> f32 {
        self.end - self.start
    }
}

#[derive(Default, Debug, Clone)]
pub struct Intervals {
    v: SmallVec<[Interval; 4]>,
}

impl Intervals {
    pub fn duration(&self) -> f32 {
        self.v.iter().map(|x| x.duration()).sum()
    }

    pub fn push(&mut self, start: f32, end: f32) {
        assert!(start < end);
        self.v.push(Interval { start, end })
    }

    pub fn start(&self) -> f32 {
        self.v[0].start
    }

    pub fn contains(&self, offset: f32) -> bool {
        self.v.iter().any(|x| x.contains(offset))
    }

    pub fn wrapped_global(&self, x: f32) -> Option<f32> {
        let d = self.duration();
        let x = x % d;
        let x = if x < 0.0 { x + d } else { x };

        self.global(x)
    }

    pub fn first_local(&self, x: f32) -> Option<f32> {
        assert!(x >= 0.0);
        let mut offset = 0.0;
        for interval in &self.v {
            let s = interval.start;
            let e = interval.end;
            let d = e - s;
            if s <= x && x < e {
                return Some(offset + (x - s));
            }
            offset += d;
        }
        None
    }

    pub fn local(&self, x: f32) -> Vec<f32> {
        assert!(x >= 0.0);
        let mut offset = 0.0;
        let mut result = vec![];
        for interval in &self.v {
            let s = interval.start;
            let e = interval.end;
            let d = e - s;
            if s <= x && x < e {
                result.push(offset + (x - s));
            }
            offset += d;
        }
        result
    }

    pub fn global(&self, x: f32) -> Option<f32> {
        assert!(x >= 0.0);
        let mut offset = 0.0;
        for interval in &self.v {
            let s = interval.start;
            let e = interval.end;
            let d = e - s;
            if offset <= x && x < offset + d {
                return Some(s + (x - offset));
            }
            offset += d;
        }
        None
    }

    // questionable
    pub fn nearest_global(&self, x: f32) -> Option<f32> {
        let (mut min, mut min_d) = match self.v.first() {
            Some(interval) => (interval.start, (x - interval.start).abs()),
            None => return None,
        };
        for i in 0..self.v.len() {
            let s = self.v[i].start;
            let e = self.v[i].end;
            let ds = (x - s).abs();
            let de = (x - e).abs();
            if ds < min_d {
                min_d = ds;
                min = s;
            }
            if de < min_d {
                min_d = de;
                min = self.v[(i + 1) % self.v.len()].start;
            }
        }
        Some(min)
    }

    pub fn as_slice(&self) -> &[Interval] {
        self.v.as_slice()
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_intervals() {
        let mut view = Intervals::default();
        view.push(10.0, 20.0);
        let mut pos = Position::start(view.as_slice());

        //view.push(100.0, 190.0);
        pos.advance(view.as_slice(), 5.0, LoopMode::Loop);
        for _ in 0..100 {
            pos.advance(view.as_slice(), -1.3, LoopMode::Loop);
            eprintln!("{:?}", pos);
        }
    }
}
