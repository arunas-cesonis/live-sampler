use nih_plug::prelude::NoteEvent;
use nih_plug::wrapper::vst3::vst3_sys::vst::NoteOffEvent;

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
