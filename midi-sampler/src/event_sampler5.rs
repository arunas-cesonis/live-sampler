use crate::count_map::CountMap;
use crate::utils::{set_event_timing, set_event_timing_mut};
use nih_plug::midi::PluginNoteEvent;
use nih_plug::prelude::NoteEvent;
use nih_plug::{nih_export_standalone, nih_warn};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fs::remove_dir;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Arc, Condvar};

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

fn partition_actions<S>(ev: Vec<NoteEvent<S>>) -> (Vec<NoteEvent<S>>, Vec<Action>) {
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

fn timed<S>(event: NoteEvent<S>, time_from_start: usize) -> TimedEvent<S> {
    TimedEvent {
        event,
        time_from_start,
    }
}

fn mk_note_off<S>(note: u8, timing: u32) -> NoteEvent<S> {
    NoteEvent::NoteOff {
        note,
        channel: 0,
        velocity: 0.0,
        timing,
        voice_id: None,
    }
}

fn mk_note_on<S>(note: u8, timing: u32) -> NoteEvent<S> {
    NoteEvent::NoteOn {
        note,
        channel: 0,
        velocity: 1.0,
        timing,
        voice_id: None,
    }
}

#[derive(Debug, Default)]
struct Clip<S> {
    events: Vec<TimedEvent<S>>,
    duration: usize, // need this for looping as it can be longer than when the last not finishes
}

impl<S> Clip<S> {
    pub fn get_cursor<'a>(&'a self) -> ClipCursor<'a, S> {
        ClipCursor {
            clip: self,
            sample: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct ClipCursor<'a, S> {
    clip: &'a Clip<S>,
    sample: usize,
}

impl<'a, S> ClipCursor<'a, S> {
    fn process_sample(&mut self, ctx: &Context) -> Vec<NoteEvent<S>>
    where
        S: Clone,
    {
        if self.clip.events.is_empty() {
            return vec![];
        }
        let mut output = vec![];
        self.sample += 1;
        output
    }
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
    fn process_sample<'a, I>(&mut self, ctx: &Context, events: I)
    where
        I: Iterator<Item = &'a NoteEvent<S>>,
        S: Clone + 'a,
    {
        for e in events {
            self.events.push(TimedEvent {
                event: (*e).clone(),
                time_from_start: ctx.now - self.start,
            })
        }
    }
    fn finish(mut self, ctx: &Context) -> Clip<S> {
        Clip {
            events: self.events,
            duration: ctx.now - self.start,
        }
    }
}

#[derive(Clone, Debug)]
struct Playhead2<S> {
    clip: Arc<Clip<S>>,
    current_event: usize,
    current_sample: usize, // number of audio samples this clip has progressed  forward
    voices: CountMap<Note>,
}

impl<S> Playhead2<S>
where
    S: Clone + Debug,
{
    fn new(clip: Arc<Clip<S>>) -> Self {
        Self {
            clip,
            current_event: 0,
            current_sample: 0,
            voices: Default::default(),
        }
    }

    fn process_sample(&mut self, ctx: &Context) -> Vec<NoteEvent<S>> {
        let normalized_sample = self.current_sample % self.clip.duration;
        let mut output = vec![];
        loop {
            let normalized_event = self.current_event % self.clip.events.len();
            if normalized_event == self.clip.events.len() {
                if normalized_sample == self.clip.duration {
                    self.current_event += 1;
                } else {
                    assert!(normalized_sample < self.clip.duration);
                    // wait for audio to reach the end of clip
                    break;
                }
            } else {
                let event = &self.clip.events[normalized_event];
                if normalized_sample == event.time_from_start {
                    let mut new_event = event.event.clone();
                    set_event_timing_mut(&mut new_event, ctx.sample_id as u32);
                    match event_to_note(&new_event) {
                        Some((note, NoteState::On)) => self.start_voice(note),
                        Some((note, NoteState::Off)) => self.stop_voice(note),
                        None => (),
                    };
                    output.push(new_event);
                    self.current_event += 1;
                } else {
                    assert!(
                        normalized_sample < event.time_from_start,
                        "normalized_sample={} {:#?}",
                        normalized_sample,
                        self
                    );
                    break;
                }
            Clip<S>>>,
    playheads: Vec<Playhead>,
    playheads2: Vec<Playhead2<S>>,
    voices: CountMap<Note>,
    now: usize,
}

fn is_note_on<S>(event: &NoteEvent<S>) -> bool {
    match event {
        NoteEvent::NoteOn { .. } => true,
        _ => false,
    }
}
fn is_note_off<S>(event: &NoteEvent<S>) -> bool {
    match event {
        NoteEvent::NoteOff { .. } => true,
        _ => false,
    }
}

#[cfg(test)]
mod test {
    use crate::event_sampler5::{
        mk_note_off, mk_note_on, timed, Clip, Context, Params, TimedEvent,
    };
    use nih_plug::midi::NoteEvent;

    #[test]
    fn test_clip() {
        let params = Params::default();
        let mut ctx = Context {
            now: 0,
            sample_id: 0,
            params: &params,
        };
        let clip = Clip {
            events: vec![
                timed(mk_note_on::<()>(0, 0), 0),
                timed(mk_note_off::<()>(0, 0), 10),
                timed(mk_note_on::<()>(1, 0), 10),
                timed(mk_note_off::<()>(1, 0), 11),
            ],
            duration: 30,
        };
        ctx.sample_id = 123;
        let mut cur = clip.get_cursor();

        for i in 0..clip.events.len() {
            eprintln!("clip.events[{}] = [{:?}] ", i, clip.events[i]);
        }
        for i in 0..clip.duration {
            eprintln!("{}: {:?}", i, cur.process_sample(&ctx));
        }
    }
}

impl<S> EventSampler<S>
where
    S: Debug + Clone,
{
    fn removed_playhead2(&mut self, ctx: &Context, i: usize, output: &mut Vec<NoteEvent<S>>) {
        let ph = self.playheads2.remove(i);
        nih_warn!("MAIN: :removing {:?}", ph);
        let mut ev = ph.gen_events_to_stop_voices(ctx).collect::<Vec<_>>();
        nih_warn!("MAIN: note-off {:?}", ev);
        output.append(&mut ev);
    }

    fn removed_playhead(&mut self, ctx: &Context, i: usize, output: &mut Vec<NoteEvent<S>>) {
        let ph = self.playheads.remove(i);
        nih_warn!("MAIN: removing {:?}", ph);
        let mut ev = ph.gen_events_to_stop_voices(ctx).collect::<Vec<_>>();
        nih_warn!("MAIN: generated note-off {:?}", ev);
        output.append(&mut ev);
    }

    pub fn process_sample(
        &mut self,
        sample_id: usize,
        events: Vec<NoteEvent<S>>,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let (non_action_events, actions) = partition_actions(events);
        let ctx = Context {
            sample_id,
            now: self.now,
            params,
        };
        let ctx = &ctx;
        let mut output = vec![];
        for a in actions {
            match a {
                Action::StartRecording => {
                    nih_warn!("[{}] Recording: start", self.now);
                    self.recorder = Some(Recorder::new(ctx));
                }
                Action::StopRecording => {
                    if let Some(mut rec) = self.recorder.take() {
                        rec.process_sample(
                            ctx,
                            non_action_events.iter().filter(|x| match x {
                                NoteEvent::NoteOn { .. } => false,
                                _ => true,
                            }),
                        );
                        let rec = rec.finish(ctx);
                        // its possible that
                        // 1. the duration of recording is 0 and
                        // a. the events are emitted once
                        // b. same events are emitted at audio rate
                        // 2. the recording contains no events and has some duration. In the
                        // case MIDI is not passed throught during playback this would function as a "mute" clip

                        if rec.events.is_empty() && rec.duration == 0 {
                            nih_warn!("[{}] Recording: finished empty", self.now);
                        } else {
                            if rec.events.is_empty() {
                                nih_warn!(
                                    "[{}] Recorded empty clip of duration {}",
                                    self.now,
                                    rec.duration
                                );
                            }
                            let mut str = String::new();
                            for (i, e) in rec.events.iter().enumerate() {
                                let tmp = match e.event {
                                    NoteEvent::NoteOn { note, channel, .. } => {
                                        format!("on({}, {})", note, channel)
                                    }
                                    NoteEvent::NoteOff { note, channel, .. } => {
                                        format!("off({}, {})", note, channel)
                                    }
                                    _ => {
                                        format!("other()")
                                    }
                                };
                                if i != 1 {
                                    str.push_str("\n");
                                }
                                str.push_str(i.to_string().as_str());
                                str.push_str(tmp.as_str());
                            }
                            eprintln!("{}", str);
                            self.clips.push(Arc::new(rec));
                        }
                    }
                }
                Action::Play if !self.clips.is_empty() => {
                    // for now safest logic
                    let ph2 = Playhead2::new(self.clips[0].clone());
                    self.playheads2.push(ph2);

                    //let playhead = Playhead {
                    //    clip: self.clips.len() - 1,
                    //    next_event: 0,
                    //    start_time: self.now,
                    //    voices: Default::default(),
                    //};
                    //self.playheads = vec![playhead];
                }
                Action::Play => {
                    nih_warn!("Play: nothing recorded");
                }
                Action::Stop => {
                    // not cancelling active note-on's yet
                    if !self.playheads2.is_empty() {
                        self.removed_playhead2(ctx, 0, &mut output);
                    }
                    //if !self.playheads.is_empty() {
                    //    self.removed_playhead(ctx, 0, &mut output);
                    //}
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
                    non_action_events.iter().filter(|x| match x {
                        NoteEvent::NoteOff { .. } => false,
                        _ => true,
                    }),
                );
            } else {
                rec.process_sample(ctx, non_action_events.iter());
            }
        }
        //
        // Process play
        //
        for i in 0..self.playheads2.len() {
            let mut new_events = self.playheads2[i].process_sample(ctx);
            output.append(&mut new_events);
        }
        self.now += 1;
        output
        /*
        let mut removed = vec![];
        for i in 0..self.playheads.len() {
            loop {
                let ph_next_event = self.playheads[i].next_event;
                let ph_start_time = self.playheads[i].start_time;
                let ph_clip = self.playheads[i].clip;
                let clip = &self.clips[ph_clip];
                if ph_next_event == clip.events.len() {
                    let time_remaining = clip.duration - (self.now - ph_start_time);
                    if time_remaining == 0 {
                        removed.push(i);
                    }
                    break;
                }
                let next_ev = &clip.events[ph_next_event];
                if self.now - ph_start_time != next_ev.time_from_start {
                    assert!(self.now - ph_start_time < next_ev.time_from_start);
                    break;
                }
                let mut ev = next_ev.event.clone();
                set_event_timing_mut(&mut ev, sample_id as u32);
                match event_to_note(&ev) {
                    Some((note, NoteState::On)) => self.playheads[i].start_voice(note),
                    Some((note, NoteState::Off)) => self.playheads[i].stop_voice(note),
                    None => (),
                };
                output.push(ev);
                self.playheads[i].next_event += 1;
                if self.playheads[i].next_event == self.clips[self.playheads[i].clip].events.len() {
                    removed.push(i);
                    break;
                }
            }
        }
        while let Some(i) = removed.pop() {
            self.removed_playhead(ctx, i, &mut output);
        }
        self.now += 1;
        if output.is_empty() && self.playheads.is_empty() {
            // pass-through
            non_action_events
        } else {
            output
        }
        */
    }
}
