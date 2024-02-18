// copy of https://github.com/robbert-vdh/nih-plug/blob/master/src/midi.rs#L55
// for convertion

use crate::PyO3Plugin;
use nih_plug::midi::NoteEvent;
use nih_plug::plugin::Plugin;
use nih_plug::prelude::PluginNoteEvent;
use pyo3::types::{IntoPyDict, PyFloat, PyInt};
use pyo3::{FromPyObject, IntoPy, PyAny, PyObject, PyResult, Python, ToPyObject};

pub fn pyo3_note_event_to_note_event(note_event: PyO3NoteEvent) -> NoteEvent<()> {
    match note_event {
        PyO3NoteEvent::NoteOn {
            timing,
            voice_id,
            channel,
            note,
            velocity,
            ..
        } => NoteEvent::NoteOn {
            timing,
            voice_id,
            channel,
            note,
            velocity,
        },
        PyO3NoteEvent::NoteOff {
            timing,
            voice_id,
            channel,
            note,
            velocity,
            ..
        } => NoteEvent::NoteOff {
            timing,
            voice_id,
            channel,
            note,
            velocity,
        },
        PyO3NoteEvent::Choke {
            timing,
            voice_id,
            channel,
            note,
        } => NoteEvent::Choke {
            timing,
            voice_id,
            channel,
            note,
        },
        PyO3NoteEvent::VoiceTerminated {
            timing,
            voice_id,
            channel,
            note,
        } => NoteEvent::VoiceTerminated {
            timing,
            voice_id,
            channel,
            note,
        },
        PyO3NoteEvent::PolyModulation {
            timing,
            voice_id,
            poly_modulation_id,
            normalized_offset,
        } => NoteEvent::PolyModulation {
            timing,
            voice_id,
            poly_modulation_id,
            normalized_offset,
        },
        PyO3NoteEvent::MonoAutomation {
            timing,
            poly_modulation_id,
            normalized_value,
        } => NoteEvent::MonoAutomation {
            timing,
            poly_modulation_id,
            normalized_value,
        },
        PyO3NoteEvent::PolyPressure {
            timing,
            voice_id,
            channel,
            note,
            pressure,
        } => NoteEvent::PolyPressure {
            timing,
            voice_id,
            channel,
            note,
            pressure,
        },
        PyO3NoteEvent::PolyVolume {
            timing,
            voice_id,
            channel,
            note,
            gain,
        } => NoteEvent::PolyVolume {
            timing,
            voice_id,
            channel,
            note,
            gain,
        },
        PyO3NoteEvent::PolyPan {
            timing,
            voice_id,
            channel,
            note,
            pan,
        } => NoteEvent::PolyPan {
            timing,
            voice_id,
            channel,
            note,
            pan,
        },
        PyO3NoteEvent::PolyTuning {
            timing,
            voice_id,
            channel,
            note,
            tuning,
        } => NoteEvent::PolyTuning {
            timing,
            voice_id,
            channel,
            note,
            tuning,
        },

        PyO3NoteEvent::PolyVibrato {
            timing,
            voice_id,
            channel,
            note,
            vibrato,
        } => NoteEvent::PolyVibrato {
            timing,
            voice_id,
            channel,
            note,
            vibrato,
        },
        PyO3NoteEvent::PolyExpression {
            timing,
            voice_id,
            channel,
            note,
            expression,
        } => NoteEvent::PolyExpression {
            timing,
            voice_id,
            channel,
            note,
            expression,
        },
        PyO3NoteEvent::PolyBrightness {
            timing,
            voice_id,
            channel,
            note,
            brightness,
        } => NoteEvent::PolyBrightness {
            timing,
            voice_id,
            channel,
            note,
            brightness,
        },
        PyO3NoteEvent::MidiChannelPressure {
            timing,
            channel,
            pressure,
        } => NoteEvent::MidiChannelPressure {
            timing,
            channel,
            pressure,
        },
        PyO3NoteEvent::MidiPitchBend {
            timing,
            channel,
            value,
        } => NoteEvent::MidiPitchBend {
            timing,
            channel,
            value,
        },
        PyO3NoteEvent::MidiCC {
            timing,
            channel,
            cc,
            value,
        } => NoteEvent::MidiCC {
            timing,
            channel,
            cc,
            value,
        },
        PyO3NoteEvent::MidiProgramChange {
            timing,
            channel,
            program,
        } => NoteEvent::MidiProgramChange {
            timing,
            channel,
            program,
        },
        PyO3NoteEvent::MidiSysEx { timing } => NoteEvent::MidiSysEx {
            timing,
            message: (),
        },
    }
}

