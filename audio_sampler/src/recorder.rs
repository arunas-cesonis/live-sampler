use crate::common_types;
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
    pub transport_pos_samples: f32,
    pub sample_id: usize,
    pub recording_mode: RecordingMode,
    pub fixed_size_samples: usize,
}

impl From<&common_types::Params> for Params {
    fn from(params: &common_types::Params) -> Self {
        Self {
            transport_pos_samples: params.transport.pos_samples,
            sample_id: params.sample_id,
            recording_mode: params.recording_mode,
            fixed_size_samples: params.fixed_size_samples,
        }
    }
}

impl Params {
    pub fn with_trigger(&self, trigger: bool) -> Self {
        Self {
            transport_pos_samples: self.transport_pos_samples,
            sample_id: self.sample_id,
            recording_mode: self.recording_mode,
            fixed_size_samples: self.fixed_size_samples,
        }
    }
    pub fn with_transport_pos_samples(&self, transport_pos_samples: i64) -> Self {
        Self {
            transport_pos_samples: transport_pos_samples as f32,
            sample_id: self.sample_id,
            recording_mode: self.recording_mode,
            fixed_size_samples: self.fixed_size_samples,
        }
    }
    pub fn with_recording_mode(&self, recording_mode: RecordingMode) -> Self {
        Self {
            transport_pos_samples: self.transport_pos_samples,
            sample_id: self.sample_id,
            recording_mode,
            fixed_size_samples: self.fixed_size_samples,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Triggers {
    Start,
    Stop,
}

#[derive(Default, Clone, Debug)]
pub struct RecorderErrors {
    skipped_samples: Vec<(usize, usize)>,
    negative_transport_pos: Vec<f32>,
    incorrect_state: Vec<(State, State)>,
}

#[derive(Clone, Debug)]
pub struct Recorder {
    state: State,
    errors: RecorderErrors,
}

impl Recorder {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            errors: RecorderErrors::default(),
        }
    }

    pub fn print_error_info(&self) -> String {
        format!(
            "skip: {:?}, neg: {:?}, inc: {:?}",
            self.errors.skipped_samples.len(),
            self.errors.negative_transport_pos.len(),
            self.errors.incorrect_state.len()
        )
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
                self.errors
                    .incorrect_state
                    .push((State::Triggered { write: 0 }, self.state.clone()));
            }
        }
    }

    pub fn last_recorded_offset(&self) -> Option<usize> {
        match self.state {
            State::AlwaysOn {
                last_recorded_offset,
                ..
            } => last_recorded_offset,
            State::Triggered { write, .. } => Some(write),
            _ => None,
        }
    }

    pub fn is_recording(&self) -> bool {
        match self.state {
            State::Triggered { .. } => true,
            State::AlwaysOn { .. } => true,
            _ => false,
        }
    }

    pub fn start(&mut self, data: &mut Vec<f32>, params: &Params) {
        self.handle_state_transitions(data, params);
        match self.state {
            State::Idle => {
                self.state = State::Triggered { write: 0 };
            }
            _ => {
                self.errors
                    .incorrect_state
                    .push((State::Idle, self.state.clone()));
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
                State::Idle => (),
                State::Triggered { .. } => (),
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
                let transport_pos_samples = params.transport_pos_samples;
                data.resize(params.fixed_size_samples, 0.0);
                if transport_pos_samples < 0.0 {
                    self.errors
                        .negative_transport_pos
                        .push(transport_pos_samples);
                }
                let i = normalize_offset(
                    transport_pos_samples + params.sample_id as f32,
                    *length as f32,
                );
                assert!(i >= 0.0, "i={}", i);
                let i = i as usize;
                data[i] = sample;
                if let Some(prev_offset) = *last_recorded_offset {
                    if !(i == 1 + prev_offset || i == 0 && prev_offset == *length - 1) {
                        self.errors.skipped_samples.push((i, prev_offset));
                    }
                }
                *last_recorded_offset = Some(i);
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
            transport_pos_samples: 0.0,
            fixed_size_samples: 100,
            recording_mode: RecordingMode::NoteTriggered,
            sample_id: 0,
        };
        let params = &params;
        rec.process_sample(1.0, &mut data, params);
        assert!(data.iter().all(|&x| x == 0.0));
        rec.start(&mut data, params);
        for i in 1..20 {
            rec.process_sample(i as f32, &mut data, params);
        }
        rec.stop(&mut data, params);
        rec.process_sample(0.0, &mut data, params);
        assert_eq!(data, (1..20).map(|x| x as f32).collect::<Vec<_>>());
        rec.start(&mut data, params);
        rec.process_sample(100.0, &mut data, params);
        assert_eq!(data[0], 100.0);
        let params = params.with_recording_mode(RecordingMode::AlwaysOn);
        rec.process_sample(101.0, &mut data, &params.with_transport_pos_samples(210));
        rec.process_sample(102.0, &mut data, &params.with_transport_pos_samples(211));
        rec.start(&mut data, &params);
        rec.process_sample(103.0, &mut data, &params.with_transport_pos_samples(212));

        assert_eq!(data[10], 101.0);
        assert_eq!(data[11], 102.0);
        assert_eq!(data[12], 103.0);
        let params = params.with_recording_mode(RecordingMode::NoteTriggered);
        rec.start(&mut data, &params);
        rec.process_sample(104.0, &mut data, &params.with_transport_pos_samples(213));
        rec.stop(&mut data, &params.with_transport_pos_samples(214));
        assert_eq!(data, vec![104.0], "{:#?}", rec);
    }
}
