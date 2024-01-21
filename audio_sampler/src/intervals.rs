use smallvec::SmallVec;
use std::fmt::Display;
use std::ops::{Add, AddAssign, Rem, Sub};

#[derive(Debug, Clone)]
struct Interval<T> {
    start: T,
    end: T,
}

#[derive(Debug, Clone, Default)]
pub struct Intervals<T> {
    intervals: SmallVec<[Interval<T>; 4]>,
}

pub fn g_wrap_to_positive_offset<T>(x: T, data_len: T) -> T
where
    T: Rem<Output = T> + num_traits::Zero + Add<Output = T> + PartialOrd + Copy,
{
    let x = x % data_len;
    if x < T::zero() {
        x + data_len
    } else {
        x
    }
}

pub fn wrap_to_positive_offset(x: f32, data_len: f32) -> f32 {
    let x = x % data_len;
    if x < 0.0 {
        x + data_len
    } else {
        x
    }
}

impl<T> Intervals<T>
where
    T: Rem<Output = T>
        + Add<Output = T>
        + AddAssign
        + Sub<Output = T>
        + PartialOrd
        + std::iter::Sum
        + num_traits::Zero
        + num_traits::Signed
        + Copy
        + Display,
{
    pub fn push(&mut self, start: T, end: T) {
        // assert!(start < end, "start = {}, end = {}", start, end);
        assert!(T::zero() <= start, "start = {}", start);
        assert!(T::zero() <= end, "end = {}", end);
        self.intervals.push(Interval { start, end });
    }

    pub fn duration(&self) -> T {
        self.intervals.iter().map(|x| (x.end - x.start).abs()).sum()
    }

    pub fn project(&self, x: T) -> T {
        let x = g_wrap_to_positive_offset(x, self.duration());
        let mut offset = T::zero();
        for interval in &self.intervals {
            let s = interval.start;
            let e = interval.end;
            let d = (e - s).abs();
            // FIXME: the code below is very susceptible to floating point errors.
            // maybe using i64 for offsets would be fine, e.g. with 1000 or so subdivisions per sample
            let xd = x - offset;
            if xd >= T::zero() && xd < d {
                if s < e {
                    return s + xd;
                } else {
                    return s - xd;
                }
            }
            offset += d;
        }
        panic!("no intervals contain x")
    }

    pub fn unproject(&self, x: T, data_len: T) -> Vec<T> {
        let x = g_wrap_to_positive_offset(x, data_len);
        let mut result = vec![];
        let mut offset = T::zero();
        for interval in &self.intervals {
            let s = interval.start;
            let e = interval.end;
            let d = (e - s).abs();
            if s <= e && x >= s && x < e {
                result.push(x - s + offset);
            } else if s > e && x <= s && x > e {
                result.push(s - x + offset);
            }
            offset += d;
        }
        result
    }
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_reverse() {
        let mut view = Intervals::<i32>::default();
        view.push(0, 10);
        view.push(20, 10);
        view.push(10, 0);
        let all = (0..view.duration())
            .map(|x| view.project(x))
            .collect::<Vec<_>>();
        assert_eq!(
            all,
            vec![
                (0..10).collect::<Vec<_>>(),
                (11..=20).rev().collect::<Vec<_>>(),
                (1..=10).rev().collect::<Vec<_>>(),
            ]
            .concat()
        );
        let all = (0..21).map(|x| view.unproject(x, 100)).collect::<Vec<_>>();

        let expected = vec![
            vec![vec![0]],
            (1..=9)
                .zip((11..=29).rev())
                .map(|(x, y)| vec![x, y])
                .collect::<Vec<_>>(),
            (10..=20).rev().map(|x| vec![x]).collect::<Vec<_>>(),
        ]
        .concat();
        assert_eq!(all, expected);
    }

    #[test]
    fn test_fp_error() {
        let mut view = Intervals::<i32>::default();
        view.push(8, 10);
        view.push(0, 3);
        let mut x: i32 = 0;
        while x.abs() < view.duration() * 2 {
            let y = view.project(x);
            let y = y % 10;
            let y = if y < 0 { y + 10 } else { y };
            eprintln!("i32 {} {}", x, y);
            x -= 1;
        }
        eprintln!("");
        let mut view = Intervals::<f32>::default();
        view.push(8.0, 10.0);
        view.push(0.0, 2.9999995);
        let mut x: f32 = 0.0;
        while x.abs() < view.duration() * 2.0 {
            let y = view.project(x);
            let y = (y.round() as i64) % 10;
            let y = if y < 0 { y + 10 } else { y };
            eprintln!("f32 {} {}", x, y);
            x -= 1.0;
        }
    }

    #[test]
    fn test_g_intervals2() {
        let mut view = Intervals::<i64>::default();
        view.push(0, 10);
        view.push(20, 30);
        view.push(40, 50);
        assert_eq!(view.unproject(5, 100), vec![5]);
        assert_eq!(view.unproject(25, 100), vec![15]);
        assert_eq!(view.unproject(21, 100), vec![11]);
        assert_eq!(view.unproject(10, 100), Vec::<i64>::new());

        assert_eq!(5, view.project(5));
        assert_eq!(20, view.project(10));
        assert_eq!(21, view.project(11));
        assert_eq!(25, view.project(15));

        let mut pos = 0;
        let mut out = vec![];
        while pos < view.duration() {
            let value = view.project(pos);
            out.push(value);
            pos += 1;
        }
        let expected: Vec<i64> = vec![
            (0..10i64).collect::<Vec<_>>(),
            (20..30i64).collect::<Vec<_>>(),
            (40..50i64).collect::<Vec<_>>(),
        ]
        .concat();
        assert_eq!(expected, out);

        let mut view = Intervals::<i64>::default();
        view.push(10, 50);

        assert_eq!(view.unproject(25, 100), vec![15]);
        assert_eq!(view.unproject(25 + 100, 100), vec![15]);
        assert_eq!(view.unproject(25 - 100, 100), vec![15]);

        let mut view = Intervals::<i64>::default();
        view.push(40, 50);
        view.push(10, 20);

        //assert_eq!(view.unproject(250, 1000), vec![150]);
        //assert_eq!(view.unproject(250 + 1000, 1000), vec![150]);
        //assert_eq!(view.unproject(250 - 1000, 1000), vec![150]);
        let mut pos = 0;
        let mut out = vec![];
        while pos < view.duration() {
            let value = view.project(pos);
            out.push(value);
            pos += 1;
        }
        let expected: Vec<_> =
            vec![(40..50).collect::<Vec<_>>(), (10..20).collect::<Vec<_>>()].concat();
        assert_eq!(expected, out);
        eprintln!("{:?}", view);
    }
}
