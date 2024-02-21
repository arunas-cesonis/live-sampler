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
    //
    // given the above and current time 'now' sample index played is calculated as
    //
    // (((now - updated_at) * speed + local_adjustment) % length + offset) % data.len()
    //
    // where in the calculation the '%" is modulo operator which flips negative values to positive
    // 'mirroring' against the second argument, e.g. -5 % 3 = -2 + 3 = 1
}

impl Clip {
    pub fn new(
        now: usize,
        offset: usize,
        length: usize,
        offset_adjustment: usize,
        speed: f32,
    ) -> Self {
        Self {
            updated_at: now,
            offset,
            length: length.max(1),
            local_adjustment: offset_adjustment,
            speed,
        }
    }

    fn local_offset(&self, now: usize, adjust: f32) -> usize {
        let elapsed = (now - self.updated_at) as f32;
        let elapsed_scaled = elapsed * self.speed;

        let local_offset = elapsed_scaled + self.local_adjustment as f32 + adjust;

        // Alternative way to calculate ping pong
        //
        // let times = (local_offset / self.length as f32).floor() as usize;
        // eprintln!("now={} times={} local_offset={}", now, times, local_offset);
        //

        // This does ping pong but sort of breaks reversing
        ///
        // (local_offset, _) = ping_pong3(local_offset, self.length as f32, self.speed);
        let local_offset = normalize_offset(local_offset, self.length as f32);
        debug_assert!(
            local_offset >= 0.0,
            "local_offset={} now={} self={:?}",
            local_offset,
            now,
            self
        );
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
        debug_assert!(new_length > 0, "new_length={} now={} self={:?}", new_length, now, self);
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
    fn test_clip() {
        let mut now = 0;
        let mut clip = Clip::new(now, 0, 10, 0, -1.0);
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
