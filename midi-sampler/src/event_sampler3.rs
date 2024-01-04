use nih_plug::midi::NoteEvent;
use nih_plug::midi::NoteEvent::NoteOff;
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

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
enum NoteState {
    On,
    Off,
}

#[derive(Debug, Eq, PartialEq)]
enum Action {
    StartRecording,
    StopRecording,
    Play,
    Stop,
}

fn event_to_action<S>(event: &NoteEvent<S>) -> Option<Action> {
    Some(match event {
        NoteEvent::NoteOn { note, .. } if *note == 0 => Action::StartRecording,
        NoteEvent::NoteOff { note, .. } if *note == 0 => Action::StopRecording,
        NoteEvent::NoteOn { note, .. } if *note == 1 => Action::Play,
        NoteEvent::NoteOff { note, .. } if *note == 1 => Action::Stop,
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

fn event_to_note<S>(ev: &NoteEvent<S>) -> Option<(Note, NoteState)> {
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
        // TODO: check how important other events are
        //NoteEvent::Choke { note, channel, .. } => Note {
        //    note: *note,
        //    channel: *channel,
        //},
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

#[derive(Debug, Clone)]
struct RecordedEvent<S>
where
    S: Clone,
{
    event: NoteEvent<S>,
    time_from_start: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum State {
    Recording,
    Playing,
    Idle,
}
impl Default for State {
    fn default() -> Self {
        State::Idle
    }
}

#[derive(Default, Debug)]
pub struct EventSampler<S: Clone> {
    state: State,
    recording_events: Vec<RecordedEvent<S>>,
    recording_since: usize,
    playing_since: usize,
    playing_i: usize,
    last_recording: Vec<RecordedEvent<S>>,
    last_recording_duration: usize,
    output: Vec<NoteEvent<S>>,
    recording_pressed: HashSet<Note>,
    playing_pressed: HashSet<Note>,
    time: usize,
}

fn partition_map<A, B, C, F, I>(f: F, v: I) -> (Vec<B>, Vec<C>)
where
    F: Fn(A) -> std::result::Result<B, C>,
    I: IntoIterator<Item = A>,
{
    let mut bs = vec![];
    let mut cs = vec![];
    for x in v {
        match f(x) {
            Ok(b) => bs.push(b),
            Err(c) => cs.push(c),
        }
    }
    (bs, cs)
}

fn partition_note_offs<S>(events: Vec<NoteEvent<S>>) -> (Vec<NoteEvent<S>>, Vec<NoteEvent<S>>) {
    partition_map(
        |x| match x {
            NoteEvent::NoteOff { .. } => Ok(x),
            _ => Err(x),
        },
        events,
    )
}

impl<S: Debug + Clone> EventSampler<S> {
    pub fn handle_event(&mut self, event: NoteEvent<S>, params: &Params) {}

    fn split_actions(ev: Vec<NoteEvent<S>>) -> (Vec<NoteEvent<S>>, Vec<Action>) {
        let mut out_events = vec![];
        let mut out_actions = vec![];
        for e in ev {
            if let Some(a) = event_to_action(&e) {
                out_actions.push(a);
            } else {
                out_events.push(e);
            }
        }
        (out_events, out_actions)
    }

    fn record_event(&mut self, event: NoteEvent<S>) {
        self.recording_events.push(RecordedEvent {
            event,
            time_from_start: self.time - self.recording_since,
        });
    }

    fn record_note_offs(
        &mut self,
        sample_id: usize,
        events: Vec<NoteEvent<S>>,
    ) -> Vec<NoteEvent<S>> {
        let (_, rest) = partition_note_offs(events);
        let pressed = std::mem::take(&mut self.recording_pressed);
        for note in pressed {
            self.record_event(NoteEvent::NoteOff {
                timing: sample_id as u32,
                voice_id: None,
                channel: note.channel,
                note: note.note,
                velocity: 0.0,
            });
            nih_log!("{} recording note off {:?}", self.time, note);
        }
        rest
        //if !note_offs.is_empty() {
        //    nih_log!("recording {} final note offs", note_offs.len());
        //}
        //for ev in note_offs {
        //    self.recording_event(ev);
        //}
    }

    pub fn process_sample(
        &mut self,
        sample_id: usize,
        events: Vec<NoteEvent<S>>,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let (mut events, actions) = Self::split_actions(events);
        let has_stop_rec = actions.iter().any(|a| *a == Action::StopRecording);
        let has_start_rec = actions.iter().any(|a| *a == Action::StartRecording);
        let has_play = actions.iter().any(|a| *a == Action::Play);
        let has_stop = actions.iter().any(|a| *a == Action::Stop);

        if has_stop_rec && self.state == State::Recording {
            nih_log!("{} stop rec", self.time);
            events = self.record_note_offs(sample_id, events);
            let mut data = vec![];
            std::mem::swap(&mut data, &mut self.recording_events);
            self.last_recording = data;
            self.last_recording_duration = self.time - self.recording_since;
            nih_log!(
                "recorded {} events in {:.3}s ",
                self.last_recording.len(),
                self.last_recording_duration as f32 / params.sample_rate
            );
            self.state = State::Idle;
        }

        if has_start_rec && self.state == State::Idle {
            nih_log!("{} start", self.time);
            self.recording_events = vec![];
            self.recording_since = self.time;
            self.recording_pressed.clear();
            self.state = State::Recording;
        }

        if has_play && self.state == State::Idle && self.last_recording_duration > 0 {
            nih_log!("{} play", self.time);
            self.playing_since = self.time;
            self.playing_i = 0;
            self.playing_pressed.clear();
            self.state = State::Playing;
        }

        let mut output = vec![];
        if has_stop && self.state == State::Playing {
            nih_log!("{} stop", self.time);
            let play_pressed = std::mem::take(&mut self.playing_pressed);
            for note in play_pressed {
                nih_log!("{} stop note {:?}", self.time, note);
                output.push(NoteEvent::NoteOff {
                    timing: sample_id as u32,
                    voice_id: None,
                    channel: note.channel,
                    note: note.note,
                    velocity: 0.0,
                });
            }
            self.state = State::Idle;
        }

        let unhandled = events;

        match self.state {
            State::Recording => {
                for ev in unhandled {
                    if let Some((note, note_state)) = event_to_note(&ev) {
                        match note_state {
                            NoteState::On => {
                                if !self.recording_pressed.insert(note) {
                                    continue;
                                }
                            }
                            NoteState::Off => {
                                if !self.recording_pressed.remove(&note) {
                                    continue;
                                }
                            }
                        }
                    }

                    self.recording_events.push(RecordedEvent {
                        event: ev,
                        time_from_start: self.time - self.recording_since,
                    });
                }
            }
            State::Playing => loop {
                if self.playing_i == self.last_recording.len() {
                    if self.last_recording_duration + self.playing_since == self.time {
                        self.playing_i = 0;
                        self.playing_since = self.time;
                    } else {
                        break;
                    }
                }
                let ev = &mut self.last_recording[self.playing_i];
                if ev.time_from_start != self.time - self.playing_since {
                    break;
                }
                let mut ev = ev.clone();
                nih_log!("{} play event {:?}", self.time, ev);

                if let Some((note, note_state)) = event_to_note(&ev.event) {
                    match note_state {
                        NoteState::On => {
                            if !self.recording_pressed.insert(note) {
                                continue;
                            }
                        }
                        NoteState::Off => {
                            if !self.recording_pressed.remove(&note) {
                                continue;
                            }
                        }
                    }
                }

                set_event_timing(&mut ev.event, sample_id as u32);
                output.push(ev.event);
                self.playing_i += 1;
            },
            State::Idle => {}
        }
        self.time += 1;
        output
    }
}
