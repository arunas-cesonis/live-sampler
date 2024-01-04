use nih_plug::prelude::NoteEvent;
use nih_plug::wrapper::vst3::vst3_sys::vst::NoteOffEvent;
use std::fmt::Debug;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct Note {
    pub note: u8,
    pub channel: u8,
}
impl From<u64> for Note {
    fn from(note: u64) -> Self {
        Self {
            note: (note & 0x7F) as u8,
            channel: ((note >> 8) & 0x0F) as u8,
        }
    }
}
impl Into<u64> for Note {
    fn into(self) -> u64 {
        (self.note as u64) | ((self.channel as u64) << 8)
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum NoteState {
    On,
    Off,
}

pub fn note_from_event<S>(ev: &NoteEvent<S>) -> Option<(Note, NoteState)> {
    Some(match ev {
        NoteEvent::NoteOn { note, channel, .. } => (
            Note {
                note: *note,
                channel: *channel,
            },
            NoteState::On,
        ),
        NoteEvent::NoteOff { note, channel, .. } => (
            Note {
                note: *note,
                channel: *channel,
            },
            NoteState::Off,
        ),
        // TODO: check how important other events are and if they contain note information
        //NoteEvent::Choke { note, channel, .. } => Note {
        //    note: *note,
        //    channel: *channel,
        //},
        _ => return None,
    })
}

pub fn is_note_on<S>(event: &NoteEvent<S>) -> bool {
    match event {
        NoteEvent::NoteOn { .. } => true,
        _ => false,
    }
}

pub fn is_note_off<S>(event: &NoteEvent<S>) -> bool {
    match event {
        NoteEvent::NoteOff { .. } => true,
        _ => false,
    }
}

pub fn set_event_timing<S>(mut ev: NoteEvent<S>, value: u32) -> NoteEvent<S> {
    set_event_timing_mut(&mut ev, value);
    ev
}

pub fn set_event_timing_mut<S>(ev: &mut NoteEvent<S>, value: u32) {
    match ev {
        NoteEvent::NoteOn { timing, .. } => *timing = value,
        NoteEvent::NoteOff { timing, .. } => *timing = value,
        NoteEvent::Choke { timing, .. } => *timing = value,
        NoteEvent::VoiceTerminated { timing, .. } => *timing = value,
        NoteEvent::PolyModulation { timing, .. } => *timing = value,
        NoteEvent::MonoAutomation { timing, .. } => *timing = value,
        NoteEvent::PolyPressure { timing, .. } => *timing = value,
        NoteEvent::PolyVolume { timing, .. } => *timing = value,
        NoteEvent::PolyPan { timing, .. } => *timing = value,
        NoteEvent::PolyTuning { timing, .. } => *timing = value,
        NoteEvent::PolyVibrato { timing, .. } => *timing = value,
        NoteEvent::PolyExpression { timing, .. } => *timing = value,
        NoteEvent::PolyBrightness { timing, .. } => *timing = value,
        NoteEvent::MidiChannelPressure { timing, .. } => *timing = value,
        NoteEvent::MidiPitchBend { timing, .. } => *timing = value,
        NoteEvent::MidiCC { timing, .. } => *timing = value,
        NoteEvent::MidiProgramChange { timing, .. } => *timing = value,
        NoteEvent::MidiSysEx { timing, .. } => *timing = value,
        _ => (),
    }
}

pub fn get_event_timing<S: Debug>(ev: &NoteEvent<S>) -> u32 {
    *(match ev {
        NoteEvent::NoteOn { timing, .. } => timing,
        NoteEvent::NoteOff { timing, .. } => timing,
        NoteEvent::Choke { timing, .. } => timing,
        NoteEvent::VoiceTerminated { timing, .. } => timing,
        NoteEvent::PolyModulation { timing, .. } => timing,
        NoteEvent::MonoAutomation { timing, .. } => timing,
        NoteEvent::PolyPressure { timing, .. } => timing,
        NoteEvent::PolyVolume { timing, .. } => timing,
        NoteEvent::PolyPan { timing, .. } => timing,
        NoteEvent::PolyTuning { timing, .. } => timing,
        NoteEvent::PolyVibrato { timing, .. } => timing,
        NoteEvent::PolyExpression { timing, .. } => timing,
        NoteEvent::PolyBrightness { timing, .. } => timing,
        NoteEvent::MidiChannelPressure { timing, .. } => timing,
        NoteEvent::MidiPitchBend { timing, .. } => timing,
        NoteEvent::MidiCC { timing, .. } => timing,
        NoteEvent::MidiProgramChange { timing, .. } => timing,
        NoteEvent::MidiSysEx { timing, .. } => timing,
        _ => panic!("unmatched event {:?}", *ev),
    })
}
