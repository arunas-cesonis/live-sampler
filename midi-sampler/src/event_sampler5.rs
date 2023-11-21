use crate::count_map::CountMap;
use nih_plug::midi::PluginNoteEvent;
use nih_plug::nih_warn;
use nih_plug::prelude::NoteEvent;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fs::remove_dir;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Condvar;

#[derive(Debug, Default)]
pub struct Params {
    pub sample_rate: f32,
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

fn partition_actions<'a, S>(ev: &'a [NoteEvent<S>]) -> (Vec<&'a NoteEvent<S>>, Vec<Action>) {
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

#[derive(Debug)]
struct Context<'a> {
    now: usize,
    sample_id: usize,
    params: &'a Params,
}

#[derive(Debug)]
struct TimedEvent<S> {
    event: NoteEvent<S>,
    time_from_start: usize,
}

#[derive(Debug, Default)]
struct TimedEvents<S> {
    events: Vec<TimedEvent<S>>,
    duration: usize,
}

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
        // TODO: check how important other events are and if they contain note information
        //NoteEvent::Choke { note, channel, .. } => Note {
        //    note: *note,
        //    channel: *channel,
        //},
        _ => return None, // TODO: implement remaining events
    })
}

#[derive(Debug)]
struct Recorder<S> {
    events: Vec<TimedEvent<S>>,
    start: usize,
}

impl<S> Recorder<S>
where
    S: Clone,
{
    fn new(ctx: &Context) -> Self {
        Self {
            events: vec![],
            start: ctx.now,
        }
    }
    fn process_sample(&mut self, ctx: &Context, events: &[&NoteEvent<S>]) {
        for e in events {
            self.events.push(TimedEvent {
                event: (*e).clone(),
                time_from_start: ctx.now - self.start,
            })
        }
    }
    fn finish(mut self, ctx: &Context) -> TimedEvents<S> {
        TimedEvents {
            events: self.events,
            duration: ctx.now - self.start,
        }
    }
}

#[derive(Default)]
pub struct EventSampler<S> {
    recorder: Option<Recorder<S>>,
    idle: Vec<TimedEvents<S>>,
    now: usize,
}

fn without_note_on<'a, S>(
    events: &'a [&'a NoteEvent<S>],
) -> impl Iterator<Item = &'a NoteEvent<S>> {
    events.into_iter().copied().filter(|e| match e {
        NoteEvent::NoteOn { .. } => false,
        _ => true,
    })
}
fn without_note_off<'a, S>(
    events: &'a [&'a NoteEvent<S>],
) -> impl Iterator<Item = &'a NoteEvent<S>> {
    events.into_iter().copied().filter(|e| match e {
        NoteEvent::NoteOff { .. } => false,
        _ => true,
    })
}

// TODO: statefully handle note-on-off
impl<S> EventSampler<S>
where
    S: Debug + Clone,
{
    pub fn process_sample(
        &mut self,
        sample_id: usize,
        events: Vec<NoteEvent<S>>,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let (non_action_events, actions) = partition_actions(&events);
        let ctx = Context {
            sample_id,
            now: self.now,
            params,
        };
        let ctx = &ctx;
        for a in actions {
            match a {
                Action::StartRecording => {
                    eprintln!("STARTING RECORDING now={}", self.now);
                    self.recorder = Some(Recorder::new(ctx));
                }
                Action::StopRecording => {
                    if let Some(mut rec) = self.recorder.take() {
                        rec.process_sample(
                            ctx,
                            &without_note_on(&non_action_events).collect::<Vec<_>>(),
                        );
                        let rec = rec.finish(ctx);
                        for e in rec.events.iter().enumerate() {
                            eprintln!("{:?}", e);
                        }
                        self.idle.push(rec);
                    }
                }
                _ => (),
            }
        }
        if let Some(mut rec) = self.recorder.as_mut() {
            if rec.start == self.now {
                rec.process_sample(
                    ctx,
                    &without_note_off(&non_action_events).collect::<Vec<_>>(),
                );
            } else {
                rec.process_sample(ctx, &non_action_events);
            }
        }
        self.now += 1;
        vec![]
    }
}
