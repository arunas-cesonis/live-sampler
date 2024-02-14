use num_traits::real::Real;
use num_traits::Float;

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
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + Real
        + Copy,
{
    assert!(n >= T::zero());
    let x = offset % n;
    let x = if x >= T::zero() { x } else { x + n };
    x
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
