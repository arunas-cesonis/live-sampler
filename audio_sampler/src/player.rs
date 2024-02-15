use crate::utils::normalize_offset;
use nih_plug::nih_warn;

type T = f32;
// To how many points should the numbers be rounded to
// for equality comparison in asserts
const ASSERTS_PRECISION: usize = 2;

fn saw(t: T) -> T {
    t - t.floor()
}

fn tri_sec(t: T) -> bool {
    let z = t.abs() % 2.0;
    if z < 1.0 {
        false
    } else {
        true
    }
}

fn tri(t: T) -> T {
    let z = t.abs() % 2.0;
    if z < 1.0 {
        z
    } else {
        2.0 - z
    }
}

#[allow(unused)]
fn saw2(a: T, t: T) -> T {
    let z = t % a;
    if z >= 0.0 {
        z
    } else {
        z + a
    }
}

#[allow(unused)]
fn tri2(a: T, t: T) -> T {
    let aa = 2.0 * a;
    let z = t.abs() % aa;
    let r = if z < a { z } else { aa - z };
    // eprintln!("tri2 a={:?} t={:?} aa={:?} z={:?} r={:?}", a, t, aa, z, r);
    r
}

#[allow(unused)]
fn tri2_sec(a: T, t: T) -> bool {
    let aa = 2.0 * a;
    let z = t.abs() % aa;
    if z < a {
        false
    } else {
        true
    }
}

fn roundn(x: T, n: u32) -> T {
    let m = 10_i64.pow(n) as T;
    (x * m).round() / m
}

fn round_precision(x: T) -> T {
    roundn(x, ASSERTS_PRECISION as u32)
}

fn same_precision(a: T, b: T) -> bool {
    round_precision(a) == round_precision(b)
}

fn same_n(a: T, b: T, n: u32) -> bool {
    roundn(a, n) == roundn(b, n)
}

#[allow(unused)]
fn same_eq(a: T, b: T) -> bool {
    a == b
}

fn index(a: T, n: usize) -> usize {
    (round_precision(a) as usize) % n
}

#[derive(Debug, Clone)]
struct Saw {
    s: T,
    l: T,
    shift: T,
}

impl Saw {
    pub fn new(s: T, l: T) -> Self {
        Self { s, l, shift: 0.0 }
    }
    pub fn new_from_y(s: T, l: T, y: T) -> Self {
        let mut a = Self::new(s, l);
        a.shift_to(y);
        a
    }
    pub fn to_shifted(&self, x: T) -> Self {
        let mut other = self.clone();
        other.shift_to(self.apply(x));
        other
    }
    pub fn shift_to(&mut self, y: T) {
        let amount = y / self.l;
        self.shift = amount;

        let assert_y = self.apply(0.0);
        let n = self.l as usize;
        assert!(
            index(self.apply(0.0), n) == index(y, n),
            "assert_y={:?} y={:?} self={:?}",
            assert_y,
            y,
            self
        );
    }
    pub fn apply(&self, t: T) -> T {
        let y = saw2(self.l, self.s * t + self.shift * self.l);
        y
    }
}

#[derive(Debug, Clone)]
struct Triangle {
    s: T,
    l: T,
    shift: T,
}