pub fn note_event_to_pyo3_note_event(note_event: NoteEvent<()>) -> PyO3NoteEvent {
    match note_event {
        NoteEvent::NoteOn {
            timing,
            voice_id,
            channel,
            note,
            velocity,
        } => PyO3NoteEvent::NoteOn {
            pyo3_tag: true,
            timing,
            voice_id,
            channel,
            note,
            velocity,
        },
        NoteEvent::NoteOff {
            timing,
            voice_id,
            channel,
            note,
            velocity,
        } => PyO3NoteEvent::NoteOff {
            pyo3_tag: true,
            timing,
            voice_id,
            channel,
            note,
            velocity,
        },
        NoteEvent::Choke {
            timing,
            voice_id,
            channel,
            note,
        } => PyO3NoteEvent::Choke {
            timing,
            voice_id,
            channel,
            note,
        },
        NoteEvent::VoiceTerminated {
            timing,
            voice_id,
            channel,
            note,
        } => PyO3NoteEvent::VoiceTerminated {
            timing,
            voice_id,
            channel,
            note,
        },
        NoteEvent::PolyModulation {
            timing,
            voice_id,
            poly_modulation_id,
            normalized_offset,
        } => PyO3NoteEvent::PolyModulation {
            timing,
            voice_id,
            poly_modulation_id,
            normalized_offset,
        },
        NoteEvent::MonoAutomation {
            timing,
            poly_modulation_id,
            normalized_value,
        } => PyO3NoteEvent::MonoAutomation {
            timing,
            poly_modulation_id,
            normalized_value,
        },
        NoteEvent::PolyPressure {
            timing,
            voice_id,
            channel,
            note,
            pressure,
        } => PyO3NoteEvent::PolyPressure {
            timing,
            voice_id,
            channel,
            note,
            pressure,
        },
        NoteEvent::PolyVolume {
            timing,
            voice_id,
            channel,
            note,
            gain,
        } => PyO3NoteEvent::PolyVolume {
            timing,
            voice_id,
            channel,
            note,
            gain,
        },
        NoteEvent::PolyPan {
            timing,
            voice_id,
            channel,
            note,
            pan,
        } => PyO3NoteEvent::PolyPan {
            timing,
            voice_id,
            channel,
            note,
            pan,
        },
        NoteEvent::PolyTuning {
            timing,
            voice_id,
            channel,
            note,
            tuning,
        } => PyO3NoteEvent::PolyTuning {
            timing,
            voice_id,
            channel,
            note,
            tuning,
        },
        NoteEvent::PolyVibrato {
            timing,
            voice_id,
            channel,
            note,
            vibrato,
        } => PyO3NoteEvent::PolyVibrato {
            timing,
            voice_id,
            channel,
            note,
            vibrato,
        },
        NoteEvent::PolyExpression {
            timing,
            voice_id,
            channel,
            note,
            expression,
        } => PyO3NoteEvent::PolyExpression {
            timing,
            voice_id,
            channel,
            note,
            expression,
        },
        NoteEvent::PolyBrightness {
            timing,
            voice_id,
            channel,
            note,
            brightness,
        } => PyO3NoteEvent::PolyBrightness {
            timing,
            voice_id,
            channel,
            note,
            brightness,
        },
        NoteEvent::MidiChannelPressure {
            timing,
            channel,
            pressure,
        } => PyO3NoteEvent::MidiChannelPressure {
            timing,
            channel,
            pressure,
        },
        NoteEvent::MidiPitchBend {
            timing,
            channel,
            value,
        } => PyO3NoteEvent::MidiPitchBend {
            timing,
            channel,
            value,
        },
        NoteEvent::MidiCC {
            timing,
            channel,
            cc,
            value,
        } => PyO3NoteEvent::MidiCC {
            timing,
            channel,
            cc,
            value,
        },
        NoteEvent::MidiProgramChange {
            timing,
            channel,
            program,
        } => PyO3NoteEvent::MidiProgramChange {
            timing,
            channel,
            program,
        },
        NoteEvent::MidiSysEx { timing, .. } => PyO3NoteEvent::MidiSysEx { timing },
        _ => panic!("Unsupported note event"),
    }
}

