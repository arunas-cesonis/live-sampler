use crate::common_types::LoopMode;
use crate::utils::normalize_offset;
use enum_dispatch::enum_dispatch;
use num_traits::Float;

#[enum_dispatch]
#[derive(Debug, Clone)]
pub enum PhaseEnum {
    Saw,
    Tri,
}

#[enum_dispatch(PhaseEnum)]
trait Phase {
    fn calc(&self, x: f64) -> f64;
    fn update_speed(&self, x: f64, s: f64) -> PhaseEnum;
    fn update_length(&self, x: f64, l: f64) -> PhaseEnum;
    fn to_tri(&self, x: f64) -> PhaseEnum;
    fn to_saw(&self, x: f64) -> PhaseEnum;
}

pub fn saw(speed: f64, length: f64) -> PhaseEnum {
    PhaseEnum::Saw(Saw::new(speed, length))
}

pub fn tri(speed: f64, length: f64) -> PhaseEnum {
    PhaseEnum::Tri(Tri::new(speed, length))
}

#[derive(Debug, Clone)]
pub struct Tri {
    speed: f64,
    length: f64,
    shift: f64,
}

impl Tri {
    pub fn new(speed: f64, length: f64) -> Self {
        Self::new_shifted(speed, length, 0.0)
    }
    pub fn new_shifted(speed: f64, length: f64, shift: f64) -> Self {
        Self {
            speed,
            length,
            shift,
        }
    }
}

fn align_saw(speed: f64, length: f64, x: f64, y: f64) -> PhaseEnum {
    let shift = y - x * speed;
    PhaseEnum::Saw(Saw::new_shifted(speed, length, shift))
}

fn align_tri(speed: f64, length: f64, x: f64, y: f64, sec: bool) -> PhaseEnum {
    let shift = if !sec {
        y / speed - x
    } else {
        x * length / speed - y / speed - x
    };
    PhaseEnum::Tri(Tri::new_shifted(speed, length, shift))
}

#[inline]
fn wrap(x: f64, n: f64) -> f64 {
    let tmp = x % n;
    let r = if tmp >= 0.0 { tmp } else { n + tmp };
    debug_assert!(r >= 0.0 && r < n, "r={} x={} n={}", r, x, n);
    r.clamp(0.0, n - f64::epsilon())
}

fn mirror_usize(i: usize, l: usize) -> usize {
    let iml = i % l;
    let idl = (i / l) & 1;
    ((l - iml) - 1) * idl + iml * (1 - idl)
}

#[inline]
fn mirror(x: f64, n: f64) -> f64 {
    let nn = 2.0 * n;
    let tmp = x.abs() % nn;
    let r = if (tmp < n) { tmp } else { nn - tmp - 1.0 };
    // clamping solves edge cases with fractional speeds, e.g. 0.5
    // slightly overshooting 0.0 and n
    let r = r.clamp(0.0, n - 1.0);
    ///debug_assert!(r >= 0.0 && r < n, "r={} x={} n={}", r, x, n);
    r
    //r.clamp(0.0, n - f64::epsilon())
}

impl Phase for Tri {
    fn calc(&self, x: f64) -> f64 {
        // the difference between saw and triangle in incorporating shift into argument
        // is necessary but not intentional, i.e would be interesting to fix
        mirror((x + self.shift) * self.speed, self.length)
    }
    fn update_speed(&self, x: f64, s: f64) -> PhaseEnum {
        let y = self.calc(x);
        let sec = (x * self.speed) % (2.0 * self.length) >= self.length;
        align_tri(s, self.length, x, y, sec)
    }
    fn update_length(&self, x: f64, l: f64) -> PhaseEnum {
        let y = self.calc(x);
        let sec = (x * self.speed) % (2.0 * self.length) >= self.length;
        let (y1, sec1) = if (y >= l) { (l, true) } else { (y, sec) };
        align_tri(self.speed, l, x, y1, sec1)
    }
    fn to_tri(&self, x: f64) -> PhaseEnum {
        PhaseEnum::Tri(self.clone())
    }
    fn to_saw(&self, x: f64) -> PhaseEnum {
        let y = self.calc(x);
        align_saw(self.speed, self.length, x, y)
    }
}

#[derive(Debug, Clone)]
pub struct Saw {
    speed: f64,
    length: f64,
    shift: f64,
}

impl Saw {
    pub fn new_shifted(speed: f64, length: f64, shift: f64) -> Self {
        Self {
            speed,
            length,
            shift,
        }
    }
    pub fn new(speed: f64, length: f64) -> Self {
        Self::new_shifted(speed, length, 0.0)
    }
}

impl Phase for Saw {
    fn calc(&self, x: f64) -> f64 {
        wrap(x * self.speed + self.shift, self.length)
    }
    fn update_speed(&self, x: f64, s: f64) -> PhaseEnum {
        let y = self.calc(x);
        align_saw(s, self.length, x, y)
    }
    fn update_length(&self, x: f64, l: f64) -> PhaseEnum {
        let y = self.calc(x);
        let y1 = if y >= l { 0.0 } else { y };
        align_saw(self.speed, l, x, y1)
    }
    fn to_tri(&self, x: f64) -> PhaseEnum {
        let y = self.calc(x);
        align_tri(self.speed, self.length, x, y, self.speed < 0.0)
    }
    fn to_saw(&self, x: f64) -> PhaseEnum {
        PhaseEnum::Saw(self.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_essential() {
        let lengths = vec![1.0, 2.0, 10.0, 100.0, 44100.0];
        let l = 101;
        let p1 = saw(1.0, l as f64);
        let p2 = tri(1.0, l as f64);
        for i in 0..(5 * l) {
            let x = i as f64;
            let index1 = p1.calc(x).floor() as usize;
            let index2 = p2.calc(x).floor() as usize;
            assert_eq!(index1, i % l);
            assert_eq!(index2, mirror_usize(i, l));
        }
    }

    #[test]
    fn test_phase_enum() {
        let lengths = vec![1.0, 2.0, 10.0, 100.0];
        let speeds = vec![-2.0, -1.0, -0.5, -0.25, 0.25, 0.5, 1.0, 3.0, 10.0];
        let variants = speeds
            .iter()
            .flat_map(|s| lengths.iter().map(|l| (*s, *l)))
            .collect::<Vec<_>>();

        for (s, l) in variants {
            let p = saw(s, l);
        }
    }
}
