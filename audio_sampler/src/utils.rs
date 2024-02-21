use num_traits::real::Real;
use num_traits::Float;

#[inline]
pub fn normalize_offset<T>(offset: T, n: T) -> T
    where
        T: std::ops::Rem<Output=T>
        + std::ops::Add<Output=T>
        + std::ops::Sub<Output=T>
        + PartialOrd
        + std::ops::Neg<Output=T>
        + num_traits::Zero
        + Copy,
{
    let x = offset % n;
    let x = if x >= T::zero() { x } else { x + n };
    // to avoid -0.0
    // x.abs()
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