impl ToPyObject for PyO3NoteEvent {
    fn to_object(&self, py: Python) -> PyObject {
        match self {
            PyO3NoteEvent::NoteOn {
                pyo3_tag,
                timing,
                voice_id,
                channel,
                note,
                velocity,
            } => {
                let timing = timing.into_py(py);
                let voice_id = voice_id.into_py(py);
                let channel = channel.into_py(py);
                let note = note.into_py(py);
                let velocity = velocity.into_py(py);
                let dict = [
                    ("NoteOn", true.into_py(py)),
                    ("timing", timing),
                    ("voice_id", voice_id),
                    ("channel", channel),
                    ("note", note),
                    ("velocity", velocity),
                ]
                .into_py_dict(py);
                dict.into()
            }
            PyO3NoteEvent::NoteOff {
                pyo3_tag,
                timing,
                voice_id,
                channel,
                note,
                velocity,
            } => {
                let timing = timing.into_py(py);
                let voice_id = voice_id.into_py(py);
                let channel = channel.into_py(py);
                let note = note.into_py(py);
                let velocity = velocity.into_py(py);
                let dict = [
                    ("NoteOff", true.into_py(py)),
                    ("timing", timing),
                    ("voice_id", voice_id),
                    ("channel", channel),
                    ("note", note),
                    ("velocity", velocity),
                ]
                .into_py_dict(py);
                dict.into()
            }
            _ => todo!(),
        }
    }
}

impl IntoPy<PyObject> for PyO3NoteEvent {
    fn into_py(self, py: Python) -> PyObject {
        self.into_py(py)
    }
}

//
// copy of nih_plug::prelude::PluginNoteEvent
//
#[derive(Debug, Clone, Copy, PartialEq, FromPyObject)]
#[non_exhaustive]
pub enum PyO3NoteEvent {
    /// A note on event, available on [`MidiConfig::Basic`] and up.
    NoteOn {
        #[pyo3(attribute("NoteOn"))]
        pyo3_tag: bool,

        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's velocity, in `[0, 1]`. Some plugin APIs may allow higher precision than the
        /// 128 levels available in MIDI.
        velocity: f32,
    },
    /// A note off event, available on [`MidiConfig::Basic`] and up. Bitwig Studio does not provide
    /// a voice ID for this event.
    NoteOff {
        #[pyo3(attribute("NoteOff"))]
        pyo3_tag: bool,

        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's velocity, in `[0, 1]`. Some plugin APIs may allow higher precision than the
        /// 128 levels available in MIDI.
        velocity: f32,
    },
    /// A note choke event, available on [`MidiConfig::Basic`] and up. When the host sends this to
    /// the plugin, it indicates that a voice or all sound associated with a note should immediately
    /// stop playing.
    Choke {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
    },

