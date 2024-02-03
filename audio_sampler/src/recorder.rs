use crate::common_types::RecordingMode;
use crate::utils::normalize_offset;

#[derive(Clone, Debug, PartialEq)]
enum State {
    Triggered {
        write: usize,
    },
    AlwaysOn {
        length: usize,
        last_recorded_offset: Option<usize>,
    },
    Idle,
}

pub struct Params {
    transport_pos_samples: Option<i64>,
    sample_id: usize,
    trigger: bool,
    recording_mode: RecordingMode,
    fixed_size_samples: usize,
}

impl Params {
    pub fn with_trigger(&self, trigger: bool) -> Self {
        Self {
            transport_pos_samples: self.transport_pos_samples,
            sample_id: self.sample_id,
            recording_mode: self.recording_mode,
            fixed_size_samples: self.fixed_size_samples,
            trigger,
        }
    }
    pub fn with_transport_pos_samples(&self, transport_pos_samples: i64) -> Self {
        Self {
            transport_pos_samples: Some(transport_pos_samples),
            sample_id: self.sample_id,
            recording_mode: self.recording_mode,
            fixed_size_samples: self.fixed_size_samples,
            trigger: self.trigger,
        }
    }
    pub fn with_recording_mode(&self, recording_mode: RecordingMode) -> Self {
        Self {
            transport_pos_samples: self.transport_pos_samples,
            sample_id: self.sample_id,
            recording_mode,
            fixed_size_samples: self.fixed_size_samples,
            trigger: self.trigger,
        }
    }
}

#[derive(Clone, Debug)]
enum RecorderError {
    SkippedSamples { i: usize, prev_offset: usize },
    NegativeTransportPos { transport_pos_samples: i64 },
    IncorrectState,
}

#[derive(Clone, Debug)]
pub struct Recorder {
    state: State,
    errors: Vec<RecorderError>,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            errors: vec![],
        }
    }

    fn always_on(&mut self, data: &mut Vec<f32>, params: &Params) {
        self.state = State::AlwaysOn {
            length: params.fixed_size_samples,
            last_recorded_offset: None,
        };
    }

    fn always_off(&mut self, params: &Params) {
        self.state = State::Idle;
    }

    pub fn stop(&mut self, data: &mut Vec<f32>, params: &Params) {
        match self.state {
            State::Triggered { write } => {
                data.truncate(write);
                self.state = State::Idle;
            }
            _ => {
                self.errors.push(RecorderError::IncorrectState);
            }
        }
    }

    pub fn start(&mut self) {
        match self.state {
            State::Idle => {
                self.state = State::Triggered { write: 0 };
            }
            _ => {
                self.errors.push(RecorderError::IncorrectState);
            }
        }
    }

    fn handle_state_transitions(&mut self, data: &mut Vec<f32>, params: &Params) {
        match (params.recording_mode) {
            RecordingMode::AlwaysOn => match self.state {
                State::Idle => self.always_on(data, params),
                State::Triggered { .. } => self.always_on(data, params),
                State::AlwaysOn { .. } => (),
            },
            RecordingMode::NoteTriggered => match self.state {
                State::Idle if params.trigger => self.start(),
                State::Idle => (),
                State::Triggered { .. } if params.trigger => (),
                State::Triggered { .. } => self.stop(data, params),
                State::AlwaysOn { .. } if params.trigger => {
                    self.always_off(params);
                    self.start();
                }
                State::AlwaysOn { .. } => {
                    self.always_off(params);
                }
            },
        }
    }

    pub fn process_sample(&mut self, sample: f32, data: &mut Vec<f32>, params: &Params) {
        self.handle_state_transitions(data, params);
        match &mut self.state {
            State::Triggered { write } => {
                let n = data.len();
                let i = *write;
                assert!(i <= n);
                if i == n {
                    data.push(sample);
                } else {
                    data[i] = sample;
                }
                *write += 1;
            }
            State::AlwaysOn {
                length,
                last_recorded_offset,
            } => {
                if let Some(transport_pos_samples) = params.transport_pos_samples {
                    data.resize(params.fixed_size_samples, 0.0);
                    if transport_pos_samples < 0 {
                        self.errors.push(RecorderError::NegativeTransportPos {
                            transport_pos_samples,
                        });
                    }
                    let i = normalize_offset(
                        transport_pos_samples + params.sample_id as i64,
                        *length as i64,
                    );
                    assert!(i >= 0, "i={}", i);
                    let i = i as usize;
                    data[i] = sample;
                    if let Some(prev_offset) = *last_recorded_offset {
                        if !(i == 1 + prev_offset || i == 0 && prev_offset == *length - 1) {
                            self.errors
                                .push(RecorderError::SkippedSamples { i, prev_offset });
                        }
                    }
                    *last_recorded_offset = Some(i);
                }
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_recorder() {
        let mut rec = Recorder::new();
        let mut data = vec![0.0; 10];
        let params = Params {
            transport_pos_samples: None,
            fixed_size_samples: 100,
            recording_mode: RecordingMode::NoteTriggered,
            sample_id: 0,
            trigger: false,
        };
        let params = &params;
        rec.process_sample(1.0, &mut data, params);
        assert!(data.iter().all(|&x| x == 0.0));
        let params = &params.with_trigger(true);
        for i in 1..20 {
            rec.process_sample(i as f32, &mut data, params);
        }
        let params = &params.with_trigger(false);
        rec.process_sample(0.0, &mut data, params);
        assert_eq!(data, (1..20).map(|x| x as f32).collect::<Vec<_>>());
        let params = &params.with_trigger(true);
        rec.process_sample(100.0, &mut data, params);
        assert_eq!(data[0], 100.0);
        let params = params.with_recording_mode(RecordingMode::AlwaysOn);
        rec.process_sample(101.0, &mut data, &params.with_transport_pos_samples(210));
        rec.process_sample(102.0, &mut data, &params.with_transport_pos_samples(211));
        let params = params.with_trigger(true);
        rec.process_sample(103.0, &mut data, &params.with_transport_pos_samples(212));

        assert_eq!(data[10], 101.0);
        assert_eq!(data[11], 102.0);
        assert_eq!(data[12], 103.0);
        let params = params
            .with_recording_mode(RecordingMode::NoteTriggered)
            .with_trigger(true);
        rec.process_sample(104.0, &mut data, &params.with_transport_pos_samples(213));
        rec.stop(&mut data, &params.with_transport_pos_samples(214));
        assert_eq!(data, vec![104.0], "{:#?}", rec);
    }
}
