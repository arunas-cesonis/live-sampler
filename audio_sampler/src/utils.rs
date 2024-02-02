use crate::sampler::LoopMode;
use nih_plug::nih_warn;
use nih_plug_vizia::vizia::views::combo_box_derived_lenses::p;
use num_traits::real::Real;
use num_traits::Float;
use std::io::stdout;

pub fn ping_pong3<T>(x: T, n: T, step: T) -> (T, T)
where
    T: std::ops::Rem<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + Real
        + Copy,
{
    if x >= T::zero() && x < n {
        (x, T::one())
    } else {
        let y = normalize_offset(x, n + n);
        if y < n {
            (y, T::one())
        } else {
            (n + n - y - step, -T::one())
        }
    }
}

pub fn ping_pong2<T>(x: T, n: T) -> (T, T)
where
    T: std::ops::Rem<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + Real
        + Copy,
{
    if x >= T::zero() && x < n {
        (x, T::one())
    } else {
        let y = normalize_offset(x, n + n);
        if y < n {
            (y, T::one())
        } else {
            (n + n - y - T::one(), -T::one())
        }
    }
}

#[cfg(test)]
mod test_ping_pong2 {
    use crate::utils::ping_pong2;

    #[test]
    fn test_ping_pong2() {
        for i in -30..30 {
            let i = i as f32;
            eprintln!("({}, {}) = {:?}", i, 10.0, ping_pong2(i, 10.0));
        }
        assert_eq!(ping_pong2(0.0, 5.0), (0.0, 1.0));
        assert_eq!(ping_pong2(6.0, 5.0), (3.0, -1.0));
    }
}

#[inline]
pub fn normalize_offset<T>(offset: T, n: T) -> T
where
    T: std::ops::Rem<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Real
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + Copy,
{
    let x = offset % n;
    let x = if x < T::zero() { x + n } else { x };
    // to avoid -0.0
    x.abs()
}

#[cfg(test)]
mod test_loop {
    use super::*;
    #[test]
    fn test_bound() {
        let data_len = Bound::new(100.0);
        let start = Offset::new(90.0);
        let length = Bound::new(20.0);
        let multiply = Multiply::new(0.5);
        for i in 0..100 {
            let i = i as f32;
            let x = data_len.apply(start.apply(length.apply(multiply.apply(i))));
            eprintln!("i={} x={}", i, x);
        }
    }
}

pub struct Bound {
    bound: f32,
}

impl Bound {
    pub fn new(bound: f32) -> Self {
        Self { bound }
    }

    pub fn apply(&self, x: f32) -> f32 {
        normalize_offset(x, self.bound)
    }
}

pub struct Multiply {
    value: f32,
}

impl Multiply {
    pub fn new(value: f32) -> Self {
        Self { value }
    }

    pub fn apply(&self, x: f32) -> f32 {
        x * self.value
    }
}

pub struct Offset {
    amount: f32,
}

impl Offset {
    pub fn new(amount: f32) -> Self {
        Self { amount }
    }

    pub fn apply(&self, x: f32) -> f32 {
        x + self.amount
    }
}

// https://github.com/robbert-vdh/nih-plug/blob/92ce73700005255565c6be45412609ea87eb8b41/src/util.rs#L38
pub const MINUS_INFINITY_GAIN: f32 = 1e-5; // 10f32.powf(MINUS_INFINITY_DB / 20)

/// Convert a voltage gain ratio to decibels. Gain ratios that aren't positive will be treated as
///
/// [`MINUS_INFINITY_DB`].
#[inline]
pub fn gain_to_db(gain: f32) -> f32 {
    f32::max(gain, MINUS_INFINITY_GAIN).log10() * 20.0
}