    /// Sent by the plugin to the host to indicate that a voice has ended. This **needs** to be sent
    /// when a voice terminates when using polyphonic modulation. Otherwise you can ignore this
    /// event.
    VoiceTerminated {
        timing: u32,
        /// The voice's unique identifier. Setting this allows a single voice to be terminated if
        /// the plugin allows multiple overlapping voices for a single key.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
    },
    /// A polyphonic modulation event, available on [`MidiConfig::Basic`] and up. This will only be
    /// sent for parameters that were decorated with the `.with_poly_modulation_id()` modifier, and
    /// only by supported hosts. This event contains a _normalized offset value_ for the parameter's
    /// current, **unmodulated** value. That is, an offset for the current value before monophonic
    /// modulation is applied, as polyphonic modulation overrides monophonic modulation. There are
    /// multiple ways to incorporate this polyphonic modulation into a synthesizer, but a simple way
    /// to incorporate this would work as follows:
    ///
    /// - By default, a voice uses the parameter's global value, which may or may not include
    ///   monophonic modulation. This is `parameter.value` for unsmoothed parameters, and smoothed
    ///   parameters should use block smoothing so the smoothed values can be reused by multiple
    ///   voices.
    /// - If a `PolyModulation` event is emitted for the voice, that voice should use the the
    ///   _normalized offset_ contained within the event to compute the voice's modulated value and
    ///   use that in place of the global value.
    ///   - This value can be obtained by calling `param.preview_plain(param.normalized_value() +
    ///     event.normalized_offset)`. These functions automatically clamp the values as necessary.
    ///   - If the parameter uses smoothing, then the parameter's smoother can be copied to the
    ///     voice. [`Smoother::set_target()`][crate::prelude::Smoother::set_target()] can then be
    ///     used to have the smoother use the modulated value.
    ///   - One caveat with smoothing is that copying the smoother like this only works correctly if it last
    ///     produced a value during the sample before the `PolyModulation` event. Otherwise there
    ///     may still be an audible jump in parameter values. A solution for this would be to first
    ///     call the [`Smoother::reset()`][crate::prelude::Smoother::reset()] with the current
    ///     sample's global value before calling `set_target()`.
    ///   - Finally, if the polyphonic modulation happens on the same sample as the `NoteOn` event,
    ///     then the smoothing should not start at the current global value. In this case, `reset()`
    ///     should be called with the voice's modulated value.
    /// - If a `MonoAutomation` event is emitted for a parameter, then the values or target values
    ///   (if the parameter uses smoothing) for all voices must be updated. The normalized value
    ///   from the `MonoAutomation` and the voice's normalized modulation offset must be added and
    ///   converted back to a plain value. This value can be used directly for unsmoothed
    ///   parameters, or passed to `set_target()` for smoothed parameters. The global value will
    ///   have already been updated, so this event only serves as a notification to update
    ///   polyphonic modulation.
    /// - When a voice ends, either because the amplitude envelope has hit zero or because the voice
    ///   was stolen, the plugin must send a `VoiceTerminated` to the host to let it know that it
    ///   can reuse the resources it used to modulate the value.
    PolyModulation {
        timing: u32,
        /// The identifier of the voice this polyphonic modulation event should affect. This voice
        /// should use the values from this and subsequent polyphonic modulation events instead of
        /// the global value.
        voice_id: i32,
        /// The ID that was set for the modulated parameter using the `.with_poly_modulation_id()`
        /// method.
        poly_modulation_id: u32,
        /// The normalized offset value. See the event's docstring for more information.
        normalized_offset: f32,
    },
    /// A notification to inform the plugin that a polyphonically modulated parameter has received a
    /// new automation value. This is used in conjunction with the `PolyModulation` event. See that
    /// event's documentation for more details. The parameter's global value has already been
    /// updated when this event is emitted.
    MonoAutomation {
        timing: u32,
        /// The ID that was set for the modulated parameter using the `.with_poly_modulation_id()`
        /// method.
        poly_modulation_id: u32,
        /// The parameter's new normalized value. This needs to be added to a voice's normalized
        /// offset to get that voice's modulated normalized value. See the `PolyModulation` event's
        /// docstring for more information.
        normalized_value: f32,
    },

