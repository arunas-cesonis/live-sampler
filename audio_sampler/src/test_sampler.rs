#[cfg(test)]
mod test {
    use crate::sampler::{LoopMode, Params, Sampler};

    #[derive(Copy, Clone, Debug)]
    enum Cmd {
        StartPlaying { pos: f32 },
        StopPlaying,
        StartRecording,
        StopRecording,
    }

    #[derive(Clone, Debug)]
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
            loop_length_percent: 1.0,
            ..Params::default()
        };
        let ten_tens = vec![100.0; 10];
        let five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();

        // record first 10 smaples, then PlayOnce with loop length 50%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { pos: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_five.clone(), five_tens.clone()].concat()
        );

        // record first 10 smaples, then PlayOnce with loop length 100%
        let mut host = Host::new(Params {
            loop_length_percent: 1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { pos: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_ten.clone()].concat()
        );

        // record first 10 smaples, then wait for 2 samples and PlayOnce with loop length 50%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(12, Cmd::StartPlaying { pos: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![100.0, 100.0, 1.0, 2.0, 3.0, 4.0, 5.0, 100.0, 100.0, 100.0]
            ]
            .concat()
        );

        // record first 10 smaples, then wait for 2 samples and PlayOnce with loop length 100%
        let mut host = Host::new(Params {
            loop_length_percent: 1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(12, Cmd::StartPlaying { pos: 0.0 });
        let tmp = host.clone();
        let output = host.run_input(vec![input.clone(), ten_tens.clone()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![100.0, 100.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
                vec![9.0, 10.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]
            ]
            .concat()
        );

        // same as above, but backwards
        let mut host = tmp;
        host.params.speed = -1.0;
        let output = host.run_input(vec![input.clone(), ten_tens.clone()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![100.0, 100.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0],
                vec![2.0, 1.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]
            ]
            .concat()
        );

        eprintln!("{:?}", host.sampler);
        eprintln!("{:?}", output);
    }

    #[test]
    fn test_looping() {
        let params = Params {
            loop_mode: LoopMode::Loop,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 1.0,
            ..Params::default()
        };
        let ten_tens = vec![100.0; 10];
        let five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();

        // record first 10 smaples, then for 10 samples duration play loop length 100%
        let mut host = Host::new(Params {
            loop_length_percent: 10.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { pos: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_ten.clone()].concat()
        );

        // record first 10 smaples, then for 10 samples duration play loop length 50%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { pos: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_five.clone(), one_to_five.clone()].concat()
        );

        // ** BROKEN ** //
        // ** BROKEN ** //
        // ** BROKEN ** //
        // ** BROKEN ** //
        panic!("BROKEN, PLESAE FIX");

        // record first 10 smaples, wait 2 samples and then for 8 samples duration play loop length 50% from 20%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(12, Cmd::StopRecording);
        host.schedule(12, Cmd::StartPlaying { pos: 0.20 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![100.0, 100.0],
                vec![3.0, 4.0, 5.0, 6.0, 7.0, 3.0, 4.0, 5.0],
            ]
            .concat()
        );
    }
}
