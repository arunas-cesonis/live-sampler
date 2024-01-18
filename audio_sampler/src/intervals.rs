use std::fmt::Display;

#[derive(Debug, Clone)]
struct Interval {
    start: f32,
    end: f32,
}

#[derive(Debug, Clone)]
struct GInterval<T> {
    start: T,
    end: T,
}

#[derive(Debug, Clone, Default)]
pub struct GIntervals2<T> {
    intervals: Vec<GInterval<T>>,
}

#[derive(Debug, Clone, Default)]
pub struct Intervals2 {
    intervals: Vec<Interval>,
}

trait Zero {
    fn zero() -> Self;
}

impl Zero for f32 {
    fn zero() -> Self {
        0.0
    }
}

impl Zero for i64 {
    fn zero() -> Self {
        0
    }
}

pub fn g_wrap_to_positive_offset<T>(x: T, data_len: T) -> T
where
    T: std::ops::Rem<Output = T> + Zero + std::ops::Add<Output = T> + std::cmp::PartialOrd + Copy,
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

impl<T> GIntervals2<T>
where
    T: std::ops::Rem<Output = T>
        + Zero
        + std::ops::Add<Output = T>
        + std::ops::AddAssign
        + std::ops::Sub<Output = T>
        + std::cmp::PartialOrd
        + std::iter::Sum
        + Copy
        + Display,
{
    pub fn push(&mut self, start: T, end: T) {
        assert!(start < end, "start = {}, end = {}", start, end);
        assert!(T::zero() <= start, "start = {}", start);
        assert!(T::zero() <= end, "end = {}", end);
        self.intervals.push(GInterval { start, end });
    }

    pub fn duration(&self) -> T {
        self.intervals.iter().map(|x| x.end - x.start).sum()
    }

    pub fn project(&self, x: T) -> Vec<T> {
        let x = g_wrap_to_positive_offset(x, self.duration());
        let mut result = vec![];
        let mut offset = T::zero();
        for interval in &self.intervals {
            let s = interval.start;
            let e = interval.end;
            let d = e - s;
            if x >= offset && x < offset + d {
                eprintln!(
                    "project duration={} x={} d={} s={} offset={} result={}",
                    self.duration(),
                    x,
                    d,
                    s,
                    offset,
                    s + x - offset
                );
                // FIXME: this code fails due to floating point errors
                result.push(s + x - offset);
            }
            offset += d;
        }
        result
    }

    pub fn unproject(&self, x: T, data_len: T) -> Vec<T> {
        let x = g_wrap_to_positive_offset(x, data_len);
        let mut result = vec![];
        let mut offset = T::zero();
        for interval in &self.intervals {
            let s = interval.start;
            let e = interval.end;
            if x >= s && x < e {
                result.push(x - s + offset);
            }
            offset += e - s;
        }
        result
    }
}

impl Intervals2 {
    pub fn push(&mut self, start: f32, end: f32) {
        assert!(start < end, "start = {}, end = {}", start, end);
        assert!(0.0 <= start, "start = {}", start);
        assert!(0.0 <= end, "end = {}", end);
        self.intervals.push(Interval { start, end });
    }

    pub fn duration(&self) -> f32 {
        self.intervals.iter().map(|x| x.end - x.start).sum()
    }

    pub fn project(&self, x: f32, data_len: f32) -> Vec<f32> {
        let x = wrap_to_positive_offset(x, self.duration());
        let mut result = vec![];
        let mut offset = 0.0;
        for interval in &self.intervals {
            let s = interval.start;
            let e = interval.end;
            let d = e - s;
            if x >= offset && x < offset + d {
                eprintln!(
                    "project duration={} x={} d={} s={} offset={} result={}",
                    self.duration(),
                    x,
                    d,
                    s,
                    offset,
                    s + x - offset
                );
                // FIXME: this code fails due to floating point errors: x is 1.9999995 and is less that offset + d = 2.0 however,
                // subtracting offset and adding to start makes it go above end
                result.push((s + x - offset) % data_len);
            }
            offset += d;
        }
        result
    }

    pub fn unproject(&self, x: f32, data_len: f32) -> Vec<f32> {
        let x = wrap_to_positive_offset(x, data_len);
        let mut result = vec![];
        let mut offset = 0.0;
        for interval in &self.intervals {
            let s = interval.start;
            let e = interval.end;
            if x >= s && x < e {
                result.push(x - s + offset);
            }
            offset += e - s;
        }
        result
    }
}

