pub fn normalize_offset<T>(offset: T, n: T) -> T
where
    T: std::ops::Rem<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + std::ops::Neg<Output = T>
        + num_traits::Zero
        + Copy,
{
    let x = offset % n;
    let x = if x < T::zero() { x + n } else { x };
    x
}