    /// A polyphonic note pressure/aftertouch event, available on [`MidiConfig::Basic`] and up. Not
    /// all hosts may support polyphonic aftertouch.
    ///
    /// # Note
    ///
    /// When implementing MPE support you should use MIDI channel pressure instead as polyphonic key
    /// pressure + MPE is undefined as per the MPE specification. Or as a more generic catch all,
    /// you may manually combine the polyphonic key pressure and MPE channel pressure.
    PolyPressure {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's pressure, in `[0, 1]`.
        pressure: f32,
    },
    /// A volume expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may
    /// support these expressions.
    PolyVolume {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's voltage gain ratio, where 1.0 is unity gain.
        gain: f32,
    },
    /// A panning expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may
    /// support these expressions.
    PolyPan {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's panning from, in `[-1, 1]`, with -1 being panned hard left, and 1
        /// being panned hard right.
        pan: f32,
    },
    /// A tuning expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may support
    /// these expressions.
    PolyTuning {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's tuning in semitones, in `[-128, 128]`.
        tuning: f32,
    },
    /// A vibrato expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may support
    /// these expressions.
    PolyVibrato {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's vibrato amount, in `[0, 1]`.
        vibrato: f32,
    },
    /// A expression expression (yes, expression expression) event, available on
    /// [`MidiConfig::Basic`] and up. Not all hosts may support these expressions.
    PolyExpression {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's expression amount, in `[0, 1]`.
        expression: f32,
    },
    /// A brightness expression event, available on [`MidiConfig::Basic`] and up. Not all hosts may support
    /// these expressions.
    PolyBrightness {
        timing: u32,
        /// A unique identifier for this note, if available. Using this to refer to a note is
        /// required when allowing overlapping voices for CLAP plugins.
        voice_id: Option<i32>,
        /// The note's channel, in `0..16`.
        channel: u8,
        /// The note's MIDI key number, in `0..128`.
        note: u8,
        /// The note's brightness amount, in `[0, 1]`.
        brightness: f32,
    },
    /// A MIDI channel pressure event, available on [`MidiConfig::MidiCCs`] and up.
    MidiChannelPressure {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The pressure, normalized to `[0, 1]` to match the poly pressure event.
        pressure: f32,
    },
    /// A MIDI pitch bend, available on [`MidiConfig::MidiCCs`] and up.
    MidiPitchBend {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The pressure, normalized to `[0, 1]`. `0.5` means no pitch bend.
        value: f32,
    },
    /// A MIDI control change event, available on [`MidiConfig::MidiCCs`] and up.
    ///
    /// # Note
    ///
    /// The wrapper does not perform any special handling for two message 14-bit CCs (where the CC
    /// number is in `0..32`, and the next CC is that number plus 32) or for four message RPN
    /// messages. For now you will need to handle these CCs yourself.
    MidiCC {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The control change number. See [`control_change`] for a list of CC numbers.
        cc: u8,
        /// The CC's value, normalized to `[0, 1]`. Multiply by 127 to get the original raw value.
        value: f32,
    },
    /// A MIDI program change event, available on [`MidiConfig::MidiCCs`] and up. VST3 plugins
    /// cannot receive these events.
    MidiProgramChange {
        timing: u32,
        /// The affected channel, in `0..16`.
        channel: u8,
        /// The program number, in `0..128`.
        program: u8,
    },
    /// A MIDI SysEx message supported by the plugin's `SysExMessage` type, available on
    /// [`MidiConfig::Basic`] and up. If the conversion from the raw byte array fails (e.g. the
    /// plugin doesn't support this kind of message), then this will be logged during debug builds
    /// of the plugin, and no event is emitted.
    MidiSysEx { timing: u32 },
}