// #[derive(Debug, Clone)]
// pub struct Intervals {
//     intervals: Vec<Interval>,
//     data_len: f32,
// }
//
// impl Intervals {
//     pub fn new(data_len: f32) -> Self {
//         Self {
//             intervals: vec![],
//             data_len,
//         }
//     }
//
//     pub fn push(&mut self, start: f32, end: f32) {
//         assert!(start < end);
//         assert!(end <= self.data_len);
//         self.intervals
//             .last()
//             .iter()
//             .for_each(|x| assert!(x.end <= start));
//         self.intervals.push(Interval { start, end });
//     }
//
//     pub fn intervals_len(&self) -> f32 {
//         self.intervals.iter().map(|x| x.end - x.start).sum()
//     }
//
//     pub fn data_len(&self) -> f32 {
//         self.data_len
//     }
//
//     pub fn unproject(&self, x: f32) -> Option<f32> {
//         let x = x % self.data_len;
//         let x = if x < 0.0 { x + self.data_len } else { x };
//         let mut offset = 0.0;
//         let mut i = 0;
//         while i < self.intervals.len() {
//             let s = self.intervals[i].start;
//             let e = self.intervals[i].end;
//             if x >= s && x < e {
//                 return Some(offset + x - s);
//             }
//             if x < s {
//                 return None;
//             }
//             offset += e - s;
//             i += 1;
//         }
//         None
//     }
//
//     pub fn project(&self, x: f32) -> f32 {
//         let length = self.intervals_len();
//         let x = x % length;
//         let x = if x < 0.0 { x + length } else { x };
//         let mut offset = 0.0;
//         let mut i = 0;
//         while i < self.intervals.len() {
//             let s = self.intervals[i].start;
//             let e = self.intervals[i].end;
//             let d = e - s;
//             if x >= offset && x < offset + d {
//                 return s + (x - offset);
//             }
//             offset += d;
//             i += 1;
//         }
//         panic!(
//             "reached unreachable x= {}, offset = {}, length = {}",
//             x, offset, length
//         );
//     }
// }

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_intervals2() {
        let mut view = Intervals2::default();
        view.push(0.0, 10.0);
        view.push(20.0, 30.0);
        view.push(40.0, 50.0);
        assert_eq!(view.unproject(5.0, 100.0), vec![5.0]);
        assert_eq!(view.unproject(25.0, 100.0), vec![15.0]);
        assert_eq!(view.unproject(21.0, 100.0), vec![11.0]);
        assert_eq!(view.unproject(10.0, 100.0), Vec::<f32>::new());

        assert_eq!(vec![5.0], view.project(5.0, 100.0));
        assert_eq!(vec![20.0], view.project(10.0, 100.0));
        assert_eq!(vec![21.0], view.project(11.0, 100.0));
        assert_eq!(vec![25.0], view.project(15.0, 100.0));

        let mut pos = 0.0;
        let mut out = vec![];
        while pos < view.duration() {
            let value = view.project(pos, 100.0);
            assert!(value.len() <= 1);
            out.push(value[0]);
            pos += 1.0;
        }
        let expected: Vec<_> = vec![
            (0..10).collect::<Vec<_>>(),
            (20..30).collect::<Vec<_>>(),
            (40..50).collect::<Vec<_>>(),
        ]
        .concat()
        .into_iter()
        .map(|x| x as f32)
        .collect();
        assert_eq!(expected, out);

        let mut view = Intervals2::default();
        view.push(10.0, 50.0);

        assert_eq!(view.unproject(25.0, 100.0), vec![15.0]);
        assert_eq!(view.unproject(25.0 + 100.0, 100.0), vec![15.0]);
        assert_eq!(view.unproject(25.0 - 100.0, 100.0), vec![15.0]);

        let mut view = Intervals2::default();
        view.push(40.0, 50.0);
        view.push(10.0, 20.0);

        //assert_eq!(view.unproject(25.0, 100.0), vec![15.0]);
        //assert_eq!(view.unproject(25.0 + 100.0, 100.0), vec![15.0]);
        //assert_eq!(view.unproject(25.0 - 100.0, 100.0), vec![15.0]);
        let mut pos = 0.0;
        let mut out = vec![];
        while pos < view.duration() {
            let value = view.project(pos, 100.0);
            assert!(value.len() <= 1);
            out.push(value[0]);
            pos += 1.0;
        }
        let expected: Vec<_> = vec![(40..50).collect::<Vec<_>>(), (10..20).collect::<Vec<_>>()]
            .concat()
            .into_iter()
            .map(|x| x as f32)
            .collect();
        assert_eq!(expected, out);
        eprintln!("{:?}", view);
    }

    #[test]
    fn test_g_intervals2() {
        let mut view = GIntervals2::<i64>::default();
        view.push(0, 10);
        view.push(20, 30);
        view.push(40, 50);
        assert_eq!(view.unproject(5, 100), vec![5]);
        assert_eq!(view.unproject(25, 100), vec![15]);
        assert_eq!(view.unproject(21, 100), vec![11]);
        assert_eq!(view.unproject(10, 100), Vec::<i64>::new());

        assert_eq!(vec![5], view.project(5));
        assert_eq!(vec![20], view.project(10));
        assert_eq!(vec![21], view.project(11));
        assert_eq!(vec![25], view.project(15));

        let mut pos = 0;
        let mut out = vec![];
        while pos < view.duration() {
            let value = view.project(pos);
            assert!(value.len() <= 1);
            out.push(value[0]);
            pos += 1;
        }
        let expected: Vec<i64> = vec![
            (0..10i64).collect::<Vec<_>>(),
            (20..30i64).collect::<Vec<_>>(),
            (40..50i64).collect::<Vec<_>>(),
        ]
        .concat();
        assert_eq!(expected, out);

        let mut view = GIntervals2::<i64>::default();
        view.push(10, 50);

        assert_eq!(view.unproject(25, 100), vec![15]);
        assert_eq!(view.unproject(25 + 100, 100), vec![15]);
        assert_eq!(view.unproject(25 - 100, 100), vec![15]);

        let mut view = GIntervals2::<i64>::default();
        view.push(40, 50);
        view.push(10, 20);

        //assert_eq!(view.unproject(250, 1000), vec![150]);
        //assert_eq!(view.unproject(250 + 1000, 1000), vec![150]);
        //assert_eq!(view.unproject(250 - 1000, 1000), vec![150]);
        let mut pos = 0;
        let mut out = vec![];
        while pos < view.duration() {
            let value = view.project(pos);
            assert!(value.len() <= 1);
            out.push(value[0]);
            pos += 1;
        }
        let expected: Vec<_> =
            vec![(40..50).collect::<Vec<_>>(), (10..20).collect::<Vec<_>>()].concat();
        assert_eq!(expected, out);
        eprintln!("{:?}", view);
    }
}
