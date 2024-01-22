#[cfg(test)]
mod test {
    use crate::sampler::{LoopMode, Params, Sampler};

    #[derive(Copy, Clone, Debug)]
    enum Cmd {
        StartPlaying { start_percent: f32 },
        StopPlaying,
        StartRecording,
        StopRecording,
        SetLoopLength { length_percent: f32 },
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
                            Cmd::StartPlaying { start_percent: pos } => {
                                self.sampler.start_playing(pos, 11, 1.0, &self.params)
                            }
                            Cmd::StopPlaying => self.sampler.stop_playing(11, &self.params),
                            Cmd::StartRecording => self.sampler.start_recording(&self.params),
                            Cmd::StopRecording => self.sampler.stop_recording(&self.params),
                            Cmd::SetLoopLength { length_percent } => {
                                self.params.loop_length_percent = length_percent;
                            }
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
        let mut sampler = Sampler::new(1, &params);
        sampler.start_recording(&params);
        input.iter().for_each(|x| {
            sampler.process_sample(&mut [*x], &params);
        });
        sampler.stop_recording(&params);

        let buffer = vec![0.0; 10];
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

        // record first 10 samples, then PlayOnce with loop length 50%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_five.clone(), five_tens.clone()].concat(),
        );

        // record first 10 samples, then PlayOnce with loop length 100%
        let mut host = Host::new(Params {
            loop_length_percent: 1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_ten.clone()].concat()
        );

