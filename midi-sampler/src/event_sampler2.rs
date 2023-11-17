use nih_plug::midi::NoteEvent;
use nih_plug::{nih_log, nih_warn};
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::atomic::AtomicI32;
use std::sync::Arc;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
struct Note {
    note: u8,
    channel: u8,
}

#[derive(Debug)]
struct LiveRecording<S> {
    data: Vec<TimedEvent<S>>,
    pressed: HashSet<Note>,
    start: usize,
}

#[derive(Debug, Clone)]
struct EventTime {
    iteration: usize,
    sample_id: usize,
    time: usize,
}

#[derive(Debug, Clone)]
struct TimedEvent<S> {
    event: NoteEvent<S>,
    time: EventTime,
}

#[derive(Debug, Clone)]
struct LastRecording<S> {
    data: Vec<TimedEvent<S>>,
    duration: usize,
}

#[derive(Default, Debug)]
pub struct EventSampler<S> {
    output: Vec<NoteEvent<S>>,
    unhandled: Vec<TimedEvent<S>>,
    handled: Vec<(EventTime, Action)>,
    live_recording: Option<LiveRecording<S>>,
    last_recording: Option<Arc<LastRecording<S>>>,
    time: usize,
}

#[derive(Debug)]
enum Action {
    StartRecording,
    StopRecording,
}

fn event_to_action<S>(event: &NoteEvent<S>) -> Option<Action> {
    Some(match event {
        NoteEvent::NoteOn { note, .. } if *note == 0 => Action::StartRecording,
        NoteEvent::NoteOff { note, .. } if *note == 0 => Action::StopRecording,
        _ => return None,
    })
}

pub struct Params {
    pub sample_rate: f32,
}

