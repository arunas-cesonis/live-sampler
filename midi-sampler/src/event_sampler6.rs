use crate::utils::{is_note_off, is_note_on};
use nih_plug::log::Record;
use nih_plug::midi::NoteEvent;
use nih_plug::nih_warn;
use nih_plug::prelude::ClapFeature::NoteEffect;
use std::fmt::Debug;
use std::sync::Arc;

// 'time 'in this module is calculated as number of audio frames processed

pub struct Params {
    pub sample_rate: f32, // for better messaging / debugging purposes
}

#[derive(Debug)]
struct TimedEvent<S> {
    event: NoteEvent<S>,
    time: usize,
}

struct Clip<S> {
    events: Vec<TimedEvent<S>>,
    duration: usize,
}

impl<S> Clip<S> {
    fn push_events<'a, I>(&mut self, events: I)
    where
        I: IntoIterator<Item = &'a NoteEvent<S>> + 'a,
        S: 'a + Clone,
    {
        self.events.extend(events.into_iter().map(|e| TimedEvent {
            event: e.clone(),
            time: self.duration,
        }));
    }
}

#[derive(Default)]
pub struct EventSampler<S> {
    recording: Option<Clip<S>>,
    stored: Option<Arc<Clip<S>>>,
}

#[derive(Debug, Eq, PartialEq)]
enum Action {
    StartRecording,
    StopRecording,
    Play,
    Stop,
}

impl<S> TryFrom<&NoteEvent<S>> for Action {
    type Error = ();
    fn try_from(value: &NoteEvent<S>) -> Result<Self, Self::Error> {
        Ok(match value {
            NoteEvent::NoteOn { note, .. } if *note == 0 => Action::StartRecording,
            NoteEvent::NoteOff { note, .. } if *note == 0 => Action::StopRecording,
            NoteEvent::NoteOn { note, .. } if *note == 1 => Action::Play,
            NoteEvent::NoteOff { note, .. } if *note == 1 => Action::Stop,
            _ => return Err(()),
        })
    }
}

fn partition_actions<S>(ev: Vec<NoteEvent<S>>) -> (Vec<NoteEvent<S>>, Vec<Action>) {
    let mut out_events = vec![];
    let mut out_actions = vec![];
    for e in ev {
        if let Ok(a) = Action::try_from(&e) {
            out_actions.push(a);
        } else {
            out_events.push(e);
        }
    }
    (out_events, out_actions)
}

impl<S> EventSampler<S>
where
    S: Clone + Debug,
{
    fn do_action(
        &mut self,
        action: Action,
        events: &[NoteEvent<S>],
        output: &mut Vec<NoteEvent<S>>,
    ) {
        match action {
            Action::StartRecording => {
                nih_warn!("START RECORDING");
                self.recording = Some(Clip {
                    events: vec![],
                    duration: 0,
                });
            }
            Action::StopRecording => {
                if let Some(mut clip) = self.recording.take() {
                    clip.push_events(events.into_iter().filter(|e| !is_note_on(e)));
                    nih_warn!("STOP RECORDING");
                    clip.events
                        .iter()
                        .enumerate()
                        .for_each(|(i, e)| nih_warn!("{}: {:?}", i, e));
                    nih_warn!("^^^^^ {} events", clip.events.len());
                    self.stored = Some(Arc::new(clip));
                }
            }
            Action::Play => nih_warn!("PLAY"),
            Action::Stop => nih_warn!("STOP"),
        }
    }

    fn record_events(&mut self, events: &[NoteEvent<S>]) {
        if let Some(clip) = self.recording.as_mut() {
            if clip.duration == 0 {
                // not sure why is this necessary? but remember it solved edge case crashing
                clip.push_events(events.into_iter().filter(|e| !is_note_off(e)));
            } else {
                clip.push_events(events);
            }
            clip.duration += 1;
        }
    }

    pub fn process_sample(
        &mut self,
        events: Vec<NoteEvent<S>>,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let mut output = vec![];
        let (events, actions) = partition_actions(events);
        actions
            .into_iter()
            .for_each(|a| self.do_action(a, &events, &mut output));
        self.record_events(&events);
        output
    }
}
