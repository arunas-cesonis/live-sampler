use crate::utils::normalize_offset;

#[derive(Debug, Clone)]
pub struct Clip {
    // base index of where clip plays from
    offset: usize,
    // length of the slice played
    length: usize,
    // accumulated offset adjustment
    local_adjustment: usize,
    // sample number of when the clip was last updated
    updated_at: usize,
    // speed of the clip
    speed: f32,
    ping_pong_speed: f32,
    //
    // given the above and current time 'now' sample index played is calculated as
    //
    // (((now - updated_at) * ping_pong_speed * speed + local_adjustment) % length + offset) % data.len()
    //
    // where in the calculation the '%" is modulo operator which flips negative values to positive
    // 'mirroring' against the second argument, e.g. -5 % 3 = -2 + 3 = 1
}

impl Clip {
    pub fn new(now: usize, offset: usize, length: usize, speed: f32) -> Self {
        Self {
            updated_at: now,
            offset,
            length,
            local_adjustment: 0,
            speed,
            ping_pong_speed: 1.0,
        }
    }

    fn local_offset(&self, now: usize, adjust: f32) -> usize {
        let elapsed = (now - self.updated_at) as f32;
        let elapsed_scaled = elapsed * self.speed;

        let local_offset = elapsed_scaled + self.local_adjustment as f32 + adjust;
        let local_offset = normalize_offset(local_offset, self.length as f32);
        debug_assert!(local_offset >= 0.0, "local_offset={}", local_offset);
        local_offset.abs().floor() as usize
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn update_speed(&mut self, now: usize, new_speed: f32) {
        if self.speed == new_speed {
            return;
        }
        self.local_adjustment = self.local_offset(now, 0.0);
        self.updated_at = now;
        self.speed = new_speed;
    }

    pub fn update_offset(&mut self, new_offset: usize) {
        self.offset = new_offset;
    }

    pub fn update_length(&mut self, now: usize, new_length: usize) {
        if self.length == new_length {
            return;
        }
        let offset_adjustment = self.local_offset(now, 0.0);
        if offset_adjustment < new_length {
            self.local_adjustment = offset_adjustment;
        } else {
            self.local_adjustment = 0;
        }
        self.updated_at = now;
        self.length = new_length;
    }

    pub fn sample_index(&self, now: usize, data_len: usize) -> usize {
        let reverse_adjust = if self.speed < 0.0 { -1.0 } else { 0.0 };
        let data_index = (self.local_offset(now, reverse_adjust) + self.offset) % data_len;
        data_index
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn print_lines<A>(v: Vec<A>, per_line: usize) -> String
    where
        A: std::fmt::Debug,
    {
        let mut i = 0;
        let mut out = String::new();

        while i < v.len() {
            out.push_str(format!("{:<4}: {:?}\n", i, &v[i..(i + per_line).min(v.len())]).as_str());
            i += per_line;
        }
        out + "\n"
    }

    #[test]
    fn test_loop() {
        fn run(clip: &mut Clip, input: &[usize]) -> Vec<usize> {
            input
                .iter()
                .map(|i| clip.sample_index(*i, 100))
                .collect::<Vec<_>>()
        }
        let input = (0..20).collect::<Vec<_>>();

        let mut clip = Clip::new(0, 0, 5, 1.0);
        assert_eq!(
            run(&mut clip, &input),
            vec![vec![0, 1, 2, 3, 4]; 4].concat()
        );
        let mut clip = Clip::new(0, 2, 5, 1.0);
        assert_eq!(
            run(&mut clip, &input),
            vec![vec![2, 3, 4, 5, 6]; 4].concat()
        );
        let mut clip = Clip::new(0, 97, 5, 1.0);
        assert_eq!(
            run(&mut clip, &input),
            vec![vec![97, 98, 99, 0, 1]; 4].concat()
        );
        let mut clip = Clip::new(0, 97, 5, -1.0);
        assert_eq!(
            run(&mut clip, &input),
            vec![vec![1, 0, 99, 98, 97]; 4].concat()
        );
        let mut clip = Clip::new(0, 0, 5, 3.0);
        assert_eq!(
            run(&mut clip, &input),
            vec![vec![0, 3, 1, 4, 2]; 4].concat()
        );
        let mut clip = Clip::new(0, 0, 5, 0.5);
        assert_eq!(
            run(&mut clip, &input),
            vec![vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4]; 2].concat()
        );
        let mut clip = Clip::new(0, 97, 5, -0.5);
        assert_eq!(
            run(&mut clip, &input),
            vec![1, 0, 0, 99, 99, 98, 98, 97, 97, 1, 1, 0, 0, 99, 99, 98, 98, 97, 97, 1]
        );
    }

    #[test]
    fn test_clip() {
        let mut now = 0;
        let mut clip = Clip::new(now, 0, 10, -1.0);
        //now: usize,
        //offset: usize,
        //length: usize,
        //offset_adjustment: usize,
        //speed: f32,
        let mut out: Vec<f32> = Vec::new();
        let data: Vec<_> = (0..100).map(|x| x as f32).collect();
        while now < 65 {
            out.push(data[clip.sample_index(now, data.len())]);
            now += 1;
        }
        clip.update_length(now, 3);
        while now < 100 {
            out.push(data[clip.sample_index(now, data.len())]);
            now += 1;
        }
        eprintln!("{}", print_lines(out, 10));
    }
}
