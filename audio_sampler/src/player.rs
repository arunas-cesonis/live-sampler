use crate::common_types::LoopMode;
use std::ops::Index;

pub struct Player {
    offset: f32,
    speed: f32,
    mode: LoopMode,
}

impl Player {}

#[cfg(test)]
mod test {
    use crate::sampler::LoopMode;
    use crate::utils::{normalize_offset, ping_pong2, ping_pong3};
    use nih_plug_vizia::vizia::image::imageops::interpolate_bilinear;
    use std::ops::Index;

    pub struct Buffer {
        data: Vec<f32>,
    }

    pub fn mk_identity() -> impl Fn(f32) -> (f32, f32) {
        |offset: f32| -> (f32, f32) { (offset, 1.0) }
    }

    pub fn mk_loop<F>(v: F, start: f32, len: f32) -> impl Fn(f32) -> (f32, f32) + 'static
    where
        F: Fn(f32) -> (f32, f32),
    {
        move |offset: f32| -> (f32, f32) {
            let offset = normalize_offset(start + offset, len);
            (offset, 1.0)
        }
    }

    pub fn mk_ping_pong<F>(v: F, start: f32, len: f32) -> impl Fn(f32) -> (f32, f32) + 'static
    where
        F: Fn(f32) -> (f32, f32),
    {
        move |offset: f32| -> (f32, f32) { ping_pong2(start + offset, len) }
    }

    #[test]
    fn test_translate() {
        let f = mk_identity();
        let g = mk_loop(f, 0.0, 5.0);
        let f = mk_identity();
        let h = mk_ping_pong(f, 0.0, 5.0);

        eprintln!("{:?}", g(10.5));
        let mut pos = 0.0;
        let mut speed = -1.0;
        let mut direction = 1.0;
        let mut mode = LoopMode::Loop;
        let mut prev_index = 0.0;

        for i in 0..20 {
            if i == 12 {
                pos = match mode {
                    LoopMode::Loop => g(pos),
                    LoopMode::PingPong => h(pos),
                    _ => g(pos),
                }
                .0;
                mode = LoopMode::PingPong;
            }
            let y = match mode {
                LoopMode::Loop => g(pos),
                LoopMode::PingPong => h(pos),
                _ => g(pos),
            };
            eprintln!("i={:<4} speed={:<4} pos={:<4} index={:?}", i, speed, pos, y);
            prev_index = y.0;
            pos += speed;
        }
    }
}