        // record first 10 samples, then wait for 2 samples and PlayOnce with loop length 50%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(12, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![100.0, 100.0, 1.0, 2.0, 3.0, 4.0, 5.0, 100.0, 100.0, 100.0]
            ]
            .concat()
        );

        // record first 10 samples, then wait for 2 samples and PlayOnce with loop length 100%
        eprintln!("************************************************************");
        let mut host = Host::new(Params {
            loop_length_percent: 1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(12, Cmd::StartPlaying { start_percent: 0.0 });
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

        // backwards crossing data boundary
        eprintln!("************************************************************");
        let mut host = Host::new(Params {
            loop_length_percent: 1.0,
            speed: -1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.5 });
        let _tmp = host.clone();
        let output = host.run_input(vec![input.clone(), ten_tens.clone()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![5.0, 4.0, 3.0, 2.0, 1.0, 10.0, 9.0, 8.0, 7.0, 6.0],
                vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]
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
        let _five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();

        // record first 10 samples, then for 10 samples duration play loop length 100%
        let mut host = Host::new(Params {
            loop_length_percent: 1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_ten.clone()].concat()
        );

        // record first 10 samples, then for 10 samples duration play loop length 50%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten.clone(), one_to_five.clone(), one_to_five.clone()].concat(),
            "record first 10 samples, then for 10 samples duration play loop length 50%"
        );

        // record first 10 samples, wait 2 samples and then for 8 samples duration play loop length 50% from 20%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(
            12,
            Cmd::StartPlaying {
                start_percent: 0.20,
            },
        );
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

        // record first 10 samples, then play loop length 50% from 80%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(
            10,
            Cmd::StartPlaying {
                start_percent: 0.80,
            },
        );
        let _tmp = host.clone();
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![9.0, 10.0, 1.0, 2.0, 3.0],
                vec![9.0, 10.0, 1.0, 2.0, 3.0],
            ]
            .concat()
        );
    }

    #[test]
    fn test_dyn_length() {
        let params = Params {
            loop_mode: LoopMode::Loop,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 0.6,
            speed: -1.0,
            ..Params::default()
        };
        let ten_tens = vec![100.0; 10];
        let _five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let _one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();
        // record first 10 samples, then play loop length 50% from 80% in reverse
        let mut host = Host::new(params.clone());
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(
            10,
            Cmd::StartPlaying {
                start_percent: 0.50,
            },
        );
        let _tmp = host.clone();
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![1.0, 10.0, 9.0, 8.0, 7.0, 6.0],
                vec![1.0, 10.0, 9.0, 8.0],
            ]
            .concat()
        );
    }

    #[test]
    fn test_looping_rev() {
        let params = Params {
            loop_mode: LoopMode::Loop,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 1.0,
            ..Params::default()
        };
        let ten_tens = vec![100.0; 10];
        let _five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let _one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();
        // record first 10 samples, then play loop length 50% from 80% in reverse
        let mut host = Host::new(Params {
            loop_length_percent: 0.6,
            speed: -1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(
            10,
            Cmd::StartPlaying {
                start_percent: 0.50,
            },
        );
        let _tmp = host.clone();
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![1.0, 10.0, 9.0, 8.0, 7.0, 6.0],
                vec![1.0, 10.0, 9.0, 8.0],
            ]
            .concat()
        );
    }

    #[test]
    fn test_looping_rev2() {
        let params = Params {
            loop_mode: LoopMode::Loop,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 1.0,
            speed: -1.0,
            ..Params::default()
        };
        let ten_tens = vec![100.0; 10];
        let _five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();

        // rec 10, play rev 10
        let mut host = Host::new(params.clone());
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                one_to_ten.clone().into_iter().rev().collect()
            ]
            .concat()
        );

        // rec 10, play rev 2x10 in 20
        let mut host = Host::new(params.clone());
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(vec![input.clone(), ten_tens.clone()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                one_to_ten.clone().into_iter().rev().collect(),
                one_to_ten.clone().into_iter().rev().collect(),
            ]
            .concat()
        );

        // rec 10, play rev 2x5 in 20
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        host.schedule(20, Cmd::StopPlaying);
        let output = host.run_input(vec![input.clone(), ten_tens.clone()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                one_to_five.clone().into_iter().rev().collect(),
                one_to_five.clone().into_iter().rev().collect(),
                ten_tens.clone(),
            ]
            .concat()
        );
    }

    #[test]
    fn test_ping_pong() {
        let params = Params {
            loop_mode: LoopMode::PingPong,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 1.0,
            ..Params::default()
        };
        let one_to_ten: Vec<_> = (0..10).map(|x| x as f32).collect();
        let one_to_five: Vec<_> = (0..5).map(|x| x as f32).collect();
        let ten_tens = vec![777.0; 10];
        let input = vec![one_to_ten.clone(), ten_tens.clone(), ten_tens.clone()].concat();

        // record first 10 samples, then PingPong
        let mut host = Host::new(Params { ..params.clone() });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                one_to_ten.clone(),
                one_to_ten.clone().into_iter().rev().collect()
            ]
            .concat(),
        );

        // record first 10 samples, then PingPong 50%
        let mut host = Host::new(Params {
            loop_length_percent: 0.5,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        //
        let tmp = host.clone();
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                one_to_five.clone(),
                one_to_five.clone().into_iter().rev().collect(),
                one_to_five.clone(),
                one_to_five.clone().into_iter().rev().collect(),
            ]
            .concat(),
        );
        // wraps
        let mut host = tmp.clone();
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.8 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                vec![8.0, 9.0, 0.0, 1.0, 2.0],
                vec![2.0, 1.0, 0.0, 9.0, 8.0],
                vec![8.0, 9.0, 0.0, 1.0, 2.0],
                vec![2.0, 1.0, 0.0, 9.0, 8.0],
            ]
            .concat(),
        );
    }

    #[test]
    fn test_ping_pong_rev() {
        let params = Params {
            loop_mode: LoopMode::PingPong,
            attack_samples: 0,
            decay_samples: 0,
            loop_length_percent: 1.0,
            ..Params::default()
        };
        let one_to_ten: Vec<_> = (0..10).map(|x| x as f32).collect();
        let _one_to_five: Vec<_> = (0..5).map(|x| x as f32).collect();
        let ten_tens = vec![777.0; 10];
        let input = vec![one_to_ten.clone(), ten_tens.clone(), ten_tens.clone()].concat();

        // record first 10 samples, then PingPong 50%
        let mut host = Host::new(Params {
            loop_length_percent: 1.0,
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        //
        let _tmp = host.clone();
        host.params.speed = -1.0;
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten.clone(),
                one_to_ten.clone().into_iter().rev().collect(),
                one_to_ten.clone(),
            ]
            .concat(),
        );
    }
}
