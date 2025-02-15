#[cfg(test)]
mod test {
    use crate::common_types::*;
    use crate::sampler::*;
    use crate::time_value::*;
    use std::time::Instant;

    #[derive(Clone, Debug)]
    struct EasyHost {
        pub sampler: Sampler,
        pub params: Params,
        pub output: Vec<f32>,
    }

    impl Default for EasyHost {
        fn default() -> Self {
            Self {
                sampler: Sampler::new(1, &InitParams::default()),
                params: Params {
                    loop_mode: LoopMode::Loop,
                    attack_samples: 0,
                    decay_samples: 0,
                    loop_length: TimeOrRatio::Ratio(1.0),
                    ..Params::default()
                },
                output: vec![],
            }
        }
    }

    impl EasyHost {
        pub fn run(&mut self, n: usize) -> Vec<f32> {
            self.run_input(std::iter::repeat(0.0).take(n))
        }
        pub fn record<I>(&mut self, input: I) -> Vec<f32>
        where
            I: IntoIterator<Item = f32>,
        {
            self.start_recording();
            let out = self.run_input(input);
            self.stop_recording();
            out
        }
        pub fn run_input<I>(&mut self, input: I) -> Vec<f32>
        where
            I: IntoIterator<Item = f32>,
        {
            let mut output = vec![];
            for mut x in input {
                let mut frame = vec![&mut x];
                self.sampler.process_frame(&mut frame, &self.params);
                let y = *frame[0];
                self.output.push(y);
                output.push(y);
            }
            output
        }
        pub fn start_playing(&mut self, start_position: f32) {
            self.sampler
                .start_playing(start_position, Note::new(0, 0), 1.0, &self.params);
        }
        pub fn start_playing_note(&mut self, start_position: f32, note: u8) {
            self.sampler
                .start_playing(start_position, Note::new(note, 0), 1.0, &self.params);
        }
        pub fn stop_playing_note(&mut self, note: u8) {
            self.sampler.stop_playing(Note::new(note, 0), &self.params);
        }
        pub fn start_recording(&mut self) {
            self.sampler.start_recording();
        }
        pub fn stop_recording(&mut self) {
            self.sampler.stop_recording(&self.params);
        }
    }

    #[test]
    fn test_perf1() {
        let mut h = EasyHost::default();
        let t = Instant::now();
        h.record((0..1000).map(|i| (i as f32).sin()));
        h.start_playing(0.0);
        h.start_playing(0.5);
        h.params.loop_mode = LoopMode::PingPong;
        for i in 0..1000 {
            let _v = (i / 10) % 3;
            match (i / 10) % 3 {
                0 => h.start_playing_note(0.0, 1),
                1 => h.stop_playing_note(1),
                2 => (),
                _ => unreachable!(),
            }
            match (i / 20) % 3 {
                0 => h.start_playing_note(0.0, 2),
                1 => h.stop_playing_note(2),
                2 => (),
                _ => unreachable!(),
            }
            h.run(1000);
        }
        eprintln!("{:?}", t.elapsed());
    }
}
