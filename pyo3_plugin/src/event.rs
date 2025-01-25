use nih_plug::midi::NoteEvent;
use std::convert::Infallible;

use pyo3::prelude::PyModule;

use pyo3::{
    pyclass, pymethods, pymodule, Bound, FromPyObject, IntoPyObject, IntoPyObjectExt, PyAny,
    PyResult, Python,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct NoteOn {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub velocity: f32,
}

#[pymethods]
impl NoteOn {
    #[new]
    #[pyo3(signature = (timing, channel, note, velocity, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, velocity: f32, voice_id: Option<i32>) -> Self {
        NoteOn {
            timing,
            channel,
            note,
            velocity,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct NoteOff {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub velocity: f32,
}

#[pymethods]
impl NoteOff {
    #[new]
    #[pyo3(signature = (timing, channel, note, velocity, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, velocity: f32, voice_id: Option<i32>) -> Self {
        NoteOff {
            timing,
            channel,
            note,
            velocity,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct Choke {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
}

#[pymethods]
impl Choke {
    #[new]
    #[pyo3(signature = (timing, channel, note, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, voice_id: Option<i32>) -> Self {
        Choke {
            timing,
            channel,
            note,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct VoiceTerminated {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
}

#[pymethods]
impl VoiceTerminated {
    #[new]
    #[pyo3(signature = (timing, channel, note, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, voice_id: Option<i32>) -> Self {
        VoiceTerminated {
            timing,
            channel,
            note,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyModulation {
    pub timing: u32,
    pub voice_id: i32,
    pub poly_modulation_id: u32,
    pub normalized_offset: f32,
}

#[pymethods]
impl PolyModulation {
    #[new]
    pub fn new(
        timing: u32,
        voice_id: i32,
        poly_modulation_id: u32,
        normalized_offset: f32,
    ) -> Self {
        PolyModulation {
            timing,
            voice_id,
            poly_modulation_id,
            normalized_offset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct MonoAutomation {
    pub timing: u32,
    pub poly_modulation_id: u32,
    pub normalized_value: f32,
}

#[pymethods]
impl MonoAutomation {
    #[new]
    pub fn new(timing: u32, poly_modulation_id: u32, normalized_value: f32) -> Self {
        MonoAutomation {
            timing,
            poly_modulation_id,
            normalized_value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyPressure {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub pressure: f32,
}

#[pymethods]
impl PolyPressure {
    #[new]
    #[pyo3(signature = (timing, channel, note, pressure, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, pressure: f32, voice_id: Option<i32>) -> Self {
        PolyPressure {
            timing,
            channel,
            note,
            pressure,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyVolume {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub gain: f32,
}

#[pymethods]
impl PolyVolume {
    #[new]
    #[pyo3(signature = (timing, channel, note, gain, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, gain: f32, voice_id: Option<i32>) -> Self {
        PolyVolume {
            timing,
            channel,
            note,
            gain,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyPan {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub pan: f32,
}

#[pymethods]
impl PolyPan {
    #[new]
    #[pyo3(signature = (timing, channel, note, pan, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, pan: f32, voice_id: Option<i32>) -> Self {
        PolyPan {
            timing,
            channel,
            note,
            pan,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyTuning {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub tuning: f32,
}

#[pymethods]
impl PolyTuning {
    #[new]
    #[pyo3(signature = (timing, channel, note, tuning, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, tuning: f32, voice_id: Option<i32>) -> Self {
        PolyTuning {
            timing,
            channel,
            note,
            tuning,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyVibrato {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub vibrato: f32,
}

#[pymethods]
impl PolyVibrato {
    #[new]
    #[pyo3(signature = (timing, channel, note, vibrato, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, vibrato: f32, voice_id: Option<i32>) -> Self {
        PolyVibrato {
            timing,
            channel,
            note,
            vibrato,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyExpression {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub expression: f32,
}

#[pymethods]
impl PolyExpression {
    #[new]
    #[pyo3(signature = (timing, channel, note, expression, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, expression: f32, voice_id: Option<i32>) -> Self {
        PolyExpression {
            timing,
            channel,
            note,
            expression,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct PolyBrightness {
    pub timing: u32,
    pub voice_id: Option<i32>,
    pub channel: u8,
    pub note: u8,
    pub brightness: f32,
}

#[pymethods]
impl PolyBrightness {
    #[new]
    #[pyo3(signature = (timing, channel, note, brightness, voice_id=None))]
    pub fn new(timing: u32, channel: u8, note: u8, brightness: f32, voice_id: Option<i32>) -> Self {
        PolyBrightness {
            timing,
            channel,
            note,
            brightness,
            voice_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct MidiChannelPressure {
    pub timing: u32,
    pub channel: u8,
    pub pressure: f32,
}

#[pymethods]
impl MidiChannelPressure {
    #[new]
    pub fn new(timing: u32, channel: u8, pressure: f32) -> Self {
        MidiChannelPressure {
            timing,
            channel,
            pressure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct MidiPitchBend {
    pub timing: u32,
    pub channel: u8,
    pub value: f32,
}

#[pymethods]
impl MidiPitchBend {
    #[new]
    pub fn new(timing: u32, channel: u8, value: f32) -> Self {
        MidiPitchBend {
            timing,
            channel,
            value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct MidiCC {
    pub timing: u32,
    pub channel: u8,
    pub cc: u8,
    pub value: f32,
}

#[pymethods]
impl MidiCC {
    #[new]
    pub fn new(timing: u32, channel: u8, cc: u8, value: f32) -> Self {
        MidiCC {
            timing,
            channel,
            cc,
            value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct MidiProgramChange {
    pub timing: u32,
    pub channel: u8,
    pub program: u8,
}

#[pymethods]
impl MidiProgramChange {
    #[new]
    pub fn new(timing: u32, channel: u8, program: u8) -> Self {
        MidiProgramChange {
            timing,
            channel,
            program,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(get_all, set_all)]
pub struct MidiSysEx {
    pub timing: u32,
}

#[pymethods]
impl MidiSysEx {
    #[new]
    pub fn new(timing: u32) -> Self {
        MidiSysEx { timing }
    }
}

#[pymodule]
pub fn add_pyo3_note_events(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    use pyo3::prelude::PyModuleMethods;
    m.add_class::<NoteOn>()?;
    m.add_class::<NoteOff>()?;
    m.add_class::<Choke>()?;
    m.add_class::<VoiceTerminated>()?;
    m.add_class::<PolyModulation>()?;
    m.add_class::<MonoAutomation>()?;
    m.add_class::<PolyPressure>()?;
    m.add_class::<PolyVolume>()?;
    m.add_class::<PolyPan>()?;
    m.add_class::<PolyTuning>()?;
    m.add_class::<PolyVibrato>()?;
    m.add_class::<PolyExpression>()?;
    m.add_class::<PolyBrightness>()?;
    m.add_class::<MidiChannelPressure>()?;
    m.add_class::<MidiPitchBend>()?;
    m.add_class::<MidiCC>()?;
    m.add_class::<MidiProgramChange>()?;
    m.add_class::<MidiSysEx>()?;
    Ok(())
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

fn conv<'py, A>(a: A, py: Python<'py>) -> Bound<'py, PyAny>
where
    A: for<'a> IntoPyObject<'a>,
{
    a.into_bound_py_any(py).unwrap()
}

impl<'py> IntoPyObject<'py> for PyO3NoteEvent {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = Infallible;
    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(match self {
            PyO3NoteEvent::NoteOn(x) => conv(x, py),
            PyO3NoteEvent::NoteOff(x) => conv(x, py),
            PyO3NoteEvent::Choke(x) => conv(x, py),
            PyO3NoteEvent::VoiceTerminated(x) => conv(x, py),
            PyO3NoteEvent::PolyModulation(x) => conv(x, py),
            PyO3NoteEvent::MonoAutomation(x) => conv(x, py),
            PyO3NoteEvent::PolyPressure(x) => conv(x, py),
            PyO3NoteEvent::PolyVolume(x) => conv(x, py),
            PyO3NoteEvent::PolyPan(x) => conv(x, py),
            PyO3NoteEvent::PolyTuning(x) => conv(x, py),
            PyO3NoteEvent::PolyVibrato(x) => conv(x, py),
            PyO3NoteEvent::PolyExpression(x) => conv(x, py),
            PyO3NoteEvent::PolyBrightness(x) => conv(x, py),
            PyO3NoteEvent::MidiChannelPressure(x) => conv(x, py),
            PyO3NoteEvent::MidiPitchBend(x) => conv(x, py),
            PyO3NoteEvent::MidiCC(x) => conv(x, py),
            PyO3NoteEvent::MidiProgramChange(x) => conv(x, py),
            PyO3NoteEvent::MidiSysEx(x) => conv(x, py),
        })
    }
}

impl From<PyO3NoteEvent> for NoteEvent<()> {
    fn from(value: PyO3NoteEvent) -> Self {
        match value {
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
    use pyo3::{Python, ToPyObject};

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
