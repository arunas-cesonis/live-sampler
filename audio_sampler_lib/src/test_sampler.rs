#[cfg(test)]
mod test {
    use std::f32::consts::PI;

    use crate::common_types::{InitParams, Note, NoteOffBehaviour, Params, RecordingMode};
    use crate::sampler::{LoopMode, Sampler};
    use crate::time_value::TimeOrRatio;


    pub fn one_to(n: usize) -> Vec<f32> {
        (1..=n).map(|x| x as f32).collect()
    }

    pub fn one_to_ten() -> Vec<f32> {
        one_to(10)
    }

    pub fn one_to_five() -> Vec<f32> {
        (1..=5).map(|x| x as f32).collect()
    }

    pub fn ten_tens() -> Vec<f32> {
        vec![100.0; 10]
    }

    pub fn five_tens() -> Vec<f32> {
        vec![100.0; 5]
    }

    fn base_params() -> Params {
        let params = Params {
            loop_mode: LoopMode::PlayOnce,
            attack_samples: 0,
            decay_samples: 0,
            loop_length: TimeOrRatio::Ratio(1.0),
            recording_mode: RecordingMode::NoteTriggered,
            note_off_behavior: NoteOffBehaviour::Decay,
            ..Params::default()
        };
        params
    }

    #[derive(Copy, Clone, Debug)]
    enum Cmd {
        StartPlaying { start_percent: f32 },
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
                sampler: Sampler::new(1, &InitParams::default()),
                params,
                now: 0,
                cmds: vec![],
            }
        }

        fn schedule(&mut self, at: usize, cmd: Cmd) {
            self.cmds.push((at, cmd));
        }

        fn sampler(&self) -> &Sampler {
            &self.sampler
        }

        fn run_input<I>(&mut self, input: I) -> Vec<f32>
            where
                I: IntoIterator<Item=f32>,
        {
            input
                .into_iter()
                .map(|x| {
                    let (todo, rem) = self.cmds.iter().partition(|(at, _)| *at == self.now);
                    for &(_, cmd) in &todo {
                        match cmd {
                            Cmd::StartPlaying { start_percent: pos } => {
                                self.sampler
                                    .start_playing(pos, Note::new(11, 0), 1.0, &self.params)
                            }
                            Cmd::StopPlaying => {
                                self.sampler.stop_playing(Note::new(11, 0), &self.params)
                            }
                            Cmd::StartRecording => self.sampler.start_recording(&self.params),
                            Cmd::StopRecording => self.sampler.stop_recording(&self.params),
                        }
                    }
                    self.cmds = rem;
                    let mut x = x;
                    let mut frame = vec![&mut x];
                    self.sampler.process_frame(&mut frame, &self.params);
                    self.now += 1;
                    x
                })
                .collect::<Vec<_>>()
        }
    }

    #[test]
    fn test_play_once() {
        let params = Params {
            loop_mode: LoopMode::PlayOnce,
            attack_samples: 0,
            decay_samples: 0,
            loop_length: TimeOrRatio::Ratio(0.5),
            ..base_params()
        };
        let input = vec![one_to_ten(), ten_tens()].concat();

        // record first 10 samples, then PlayOnce with loop length 50%
        let mut host = Host::new(params.clone());
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone().clone());
        assert_eq!(
            output,
            vec![one_to_ten(), one_to_five(), five_tens()].concat(),
        );

        // record first 10 samples, then PlayOnce with loop length 100%
        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(1.0),
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(output, vec![one_to_ten(), one_to_ten()].concat());

        // record first 10 samples, then wait for 2 samples and PlayOnce with loop length 50%
        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(0.5),
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(12, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![
                one_to_ten(),
                vec![100.0, 100.0, 1.0, 2.0, 3.0, 4.0, 5.0, 100.0, 100.0, 100.0],
            ]
                .concat()
        );

        // record first 10 samples, then wait for 2 samples and PlayOnce with loop length 100%
        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(1.0),
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(12, Cmd::StartPlaying { start_percent: 0.0 });
        let tmp = host.clone();
        let output = host.run_input(vec![input.clone(), ten_tens()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten(),
                vec![100.0, 100.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
                vec![9.0, 10.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
            ]
                .concat()
        );

        // same as above, but backwards
        let mut host = tmp;
        host.params.speed = -1.0;
        let output = host.run_input(vec![input.clone(), ten_tens()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten(),
                vec![100.0, 100.0, 10.0, 9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0],
                vec![1.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
            ]
                .concat()
        );
    }

    #[test]
    fn test_play_once_reverse_crossing_boundary() {
        let params = Params {
            loop_mode: LoopMode::PlayOnce,
            attack_samples: 0,
            decay_samples: 0,
            loop_length: TimeOrRatio::Ratio(1.0),
            ..base_params()
        };
        let ten_tens = vec![100.0; 10];
        let _five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let _one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();

        // backwards crossing data boundary
        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(1.0),
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
                vec![100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0],
            ]
                .concat()
        );
    }

    #[test]
    fn test_looping() {
        let params = Params {
            loop_mode: LoopMode::Loop,
            attack_samples: 0,
            decay_samples: 0,
            loop_length: TimeOrRatio::Ratio(1.0),
            ..base_params()
        };
        let ten_tens = vec![100.0; 10];
        let _five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();

        // record first 10 samples, then for 10 samples duration play loop length 100%
        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(1.0),
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
            loop_length: TimeOrRatio::Ratio(0.5),
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
            loop_length: TimeOrRatio::Ratio(0.5),
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
            loop_length: TimeOrRatio::Ratio(0.5),
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
            loop_length: TimeOrRatio::Ratio(0.6),
            speed: -1.0,
            ..base_params()
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
            loop_length: TimeOrRatio::Ratio(1.0),
            ..base_params()
        };
        let ten_tens = vec![100.0; 10];
        let _five_tens = vec![100.0; 5];
        let one_to_ten = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let _one_to_five = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = vec![one_to_ten.clone(), ten_tens.clone()].concat();
        // record first 10 samples, then play loop length 50% from 80% in reverse
        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(0.6),
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
            loop_length: TimeOrRatio::Ratio(1.0),
            speed: -1.0,
            ..base_params()
        };
        let _five_tens = vec![100.0; 5];
        let input = vec![one_to_ten(), ten_tens()].concat();

        // rec 10, play rev 10
        let mut host = Host::new(params.clone());
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(
            output,
            vec![one_to_ten(), one_to_ten().into_iter().rev().collect()].concat()
        );

        // rec 10, play rev 2x10 in 20
        let mut host = Host::new(params.clone());
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(vec![input.clone(), ten_tens()].concat());
        assert_eq!(
            output,
            vec![
                one_to_ten(),
                one_to_ten().into_iter().rev().collect(),
                one_to_ten().into_iter().rev().collect(),
            ]
            .concat(),
        );
    }

    #[test]
    fn test_looping_rev3() {
        let params = Params {
            loop_mode: LoopMode::Loop,
            attack_samples: 0,
            decay_samples: 0,
            loop_length: TimeOrRatio::Ratio(1.0),
            speed: -1.0,
            ..base_params()
        };
        // rec 10, play rev 2x5 in 20
        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(0.5),
            ..params
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(10, Cmd::StopRecording);
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        host.schedule(20, Cmd::StopPlaying);
        let input = vec![one_to_ten(), ten_tens()].concat();
        let input = vec![input.clone(), ten_tens()].concat();
        let output = host.run_input(input.iter().copied());
        let expected = vec![
            one_to_ten(),
            one_to_five().into_iter().rev().collect(),
            one_to_five().into_iter().rev().collect(),
            ten_tens(),
        ]
            .concat();
        for i in 0..output.len() {
            eprintln!(
                "i: {} input: {} output: {} expected: {} ",
                i, input[i], output[i], expected[i]
            );
        }
        assert_eq!(output, expected);
    }

    #[test]
    fn test_empty_data() {
        let params = Params {
            loop_mode: LoopMode::PingPong,
            attack_samples: 0,
            decay_samples: 0,
            loop_length: TimeOrRatio::Ratio(1.0),
            ..base_params()
        };
        let one_to_ten: Vec<_> = (0..10).map(|x| x as f32).collect();
        let ten_zeros = vec![0.0; 10];
        let input = vec![one_to_ten.clone(), ten_zeros.clone(), ten_zeros.clone()].concat();

        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(1.0),
            ..params.clone()
        });
        let _tmp = host.clone();
        host.schedule(10, Cmd::StartPlaying { start_percent: 0.0 });
        let output = host.run_input(input.clone());
        assert_eq!(input, output);
    }

    #[test]
    fn test_waveform_info() {
        let params = Params {
            loop_mode: LoopMode::PingPong,
            attack_samples: 0,
            decay_samples: 0,
            loop_length: TimeOrRatio::Ratio(1.0),
            ..base_params()
        };
        let input = (0..44100)
            .map(|x| {
                let t = (x as f32) / 44100.0;
                let r = t * (180.0 / PI);
                r.cos()
            })
            .collect::<Vec<_>>();

        //let input = (0..44100).map(|x| x as f32).collect::<Vec<_>>();

        let mut host = Host::new(Params {
            loop_length: TimeOrRatio::Ratio(1.0),
            ..params.clone()
        });
        host.schedule(0, Cmd::StartRecording);
        host.schedule(input.len(), Cmd::StopRecording);
        host.run_input(input);

        let wave = host.sampler().get_waveform_summary(20);
        for (i, x) in wave.data.iter().take(20).enumerate() {
            eprintln!("{:<4}: {:?}", i, x);
        }
    }

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
                    recording_mode: RecordingMode::NoteTriggered,
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
                I: IntoIterator<Item=f32>,
        {
            self.start_recording();
            let out = self.run_input(input);
            self.stop_recording();
            out
        }
        pub fn run_input<I>(&mut self, input: I) -> Vec<f32>
            where
                I: IntoIterator<Item=f32>,
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
        pub fn start_recording(&mut self) {
            self.sampler.start_recording(&self.params);
        }
        pub fn stop_recording(&mut self) {
            self.sampler.stop_recording(&self.params);
        }
    }

    #[test]
    fn test_updating_params() {
        let mut h = EasyHost::default();
        h.record(one_to_ten());
        h.start_playing(0.0);

        assert_eq!(h.run(3), vec![1.0, 2.0, 3.0]);
        assert_eq!(h.run(7), vec![4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);

        assert_eq!(h.run(3), vec![1.0, 2.0, 3.0]);
        h.params.loop_length = TimeOrRatio::Ratio(0.5);
        assert_eq!(h.run(7), vec![4.0, 5.0, 1.0, 2.0, 3.0, 4.0, 5.0]);

        assert_eq!(h.run(3), vec![1.0, 2.0, 3.0]);
        h.params.loop_length = TimeOrRatio::Ratio(0.3);
        assert_eq!(h.run(7), vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0]);

        assert_eq!(h.run(3), vec![2.0, 3.0, 1.0]);
        h.params.reverse_speed = -1.0;
        assert_eq!(h.run(7), vec![1.0, 3.0, 2.0, 1.0, 3.0, 2.0, 1.0]);

        h.params.loop_length = TimeOrRatio::Ratio(1.0);
        assert_eq!(h.run(7), vec![2.0, 1.0, 10.0, 9.0, 8.0, 7.0, 6.0]);

        h.params.loop_mode = LoopMode::PingPong;

        assert_eq!(
            h.run(10),
            vec![4.0, 3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
        );
    }

    #[test]
    fn test_updating_data_length() {
        let mut h = EasyHost::default();
        h.record(one_to_ten());
        h.start_playing(0.0);
        assert_eq!(h.run(3), vec![1.0, 2.0, 3.0]);
        h.start_recording();
        assert_eq!(h.run_input(vec![11., 22., 33.]), vec![4.0, 5.0, 6.0]);
        h.stop_recording();
        assert_eq!(h.run(3), vec![11.0, 22.0, 33.0]);
        assert_eq!(h.run(2), vec![11.0, 22.0]);
        h.start_recording();
        assert_eq!(h.run_input(vec![111., 222., 333.]), vec![33.0, 111.0, 222.0]);
        h.stop_recording();
        assert_eq!(h.run(3), vec![333.0, 111.0, 222.0]);
    }

    #[test]
    fn test_updating_speed() {
        let mut h = EasyHost::default();
        h.record(one_to(50));
        h.start_playing(0.0);
        h.params.speed = 2.0;
        h.params.loop_mode = LoopMode::PingPong;
        for i in 0..100 {
            let out = h.run(1);
            let is_rev = h.sampler.channels[0].voices[0].clip2.is_pingpong_reversing(h.sampler.channels[0].now);
            eprintln!("now={:>8} {:<8} {:<6} {:?}", i, out[0], is_rev, h.sampler.channels[0].voices[0].clip2);
        }
    }
}