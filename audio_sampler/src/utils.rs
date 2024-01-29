use crate::sampler::LoopMode;
use nih_plug::nih_warn;
use nih_plug_vizia::vizia::views::combo_box_derived_lenses::p;
use num_traits::real::Real;
use std::io::stdout;

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

#[inline]
pub fn normalize_offset<T>(offset: T, n: T) -> T
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
