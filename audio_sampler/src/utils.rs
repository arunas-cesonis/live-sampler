use nih_plug::nih_warn;

#[inline]
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

pub struct LoopConfig {
    loop_start_percent: f32,
    loop_length_percent: f32,
    data_len: usize,
}

impl LoopConfig {
    pub fn new(loop_start_percent: f32, loop_length_percent: f32, data_len: usize) -> Self {
        assert!(data_len > 0);
        Self {
            loop_start_percent,
            loop_length_percent,
            data_len,
        }
    }

    #[inline]
    pub fn loop_length(&self) -> f32 {
        self.loop_length_percent * self.data_len()
    }

    #[inline]
    pub fn loop_start(&self) -> f32 {
        self.loop_start_percent * self.data_len()
    }

    #[inline]
    pub fn data_len(&self) -> f32 {
        self.data_len as f32
    }

    #[inline]
    pub fn loop_end(&self) -> f32 {
        (self.loop_start() + self.loop_length()) % self.data_len()
    }

    pub fn loop_to_buffer(&self, offset: f32) -> Option<f32> {
        let s = self.loop_start();
        let m = self.loop_length();
        let n = self.data_len();
        if 0.0 <= offset && offset < m {
            Some(normalize_offset(s + offset, n))
        } else {
            None
        }
    }

    pub fn buffer_to_loop(&self, offset: f32) -> Option<f32> {
        let s = self.loop_start();
        let e = self.loop_end();
        let n = self.data_len();
        if s < e {
            if s <= offset && offset < e {
                Some(offset - s)
            } else {
                None
            }
        } else {
            if s <= offset && offset < n {
                Some(offset - s)
            } else if 0.0 <= offset && offset < e {
                Some(n - s + offset)
            } else {
                None
            }
        }
    }

    pub fn contains_buffer_offset(&self, offset: f32) -> bool {
        let s = self.loop_start();
        let e = self.loop_end();
        if s < e {
            s <= offset && offset < e
        } else {
            let n = self.data_len();
            (s <= offset && offset < n) || (0.0 <= offset && offset < e)
        }
    }

    pub fn translate_wrapping(&self, offset: f32, delta: f32) -> Option<f32> {
        let x = self.buffer_to_loop(offset)? + delta;
        let x = normalize_offset(x, self.loop_length());
        self.loop_to_buffer(x)
    }

    pub fn translate_reflecting(&self, offset: f32, delta: f32) -> Option<f32> {
        let l = self.loop_length();
        let x = self.buffer_to_loop(offset)?;
        let x = normalize_offset(x + delta, 2.0 * l);
        let x = if x >= l { 2.0 * l - x } else { x };
        self.loop_to_buffer(normalize_offset(x, l))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TranslateToBoundaryResult {
    ReachedStart {
        remaining_delta: f32,
        new_offset: f32,
    },
    ReachedEnd {
        remaining_delta: f32,
        new_offset: f32,
    },
    Uneventful {
        new_offset: f32,
    },
}

#[cfg(test)]
mod test {
    use crate::utils::LoopConfig;
    use std::hash::Hasher;

    struct Rng {
        seed: i32,
    }
    impl Rng {
        pub fn new(seed: i32) -> Self {
            Self { seed }
        }
        pub fn gen_u64(&mut self) -> u64 {
            let mut hasher = std::hash::DefaultHasher::new();
            hasher.write_i32(self.seed);
            let r = hasher.finish();
            self.seed = (r & 0x7fffffff) as i32;
            r
        }

        pub fn gen_f32(&mut self) -> f32 {
            let r = self.gen_u64();
            let r = r as f32;
            r / (std::u64::MAX as f32)
        }

        pub fn gen_usize(&mut self, n: usize) -> usize {
            let r = self.gen_u64();
            (r % (n as u64)) as usize
        }
    }

    #[test]
    fn test_wrap_around() {
        use super::*;
        let loop_start_percent = 0.4;
        let loop_length_percent = 0.8;
        let data_len = 100;
        let config = LoopConfig {
            loop_start_percent,
            loop_length_percent,
            data_len,
        };
        assert_eq!(Some(0.0), config.buffer_to_loop(40.0));
        assert_eq!(None, config.buffer_to_loop(-20.0));
        assert_eq!(config.loop_start(), 40.0);
        assert_eq!(config.loop_end(), 20.0);
        let r = config.translate_wrapping(40.0, config.loop_length() - 10.0);
        eprintln!("{:?}", r);
        let r = config.translate_wrapping(40.0, config.loop_length() - 10.0);
        eprintln!("{:?}", r);
    }

    #[test]
    fn test_reflect() {
        let config = LoopConfig::new(0.0, 1.0, 100);
        assert_eq!(Some(20.0), config.translate_reflecting(10.0, -30.0));
        assert_eq!(Some(80.0), config.translate_reflecting(10.0, -130.0));
        assert_eq!(Some(20.0), config.translate_reflecting(10.0, -230.0));
        assert_eq!(100.0, config.loop_length());
        assert_eq!(Some(90.0), config.translate_reflecting(50.0, 40.0));
        assert_eq!(Some(0.0), config.translate_reflecting(50.0, 50.0));
        assert_eq!(
            Some(90.0),
            config.translate_reflecting(97.0, 13.0 + 2.0 * config.loop_length())
        );
        assert_eq!(
            Some(10.0),
            config.translate_reflecting(97.0, 13.0 + 3.0 * config.loop_length())
        );
    }
    #[test]
    fn test_reflect_more() {
        let config = LoopConfig::new(0.5, 0.7, 100);
        assert_eq!(Some(51.0), config.translate_reflecting(50.0, 1.0));
        assert_eq!(Some(51.0), config.translate_wrapping(50.0, 1.0));
        assert_eq!(Some(19.0), config.translate_wrapping(50.0, -1.0));
        assert_eq!(Some(60.0), config.translate_reflecting(60.0, -20.0));
        assert_eq!(Some(00.0), config.translate_wrapping(99.0, 1.0));
    }

    #[test]
    fn test_contains() {
        assert!(LoopConfig::new(0.0, 1.0, 100).contains_buffer_offset(0.0));
        assert!(!LoopConfig::new(0.0, 1.0, 100).contains_buffer_offset(100.0));
        assert!(!LoopConfig::new(0.5, 1.0, 100).contains_buffer_offset(100.0));
        assert!(LoopConfig::new(0.5, 0.7, 100).contains_buffer_offset(50.0));
        assert!(LoopConfig::new(0.5, 0.7, 100).contains_buffer_offset(0.0));
        assert!(LoopConfig::new(0.5, 0.7, 100).contains_buffer_offset(99.0));
        assert!(!LoopConfig::new(0.5, 0.7, 100).contains_buffer_offset(100.0));
        assert!(!LoopConfig::new(0.5, 0.7, 100).contains_buffer_offset(30.0));
    }

    #[test]
    fn test_reflected_direction() {}
}