impl Triangle {
    pub fn new(s: T, l: T) -> Self {
        Self { s, l, shift: 0.0 }
    }
    pub fn new_from_y(s: T, l: T, y: T) -> Self {
        let mut a = Self::new(s, l);
        a.shift_to(y, false, 1.0);
        a
    }
    pub fn to_shifted(&self, x: T) -> Self {
        let mut other = self.clone();
        other.shift_to(self.apply(x), self.is_sec(x), self.s.signum());
        other
    }
    pub fn shift_to(&mut self, y: T, sec: bool, signum: T) {
        let amount = y / self.l;
        self.shift = signum * (if sec { 2.0 - amount } else { amount });

        //let assert_y = self.apply(0.0);
        //assert!(
        //    same_precision(self.apply(0.0), y),
        //    "assert_y={:?} y={:?}",
        //    assert_y,
        //    y
        //);
    }
    pub fn is_sec(&self, t: T) -> bool {
        let y = tri2_sec(self.l, self.s * t + self.shift * self.l);
        y
    }
    pub fn apply(&self, t: T) -> T {
        let y = tri2(self.l, self.s * t + self.shift * self.l);
        y
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub fn run_saw(s: T, l: T) -> usize {
        let mut v: Vec<(Saw, T)> = vec![];
        let mut iter = 0;

        for i in 0..(3 * (l as usize)) {
            let x = i as T;
            let a = Saw::new(s, l);
            let y = a.apply(x);
            for (q, dx) in &v {
                let y2 = q.apply(x - dx);

                let i1 = index(y, l as usize);
                let i2 = index(y2, l as usize);
                assert_eq!(
                    i1,
                    i2,
                    "a {:?} q {:?} x {:?} dx {:?} y {:?} y2 {:?} y2-l={:?}",
                    a,
                    q,
                    x,
                    dx,
                    y,
                    y2,
                    same_n(l, y2, 4)
                );
                iter += 1;
            }
            v.push((a.to_shifted(x), x));
        }
        iter
    }

    pub fn run_triangle(s: T, l: T) -> usize {
        let mut v: Vec<(Triangle, T)> = vec![];
        let mut iter = 0;

        for i in 0..(3 * (l as usize)) {
            let x = i as T;
            let a = Triangle::new(s, l);
            let y = a.apply(x);
            for (q, dx) in &v {
                let y2 = q.apply(x - dx);
                let i1 = index(y, l as usize);
                let i2 = index(y2, l as usize);
                assert_eq!(round_precision(y), round_precision(y2));
                assert_eq!(
                    i1,
                    i2,
                    "a {:?} q {:?} x {:?} dx {:?} y {:?} y2 {:?} y2-l={:?}",
                    a,
                    q,
                    x,
                    dx,
                    y,
                    y2,
                    same_n(l, y2, 4)
                );
                iter += 1;
            }
            v.push((a.to_shifted(x), x));
        }
        iter
    }

    fn variants() -> Vec<(T, T)> {
        let speeds = vec![0.25, 0.5, 1.0, 2.0, 3.0, 40.0, 50.0];
        let speeds = vec![
            speeds.clone(),
            speeds.iter().map(|s| -*s).collect::<Vec<_>>(),
        ]
        .concat();
        let lengths = vec![1.0, 2.0, 3.0, 40.0, 50.0];
        let variants = speeds.iter().flat_map(|s| lengths.iter().map(|l| (*s, *l)));
        variants.collect()
    }

    #[test]
    pub fn test2() {
        let iterations = variants()
            .into_iter()
            .map(|(s, l)| run_triangle(s, l) + run_saw(s, l))
            .sum::<usize>();
        eprintln!("{}", iterations);
    }

    #[test]
    pub fn test() {
        let mut iter = 0;
        for (s, l) in variants() {
            let a = Triangle::new(s, l);
            let mut b = Triangle::new(s, l);
            let c = Saw::new(s, l);
            let mut d = Saw::new(s, l);
            for i in 0..(3 * (l as usize)) {
                let x = i as T;
                let y = a.apply(x);
                let sec = a.is_sec(x);
                b.shift_to(y, sec, a.s.signum());
                assert!(same_n(y, b.apply(0.0), 4));
                let y = c.apply(x);
                d.shift_to(y);
                assert!(same_n(y, d.apply(0.0), 4));
                iter += 1;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Loop,
    PingPong,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoopOffset(T);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataOffset(T);

#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub speed: T,
    pub start: T,
    pub length: T,
    pub data_len: usize,
    pub mode: Mode,
}

impl Clip {
    pub fn verify_loop_offset(&self, x: T) -> Option<LoopOffset> {
        if (0.0 <= x && x < self.length) {
            Some(LoopOffset(x))
        } else {
            None
        }
    }

    pub fn verify_data_offset(&self, x: T) -> Option<DataOffset> {
        if (0.0 <= x && x < self.data_len as T) {
            Some(DataOffset(x))
        } else {
            None
        }
    }

    pub fn loop_offset_to_data_offset(&self, offset: LoopOffset) -> DataOffset {
        assert!(self.verify_loop_offset(offset.0).is_some());
        DataOffset(normalize_offset(self.start + offset.0, self.data_len as T))
    }

    pub fn data_offset_to_loop_offset(&self, offset: DataOffset) -> Option<LoopOffset> {
        assert!(self.verify_data_offset(offset.0).is_some());
        let len_t = self.data_len as T;
        let s = self.start;
        let e = self.start + self.length;
        let x = offset.0;
        match () {
            _ if x >= s && x < e => Some(LoopOffset(x - s)),
            _ if x < s && (x + len_t < e) => Some(LoopOffset(x + len_t - s)),
            _ => None,
        }
    }

    // LoopOffset
    pub fn offset_to_data_index(&self, offset: T) -> usize {
        let x = offset % self.length;
        let x = if x >= 0.0 { x } else { x + self.length };
        let x = self.start + x;
        let index = (x.round() as usize) % self.data_len;
        index
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    saw: Saw,
    tri: Triangle,
    updated_at: usize,
    clip: Clip,
}

impl Player {
    pub fn new(now: usize, clip: &Clip) -> Self {
        Self {
            saw: Saw::new(clip.speed, clip.length),
            tri: Triangle::new(clip.speed, clip.length),
            updated_at: now,
            clip: clip.clone(),
        }
    }

    pub fn clip(&self) -> &Clip {
        &self.clip
    }

    pub fn elapsed(&self, now: usize) -> usize {
        now - self.updated_at
    }

    pub fn offset(&self, now: usize) -> T {
        let offset = match self.clip.mode {
            Mode::Loop => self.saw.apply(self.elapsed(now) as T),
            Mode::PingPong => self.tri.apply(self.elapsed(now) as T),
        };
        assert!(offset >= 0.0);
        assert!(offset <= self.clip.length, "now={} player={:?}", now, self);
        offset.min(self.clip.length - 1.0)
    }

    fn speed(&self) -> T {
        match self.clip.mode {
            Mode::Loop => self.saw.s,
            Mode::PingPong => self.tri.s,
        }
    }

    fn is_sec(&self, now: usize) -> bool {
        self.tri.is_sec(self.elapsed(now) as T)
    }

    fn calc_index(&self, now: usize) -> usize {
        let mut offset = self.offset(now);
        match self.clip.mode {
            Mode::Loop if self.speed() < 0.0 => {
                offset -= 1.0;
            }
            Mode::PingPong if self.speed() < 0.0 => {
                // FIXME: figure out the PingPong case when playing backwards
                /*
                let prev = offset;
                if offset >= 0.0 && offset - 1.0 < 0.0 {
                    offset = -(offset - 1.0);
                } else {
                    offset -= 1.0;
                }
                 */
            }
            Mode::Loop => (),
            Mode::PingPong => (),
        };
        // eprintln!("calc_index offset={:?}", offset);
        self.clip.offset_to_data_index(offset)
    }

    fn calc_available_offset(&self, now: usize, clip: &Clip) -> T {
        let y = self.offset(now);
        if let Some(tmp) = self.clip().verify_loop_offset(y) {
            let current_data_offset = self.clip.loop_offset_to_data_offset(tmp);
            let available_loop_offset = clip.data_offset_to_loop_offset(current_data_offset);
            let y = if let Some(available) = available_loop_offset {
                available.0
            } else {
                0.0
            };
            y
        } else {
            0.0
        }
    }

    pub fn sample_index(&mut self, now: usize, clip: &Clip) -> usize {
        if &self.clip == clip {
            self.calc_index(now)
        } else {
            let y = self.calc_available_offset(now, clip);
            let mut tri = Triangle::new(clip.speed, clip.length);
            let mut saw = Saw::new(clip.speed, clip.length);
            match self.clip.mode {
                Mode::Loop => {
                    tri.shift_to(y, false, 1.0);
                    saw.shift_to(y);
                }
                Mode::PingPong => {
                    tri.shift_to(y, self.is_sec(now), self.clip.speed.signum());
                    saw.shift_to(y);
                }
            }
            let mut new_self = Player {
                saw,
                tri,
                updated_at: now,
                clip: clip.clone(),
            };
            let index = new_self.calc_index(now);

            //assert_eq!(index, self.calc_index(now));
            *self = new_self;
            index
        }
    }
}

#[cfg(test)]
mod test2 {
    use super::*;
    #[test]
    pub fn test_clip() {
        let clip = Clip {
            speed: 0.0,
            start: 190.0,
            length: 50.0,
            data_len: 200,
            mode: Mode::Loop,
        };
        let x = clip.verify_loop_offset(40.0);
        eprintln!("{:?}", x);

        eprintln!("{:?}", clip.loop_offset_to_data_offset(x.unwrap()));
        eprintln!(
            "{:?}",
            clip.data_offset_to_loop_offset(clip.loop_offset_to_data_offset(x.unwrap()))
        );
    }
}
