use nih_plug::midi::NoteEvent;
use nih_plug::plugin::Plugin;
use pyo3::types::IntoPyDict;
use pyo3::{pyclass, FromPyObject, IntoPy, Py, PyObject, Python, ToPyObject};

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct NoteOn {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    velocity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct NoteOff {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    velocity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct Choke {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct VoiceTerminated {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyModulation {
    timing: u32,
    voice_id: i32,
    poly_modulation_id: u32,
    normalized_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct MonoAutomation {
    timing: u32,
    poly_modulation_id: u32,
    normalized_value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyPressure {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    pressure: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyVolume {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    gain: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyPan {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    pan: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyTuning {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    tuning: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyVibrato {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    vibrato: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyExpression {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    expression: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct PolyBrightness {
    timing: u32,
    voice_id: Option<i32>,
    channel: u8,
    note: u8,
    brightness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct MidiChannelPressure {
    timing: u32,
    channel: u8,
    pressure: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct MidiPitchBend {
    timing: u32,
    channel: u8,
    value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct MidiCC {
    timing: u32,
    channel: u8,
    cc: u8,
    value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct MidiProgramChange {
    timing: u32,
    channel: u8,
    program: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass]
pub struct MidiSysEx {
    timing: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPyObject)]
pub enum PyO3NoteEvent {
    NoteOn(NoteOn),
    NoteOff(NoteOff),
    Choke(Choke),
    VoiceTerminated(VoiceTerminated),
    PolyModulation(PolyModulation),
    MonoAutomation(MonoAutomation),
    PolyPressure(PolyPressure),
    PolyVolume(PolyVolume),
    PolyPan(PolyPan),
    PolyTuning(PolyTuning),
    PolyVibrato(PolyVibrato),
    PolyExpression(PolyExpression),
    PolyBrightness(PolyBrightness),
    MidiChannelPressure(MidiChannelPressure),
    MidiPitchBend(MidiPitchBend),
    MidiCC(MidiCC),
    MidiProgramChange(MidiProgramChange),
    MidiSysEx(MidiSysEx),
}

impl ToPyObject for PyO3NoteEvent {
    fn to_object(&self, py: Python) -> PyObject {
        match self {
            PyO3NoteEvent::NoteOn(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::NoteOff(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::Choke(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::VoiceTerminated(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyModulation(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::MonoAutomation(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyPressure(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyVolume(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyPan(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyTuning(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyVibrato(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyExpression(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::PolyBrightness(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::MidiChannelPressure(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::MidiPitchBend(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::MidiCC(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::MidiProgramChange(x) => Py::new(py, *x).unwrap().into_py(py),
            PyO3NoteEvent::MidiSysEx(x) => Py::new(py, *x).unwrap().into_py(py),
        }
    }
}

impl Into<NoteEvent<()>> for PyO3NoteEvent {
    fn into(self) -> NoteEvent<()> {
        match self {
            PyO3NoteEvent::NoteOn(x) => NoteEvent::NoteOn {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                velocity: x.velocity,
            },
            PyO3NoteEvent::NoteOff(x) => NoteEvent::NoteOff {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                velocity: x.velocity,
            },
            PyO3NoteEvent::Choke(x) => NoteEvent::Choke {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
            },
            PyO3NoteEvent::VoiceTerminated(x) => NoteEvent::VoiceTerminated {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
            },
            PyO3NoteEvent::PolyModulation(x) => NoteEvent::PolyModulation {
                timing: x.timing,
                voice_id: x.voice_id,
                poly_modulation_id: x.poly_modulation_id,
                normalized_offset: x.normalized_offset,
            },
            PyO3NoteEvent::MonoAutomation(x) => NoteEvent::MonoAutomation {
                timing: x.timing,
                poly_modulation_id: x.poly_modulation_id,
                normalized_value: x.normalized_value,
            },
            PyO3NoteEvent::PolyPressure(x) => NoteEvent::PolyPressure {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                pressure: x.pressure,
            },
            PyO3NoteEvent::PolyVolume(x) => NoteEvent::PolyVolume {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                gain: x.gain,
            },
            PyO3NoteEvent::PolyPan(x) => NoteEvent::PolyPan {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                pan: x.pan,
            },
            PyO3NoteEvent::PolyTuning(x) => NoteEvent::PolyTuning {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                tuning: x.tuning,
            },
            PyO3NoteEvent::PolyVibrato(x) => NoteEvent::PolyVibrato {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                vibrato: x.vibrato,
            },
            PyO3NoteEvent::PolyExpression(x) => NoteEvent::PolyExpression {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                expression: x.expression,
            },
            PyO3NoteEvent::PolyBrightness(x) => NoteEvent::PolyBrightness {
                timing: x.timing,
                voice_id: x.voice_id,
                channel: x.channel,
                note: x.note,
                brightness: x.brightness,
            },
            PyO3NoteEvent::MidiChannelPressure(x) => NoteEvent::MidiChannelPressure {
                timing: x.timing,
                channel: x.channel,
                pressure: x.pressure,
            },
            PyO3NoteEvent::MidiPitchBend(x) => NoteEvent::MidiPitchBend {
                timing: x.timing,
                channel: x.channel,
                value: x.value,
            },
            PyO3NoteEvent::MidiCC(x) => NoteEvent::MidiCC {
                timing: x.timing,
                channel: x.channel,
                cc: x.cc,
                value: x.value,
            },
            PyO3NoteEvent::MidiProgramChange(x) => NoteEvent::MidiProgramChange {
                timing: x.timing,
                channel: x.channel,
                program: x.program,
            },
            PyO3NoteEvent::MidiSysEx(x) => NoteEvent::MidiSysEx {
                timing: x.timing,
                message: (),
            },
        }
    }
}

impl From<NoteEvent<()>> for PyO3NoteEvent {
    fn from(value: NoteEvent<()>) -> Self {
        match value {
            NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            } => PyO3NoteEvent::NoteOn(NoteOn {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            }),
            NoteEvent::NoteOff {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            } => PyO3NoteEvent::NoteOff(NoteOff {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            }),
            NoteEvent::Choke {
                timing,
                voice_id,
                channel,
                note,
            } => PyO3NoteEvent::Choke(Choke {
                timing,
                voice_id,
                channel,
                note,
            }),
            NoteEvent::VoiceTerminated {
                timing,
                voice_id,
                channel,
                note,
            } => PyO3NoteEvent::VoiceTerminated(VoiceTerminated {
                timing,
                voice_id,
                channel,
                note,
            }),
            NoteEvent::PolyModulation {
                timing,
                voice_id,
                poly_modulation_id,
                normalized_offset,
            } => PyO3NoteEvent::PolyModulation(PolyModulation {
                timing,
                voice_id,
                poly_modulation_id,
                normalized_offset,
            }),
            NoteEvent::MonoAutomation {
                timing,
                poly_modulation_id,
                normalized_value,
            } => PyO3NoteEvent::MonoAutomation(MonoAutomation {
                timing,
                poly_modulation_id,
                normalized_value,
            }),
            NoteEvent::PolyPressure {
                timing,
                voice_id,
                channel,
                note,
                pressure,
            } => PyO3NoteEvent::PolyPressure(PolyPressure {
                timing,
                voice_id,
                channel,
                note,
                pressure,
            }),
            NoteEvent::PolyVolume {
                timing,
                voice_id,
                channel,
                note,
                gain,
            } => PyO3NoteEvent::PolyVolume(PolyVolume {
                timing,
                voice_id,
                channel,
                note,
                gain,
            }),
            NoteEvent::PolyPan {
                timing,
                voice_id,
                channel,
                note,
                pan,
            } => PyO3NoteEvent::PolyPan(PolyPan {
                timing,
                voice_id,
                channel,
                note,
                pan,
            }),
            NoteEvent::PolyTuning {
                timing,
                voice_id,
                channel,
                note,
                tuning,
            } => PyO3NoteEvent::PolyTuning(PolyTuning {
                timing,
                voice_id,
                channel,
                note,
                tuning,
            }),
            NoteEvent::PolyVibrato {
                timing,
                voice_id,
                channel,
                note,
                vibrato,
            } => PyO3NoteEvent::PolyVibrato(PolyVibrato {
                timing,
                voice_id,
                channel,
                note,
                vibrato,
            }),
            NoteEvent::PolyExpression {
                timing,
                voice_id,
                channel,
                note,
                expression,
            } => PyO3NoteEvent::PolyExpression(PolyExpression {
                timing,
                voice_id,
                channel,
                note,
                expression,
            }),
            NoteEvent::PolyBrightness {
                timing,
                voice_id,
                channel,
                note,
                brightness,
            } => PyO3NoteEvent::PolyBrightness(PolyBrightness {
                timing,
                voice_id,
                channel,
                note,
                brightness,
            }),
            NoteEvent::MidiChannelPressure {
                timing,
                channel,
                pressure,
            } => PyO3NoteEvent::MidiChannelPressure(MidiChannelPressure {
                timing,
                channel,
                pressure,
            }),
            NoteEvent::MidiPitchBend {
                timing,
                channel,
                value,
            } => PyO3NoteEvent::MidiPitchBend(MidiPitchBend {
                timing,
                channel,
                value,
            }),
            NoteEvent::MidiCC {
                timing,
                channel,
                cc,
                value,
            } => PyO3NoteEvent::MidiCC(MidiCC {
                timing,
                channel,
                cc,
                value,
            }),
            NoteEvent::MidiProgramChange {
                timing,
                channel,
                program,
            } => PyO3NoteEvent::MidiProgramChange(MidiProgramChange {
                timing,
                channel,
                program,
            }),
            NoteEvent::MidiSysEx { timing, .. } => PyO3NoteEvent::MidiSysEx(MidiSysEx { timing }),
            _ => panic!("Unsupported note event"),
        }
    }
}

#[cfg(test)]
mod test {
    use pyo3::{FromPyObject, Python, ToPyObject};

    use super::*;

    pub fn test_event_tag_serde(py: Python, e: PyO3NoteEvent) {
        let o = e.to_object(py);
        let g: PyO3NoteEvent = o.extract(py).unwrap();
        assert_eq!(e, g);
    }

    #[test]
    pub fn test_event_tags_serde() {
        Python::with_gil(|py| {
            test_event_tag_serde(
                py,
                PyO3NoteEvent::NoteOn(NoteOn {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    velocity: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::NoteOff(NoteOff {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    velocity: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::Choke(Choke {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::VoiceTerminated(VoiceTerminated {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyModulation(PolyModulation {
                    timing: 0,
                    voice_id: 0,
                    poly_modulation_id: 0,
                    normalized_offset: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::MonoAutomation(MonoAutomation {
                    timing: 0,
                    poly_modulation_id: 0,
                    normalized_value: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyPressure(PolyPressure {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    pressure: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyVolume(PolyVolume {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    gain: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyPan(PolyPan {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    pan: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyTuning(PolyTuning {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    tuning: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyVibrato(PolyVibrato {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    vibrato: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyExpression(PolyExpression {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    expression: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::PolyBrightness(PolyBrightness {
                    timing: 0,
                    voice_id: Some(0),
                    channel: 0,
                    note: 0,
                    brightness: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::MidiChannelPressure(MidiChannelPressure {
                    timing: 0,
                    channel: 0,
                    pressure: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::MidiPitchBend(MidiPitchBend {
                    timing: 0,
                    channel: 0,
                    value: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::MidiCC(MidiCC {
                    timing: 0,
                    channel: 0,
                    cc: 0,
                    value: 0.0,
                }),
            );

            test_event_tag_serde(
                py,
                PyO3NoteEvent::MidiProgramChange(MidiProgramChange {
                    timing: 0,
                    channel: 0,
                    program: 0,
                }),
            );

            test_event_tag_serde(py, PyO3NoteEvent::MidiSysEx(MidiSysEx { timing: 0 }));
        });
    }
}