fn set_event_timing<S>(ev: &mut NoteEvent<S>, value: u32) {
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

fn event_to_note<S>(ev: &NoteEvent<S>) -> Option<Note> {
    Some(match ev {
        NoteEvent::NoteOn { note, channel, .. } => Note {
            note: *note,
            channel: *channel,
        },
        NoteEvent::NoteOff { note, channel, .. } => Note {
            note: *note,
            channel: *channel,
        },
        NoteEvent::Choke { note, channel, .. } => Note {
            note: *note,
            channel: *channel,
        },
        _ => return None, // TODO: implement remaining events
    })
}

fn get_event_timing<S: Debug>(ev: &NoteEvent<S>) -> u32 {
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

fn debug_event<S: Debug>(e: &NoteEvent<S>) -> String {
    match e {
        NoteEvent::NoteOn { timing, note, .. } => format!("NoteOn timing={} note={}", timing, note),
        NoteEvent::NoteOff { timing, note, .. } => {
            format!("NoteOff timing={} note={}", timing, note)
        }
        NoteEvent::Choke { timing, .. } => format!("Choke timing={}", timing),
        NoteEvent::VoiceTerminated { timing, .. } => format!("VoiceTerminated timing={}", timing),
        NoteEvent::PolyModulation { timing, .. } => format!("PolyModulation timing={}", timing),
        NoteEvent::MonoAutomation { timing, .. } => format!("MonoAutomation timing={}", timing),
        NoteEvent::PolyPressure { timing, .. } => format!("PolyPressure timing={}", timing),
        NoteEvent::PolyVolume { timing, .. } => format!("PolyVolume timing={}", timing),
        NoteEvent::PolyPan { timing, .. } => format!("PolyPan timing={}", timing),
        NoteEvent::PolyTuning { timing, .. } => format!("PolyTuning timing={}", timing),
        NoteEvent::PolyVibrato { timing, .. } => format!("PolyVibrato timing={}", timing),
        NoteEvent::PolyExpression { timing, .. } => format!("PolyExpression timing={}", timing),
        NoteEvent::PolyBrightness { timing, .. } => format!("PolyBrightness timing={}", timing),
        NoteEvent::MidiChannelPressure { timing, .. } => {
            format!("MidiChannelPressure timing={}", timing)
        }
        NoteEvent::MidiPitchBend { timing, .. } => format!("MidiPitchBend timing={}", timing),
        NoteEvent::MidiCC { timing, .. } => format!("MidiCC timing={}", timing),
        NoteEvent::MidiProgramChange { timing, .. } => {
            format!("MidiProgramChange timing={}", timing)
        }
        NoteEvent::MidiSysEx { timing, .. } => format!("MidiSysEx timing={}", timing),
        _ => panic!("unmatched event {:?}", e),
    }
}

fn debug_note_on_off<S: Debug>(e: &[TimedEvent<S>]) -> String {
    let mut s = vec![];
    for e in e {
        match e.event {
            NoteEvent::NoteOn { note, .. } => s.push(format!("on={}", note)),
            NoteEvent::NoteOff { note, .. } => s.push(format!("off={}", note)),
            _ => (),
        }
    }
    s.join(" ")
}

impl<S: Debug> EventSampler<S> {
    pub fn handle_event(&mut self, iteration: usize, event: NoteEvent<S>, params: &Params) {
        let ev_time = EventTime {
            iteration,
            sample_id: get_event_timing(&event) as usize,
            time: self.time,
        };
        if let Some(action) = event_to_action(&event) {
            self.handled.push((ev_time, action));
        } else {
            let timed = TimedEvent {
                event,
                time: ev_time,
            };
            self.unhandled.push(timed);
        }
    }

    fn handle_action(
        &mut self,
        iteration: usize,
        sample_id: usize,
        action: &Action,
        params: &Params,
    ) {
        match action {
            Action::StartRecording => {
                if self.live_recording.is_some() {
                    nih_warn!("start_recording: restarting");
                } else {
                    nih_warn!("start_recording: starting");
                }
                self.live_recording = Some(LiveRecording {
                    data: vec![],
                    pressed: HashSet::new(),
                    start: self.time,
                })
            }
            Action::StopRecording => {
                if let Some(mut r) = self.live_recording.take() {
                    let duration = self.time - r.start;
                    nih_warn!(
                        "stop_recording: saving {} events ({:.3}s)",
                        r.data.len(),
                        duration as f32 / params.sample_rate
                    );
                    for (i, e) in r.data.iter().enumerate() {
                        nih_log!("{} {:?}  {}", i, e.time, debug_event(&e.event));
                    }
                    for note in r.pressed {
                        let time = EventTime {
                            iteration,
                            sample_id,
                            time: self.time,
                        };
                        nih_log!("adding NoteOff for {:?} {:?}", note, time);
                        r.data.push(TimedEvent {
                            time,
                            event: NoteEvent::NoteOff {
                                timing: sample_id as u32,
                                voice_id: None,
                                channel: note.channel,
                                note: note.note,
                                velocity: 0.0,
                            },
                        });
                    }
                    nih_log!("{}", debug_note_on_off(&r.data));
                    self.last_recording = Some(Arc::new(LastRecording {
                        data: r.data,
                        duration,
                    }));
                } else {
                    nih_warn!("stop_recording: no live recording to stop");
                }
            }
        }
    }

    pub fn process_sample(
        &mut self,
        iteration: usize,
        sample_id: usize,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let actions = std::mem::take(&mut self.handled);
        for (t, a) in actions {
            nih_log!("{:?} {:?}", t, a);
            self.handle_action(iteration, sample_id, &a, params);
        }
        let unhandled = std::mem::take(&mut self.unhandled);
        if let Some(r) = &mut self.live_recording {
            for e in unhandled {
                if let Some(note) = event_to_note(&e.event) {
                    match e.event {
                        NoteEvent::NoteOn { .. } => {
                            r.pressed.insert(note);
                        }
                        NoteEvent::NoteOff { .. } => {
                            r.pressed.remove(&note);
                        }
                        NoteEvent::Choke { .. } => {
                            r.pressed.remove(&note);
                        }
                        _ => (),
                    };
                };
                r.data.push(e);
            }
        }
        self.time += 1;
        std::mem::take(&mut self.output)
    }
}
