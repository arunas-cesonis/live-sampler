use crate::count_map::CountMap;
use crate::utils::set_event_timing_mut;
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

#[derive(Copy, Clone, Debug)]
struct Playhead {
    clip: usize,
    next_event: usize,
    start_time: usize,
}

#[derive(Default, Debug)]
pub struct EventSampler<S> {
    recorder: Option<Recorder<S>>,
    clips: Vec<TimedEvents<S>>,
    playheads: Vec<Playhead>,
    voices: CountMap<Note>,
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

#[cfg(test)]
mod test {
    use crate::event_sampler5::{EventSampler, Params};
    use crate::utils::{set_event_timing, set_event_timing_mut};
    use nih_plug::prelude::NoteEvent;

    const fn mk_note_off(note: u8, channel: u8) -> NoteEvent<()> {
        NoteEvent::NoteOff {
            note,
            channel,
            velocity: 0.0,
            timing: 0,
            voice_id: None,
        }
    }
    const fn mk_note_on(note: u8, channel: u8) -> NoteEvent<()> {
        NoteEvent::NoteOn {
            note,
            channel,
            velocity: 1.0,
            timing: 0,
            voice_id: None,
        }
    }
    const START_RECORDING: NoteEvent<()> = mk_note_on(0, 0);
    const STOP_RECORDING: NoteEvent<()> = mk_note_off(0, 0);
    const START_PLAYING: NoteEvent<()> = mk_note_on(1, 0);
    const STOP_PLAYING: NoteEvent<()> = mk_note_off(1, 0);
    const C3: NoteEvent<()> = mk_note_on(0x30, 0);
    const C3_OFF: NoteEvent<()> = mk_note_off(0x30, 0);
    const F5: NoteEvent<()> = mk_note_on(0x4d, 0);
    const F5_OFF: NoteEvent<()> = mk_note_off(0x4d, 0);
    const PARAMS: Params = Params {
        sample_rate: 44100.0,
    };

    #[test]
    fn test_recoding() {
        let mut m = EventSampler::default();
        m.process_sample(0, vec![START_RECORDING.clone()], &PARAMS);
        m.process_sample(1, vec![C3.clone()], &PARAMS);
        m.process_sample(2, vec![C3_OFF.clone()], &PARAMS);
        m.process_sample(3, vec![STOP_RECORDING.clone()], &PARAMS);
        //
        m.process_sample(4, vec![START_PLAYING.clone()], &PARAMS);
        assert_eq!(
            vec![set_event_timing(C3.clone(), 5)],
            m.process_sample(5, vec![], &PARAMS)
        );
        assert_eq!(
            vec![set_event_timing(C3_OFF.clone(), 6)],
            m.process_sample(6, vec![], &PARAMS)
        );
        m.process_sample(7, vec![STOP_PLAYING.clone()], &PARAMS);
        eprintln!("{:?}", m);
    }
}

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
                        self.clips.push(rec);
                    }
                }
                Action::Play if !self.clips.is_empty() => {
                    // for now safest logic
                    let playhead = Playhead {
                        clip: self.clips.len() - 1,
                        next_event: 0,
                        start_time: self.now,
                    };
                    self.playheads = vec![playhead];
                }
                Action::Play => {
                    nih_warn!("attempted to play but nothing has been recorded")
                }
                Action::Stop => {
                    // not cancelling active note-on's yet
                    self.playheads = vec![];
                }
            }
        }
        //
        // Process ongoing recording
        //
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
        //
        // Process play
        //
        let mut output = vec![];
        let mut removed = vec![];
        for i in 0..self.playheads.len() {
            loop {
                let ph = self.playheads[i];
                let mut next_ev = &self.clips[ph.clip].events[ph.next_event];
                if self.now - ph.start_time != next_ev.time_from_start {
                    assert!(self.now - ph.start_time < next_ev.time_from_start);
                    break;
                }
                let mut ev = next_ev.event.clone();
                set_event_timing_mut(&mut ev, sample_id as u32);
                output.push(ev);
                self.playheads[i].next_event += 1;
                if self.playheads[i].next_event == self.clips[ph.clip].events.len() {
                    removed.push(i);
                    break;
                }
            }
        }
        while let Some(i) = removed.pop() {
            self.playheads.remove(i);
        }
        self.now += 1;
        output
    }
}
