#[inline]
pub fn normalize_offset(offset: f32, n: f32) -> f32
{
    let x = offset % n;
    let x = if x >= 0.0 { x } else { x + n };
    // to avoid -0.0
    // x.abs()
    x
}

