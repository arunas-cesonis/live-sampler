#[cfg(test)]
mod test {
    use crate::sampler::{LoopMode, Params, Sampler};

    #[derive(Copy, Clone)]
    enum Cmd {
        StartPlaying { pos: f32 },
        StopPlaying,
        StartRecording,
        StopRecording,
    }

    struct Host {
        sampler: Sampler,
        params: Params,
        now: usize,
        cmds: Vec<(usize, Cmd)>,
    }
    impl Host {
        fn new(params: Params) -> Self {
            Host {
                sampler: Sampler::new(1, &params),
                params,
                now: 0,
                cmds: vec![],
            }
        }

        fn schedule(&mut self, at: usize, cmd: Cmd) {
            self.cmds.push((at, cmd));
        }

        fn run_input<I>(&mut self, input: I) -> Vec<f32>
        where
            I: IntoIterator<Item = f32>,
        {
            input
                .into_iter()
                .map(|x| {
                    let (todo, rem) = self.cmds.iter().partition(|(at, _)| *at == self.now);
                    for (_, x) in todo {
                        match x {
                            Cmd::StartPlaying { pos } => {
                                self.sampler.start_playing(pos, 11, 1.0, &self.params)
                            }
                            Cmd::StopPlaying => self.sampler.stop_playing(11, &self.params),
                            Cmd::StartRecording => self.sampler.start_recording(&self.params),
                            Cmd::StopRecording => self.sampler.stop_recording(&self.params),
                        }
                    }
                    self.cmds = rem;
                    let mut frame = vec![x];
                    self.sampler.process_sample(&mut frame, &self.params);
                    self.now += 1;
                    frame[0]
                })
                .collect::<Vec<_>>()
        }
    }

    fn run_sampler(
        input: &[f32],
        note_on_index: usize,
        start_percent: f32,
        params: &Params,
    ) -> Vec<f32> {
        //let params = Params {
        //    loop_mode: LoopMode::Loop,
        //    attack_samples: 0,
        //    decay_samples: 0,
        //    loop_length_percent: 0.5,
        //    ..Params::default()
        //};
        let mut sampler = Sampler::new(1, &params);
        sampler.start_recording(&params);
        input.iter().for_each(|x| {
            sampler.process_sample(&mut [*x], &params);
        });
        sampler.stop_recording(&params);

        let mut buffer = vec![0.0; 10];
        let output: Vec<_> = buffer
            .into_iter()
            .enumerate()
            .map(|(index, x)| {
                if index == note_on_index {
                    sampler.start_playing(start_percent, 11, 1.0, &params);
                }
                let mut frame = vec![x];
                sampler.process_sample(&mut frame, &params);
                frame[0]
            })
            .collect();
        sampler.stop_playing(11, &params);
        output
    }
    fn simple_input() -> Vec<f32> {
        let input: Vec<_> = (0..10).into_iter().map(|x| x as f32).collect();
        input
    }

    #[test]
    fn test_play_once() {
        let params = Params {
            loop_mode: LoopMode::PlayOnce,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 0.5,
            ..Params::default()
        };
        let mut host = Host::new(params.clone());
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { pos: 0.0 });
        let tens = vec![10.0; 10];
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let output = host.run_input(vec![input.clone(), tens].concat());
        assert_eq!(
            &output[input.len()..output.len()],
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 10.0, 10.0, 10.0, 10.0, 10.0]
        );
        eprintln!("{:?}", host.sampler);
        eprintln!("{:?}", output);
    }

    #[test]
    fn test_looping() {
        let input = simple_input();
        let params = Params {
            loop_mode: LoopMode::Loop,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 0.5,
            ..Params::default()
        };

        let output = run_sampler(&input, 0, 0.0, &params);
        assert_eq!(&output[0..5], &input[0..5]);
        assert_eq!(&output[5..10], &input[0..5]);

        let output = run_sampler(&input, 0, 0.8, &params);
        eprintln!("{:?}", output);
        assert_eq!(output[0..5], vec![&input[8..10], &input[0..3]].concat());
        assert_eq!(output[5..10], vec![&input[8..10], &input[0..3]].concat());
        //assert_eq!(output[0], input[8]);
        //assert_eq!(output[1], input[9]);
        //assert_eq!(output[2], input[0]);
        //assert_eq!(output[3], input[1]);
        //assert_eq!(output[4], input[2]);
        //assert_eq!(output[5], input[8]);
        //assert_eq!(output[6], input[9]);
        //assert_eq!(output[7], input[0]);
        //assert_eq!(output[8], input[1]);
        //assert_eq!(output[9], input[2]);
        //assert_eq!(&output[0..5], &input[0..5]);
        //assert_eq!(&output[5..10], &input[0..5]);
    }
}
