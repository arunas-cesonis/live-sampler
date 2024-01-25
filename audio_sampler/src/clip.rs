use crate::sampler::LoopMode;
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::fs::read;

pub trait F:
    std::ops::Rem<Output = Self>
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + PartialOrd
    + std::ops::Neg<Output = Self>
    + num_traits::Zero
    + num_traits::Signed
    + std::iter::Sum
    + Default
    + Copy
{
}

#[derive(Default, Copy, Clone, Debug)]
pub struct Interval<T> {
    start: T,
    end: T,
}

impl<T> Interval<T>
where
    T: std::ops::Rem<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + num_traits::Signed
        + std::iter::Sum
        + Default
        + Copy,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start < end);
        Self { start, end }
    }

    pub fn duration(&self) -> T {
        self.end - self.start
    }

    pub fn contains(&self, offset: T) -> bool {
        self.start <= offset && offset < self.end
    }

    pub fn at_the_end(&self, offset: T) -> bool {
        offset == self.end
    }

    pub fn distance(&self, offset: T) -> T {
        let start = (self.start - offset).abs();
        let end = (self.end - offset).abs();
        if start > end {
            start
        } else {
            end
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct Intervals<T> {
    intervals: SmallVec<[Interval<T>; 4]>,
}

impl<T> Intervals<T>
where
    T: std::ops::Rem<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + num_traits::Signed
        + std::iter::Sum
        + Default
        + Debug
        + Copy,
{
    pub fn duration(&self) -> T {
        self.intervals.iter().map(|v| v.duration()).sum()
    }

    pub fn position_from_start(&self, offset: T) -> Option<Position<T>> {
        let mut pos = Position::start(self);
        pos.advance(self, offset);
        Some(pos)
    }

    pub fn push(&mut self, start: T, end: T) {
        self.intervals.push(Interval::new(start, end));
    }

    pub fn to_vec(&self) -> Vec<(T, T)> {
        self.intervals.iter().map(|x| (x.start, x.end)).collect()
    }

    pub fn next_index(&self, index: usize) -> usize {
        assert!(!self.intervals.is_empty());
        if index + 1 < self.intervals.len() {
            index + 1
        } else {
            0
        }
    }

    pub fn prev_index(&self, index: usize) -> usize {
        assert!(!self.intervals.is_empty());
        if index > 0 {
            index - 1
        } else {
            self.intervals.len() - 1
        }
    }

    pub fn nearest_index(&self, offset: T) -> Option<usize> {
        self.intervals
            .iter()
            .enumerate()
            .min_by(|(i, x), (j, y)| {
                let a = x.distance(offset);
                let b = y.distance(offset);
                if a < b {
                    Ordering::Less
                } else if a > b {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .map(|x| x.0)
    }

    pub fn first_index_contains(&self, offset: T) -> Option<usize> {
        self.intervals.iter().position(|x| x.contains(offset))
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Position<T> {
    index: usize,
    offset: T,
}

impl<T> Position<T>
where
    T: std::ops::Rem<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + num_traits::Signed
        + std::iter::Sum
        + Default
        + std::fmt::Debug
        + Copy,
{
    pub fn start(v: &Intervals<T>) -> Self {
        Self {
            index: 0,
            offset: v.intervals[0].start,
        }
    }

    pub fn ensure_within(&mut self, v: &Intervals<T>) {
        let (index, offset) =
            if self.index < v.intervals.len() && v.intervals[self.index].contains(self.offset) {
                (self.index, self.offset)
            } else {
                if let Some(index) = v.first_index_contains(self.offset) {
                    (index, self.offset)
                } else {
                    let index = v.nearest_index(self.offset);
                    (index, v.intervals[index].start)
                }
            };
        self.index = index;
        self.offset = offset;
    }

    pub fn advance(&mut self, v: &Intervals<T>, mut amount: T) {
        self.ensure_within(v);
        let mut index = self.index;
        let mut offset = self.offset;

        while amount < T::zero() {
            assert!(v.intervals[index].contains(offset) || v.intervals[index].at_the_end(offset));
            if offset == v.intervals[index].start {
                index = v.prev_index(index);
                offset = v.intervals[index].end;
                continue;
            };
            let remaining = offset - v.intervals[index].start;
            if remaining > amount.abs() {
                offset = offset + amount;
                amount = T::zero();
            } else if remaining < amount.abs() {
                index = v.prev_index(index);
                offset = v.intervals[index].end;
                amount = amount + remaining;
            } else {
                offset = v.intervals[index].start;
                amount = T::zero();
            }
        }

        while amount > T::zero() {
            assert!(v.intervals[index].contains(offset));
            let remaining = v.intervals[index].end - offset;
            if remaining <= amount {
                index = v.next_index(index);
                offset = v.intervals[index].start;
                amount = amount - remaining;
            } else {
                offset = offset + amount;
                amount = T::zero();
            }
        }
        self.index = index;
        self.offset = offset;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_position() {
        let mut v = Intervals::<f32>::default();
        let pos = v.position_from_start(10.0);
        eprintln!("{:?}", pos);
    }
}
