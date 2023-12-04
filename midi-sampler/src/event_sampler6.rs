use crate::count_map::CountMap;
use crate::utils::{is_note_off, is_note_on, note_from_event, Note, NoteState};
use nih_plug::log::Record;
use nih_plug::midi::NoteEvent;
use nih_plug::nih_warn;
use nih_plug::prelude::ClapFeature::NoteEffect;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::process::Output;
use std::slice::from_raw_parts;
use std::sync::Arc;

// 'time 'in this module is calculated as number of audio frames processed

pub struct Params {
    pub sample_rate: f32, // for better messaging / debugging purposes
}

#[derive(Debug)]
struct TimedEvents<S> {
    note_events: Vec<NoteEvent<S>>,
    time: usize,
}

#[derive(Debug)]
struct Clip<S> {
    events: Vec<TimedEvents<S>>,
    duration: usize,
}

impl<S> Default for Clip<S> {
    fn default() -> Self {
        Self {
            events: vec![],
            duration: 0,
        }
    }
}

impl<S> Clip<S> {
    fn count_events(&self) -> usize {
        self.events
            .iter()
            .map(|e| e.note_events.len())
            .sum::<usize>()
    }
    fn push_events<'a, I>(&mut self, time: usize, note_events: I)
    where
        I: IntoIterator<Item = &'a NoteEvent<S>> + 'a,
        S: 'a + Clone + Debug,
    {
        let note_events: Vec<_> = note_events.into_iter().cloned().collect();
        if note_events.is_empty() {
            return;
        }
        let tmp = note_events.clone();
        self.events.push(TimedEvents {
            note_events,
            time: self.duration,
        });
        nih_warn!(
            "time={} PUSH EVENTS total={} events={:?}",
            time,
            self.count_events(),
            tmp,
        );
    }
}

#[derive(Debug)]
struct Playing<S> {
    clip: Arc<Clip<S>>,
    time: usize,
    clip_events_index: usize,
    voices: Voices,
}

#[derive(Debug, Default)]
pub struct EventSampler<S> {
    recording: Option<Clip<S>>,
    stored: Option<Arc<Clip<S>>>,
    playing: Option<Playing<S>>,
    time: usize,
}

#[derive(Default)]
struct FrameActions {
    start_recording: bool,
    stop_recording: bool,
    play: bool,
    stop: bool,
}

fn partition_actions<S>(ev: Vec<NoteEvent<S>>) -> (Vec<NoteEvent<S>>, FrameActions) {
    let mut out_events = vec![];
    let mut frame_actions = FrameActions::default();
    for e in ev {
        match e {
            NoteEvent::NoteOn { note, .. } if note == 0 => frame_actions.start_recording = true,
            NoteEvent::NoteOff { note, .. } if note == 0 => frame_actions.stop_recording = true,
            NoteEvent::NoteOn { note, .. } if note == 1 => frame_actions.play = true,
            NoteEvent::NoteOff { note, .. } if note == 1 => frame_actions.stop = true,
            _ => {
                out_events.push(e);
            }
        };
    }
    (out_events, frame_actions)
}

fn dump_events<'a, I, S>(events: I)
where
    I: IntoIterator<Item = &'a TimedEvents<S>>,
    S: Debug + 'a,
{
    let events = events.into_iter().collect::<Vec<_>>();
    if events.len() > 10 {
        events[0..3]
            .iter()
            .enumerate()
            .for_each(|(i, e)| nih_warn!("{}: {:?}", i, e));
        nih_warn!("...");
        events[events.len() - 3..events.len()]
            .iter()
            .enumerate()
            .for_each(|(i, e)| nih_warn!("{}: {:?}", events.len() - 3 + i, e));
    } else {
        events
            .iter()
            .enumerate()
            .for_each(|(i, e)| nih_warn!("{}: {:?}", i, e));
    }
}

#[derive(Debug, Default)]
struct Voices {
    m: CountMap<Note>,
}

impl Voices {
    fn handle_event<S>(&mut self, e: &NoteEvent<S>) {
        match note_from_event(e) {
            Some((note, NoteState::On)) => {
                self.m.inc(&note);
                nih_warn!("VOICES[{}]: add {}", self.m.get(&note), note.note);
            }
            Some((note, NoteState::Off)) => {
                self.m.dec(&note);
                nih_warn!("VOICES[{}]: del {}", self.m.get(&note), note.note);
            }
            None => (),
        };
    }
    fn gen_events_to_stop_voices<S>(&self, output: &mut Vec<NoteEvent<S>>) {
        let mut count = self.m.count_nonzero();
        output.extend(self.m.iter_nonzero().map(|(note, _)| {
            nih_warn!("VOICES[{}]: del {}", count, note.note);
            count -= 1;
            NoteEvent::NoteOff {
                timing: 0, // this bit is set in the Plugin implementation
                voice_id: None,
                channel: note.channel,
                note: note.note,
                velocity: 0.0,
            }
        }))
    }
}

// To get this to work usefully:
// 1. Do not record events on finishing edge
// 2. Insert note-offs when stopping playback
// 3. Insert note-offs when looped playback wraps around
// 4. Do not record note-offs on starting edge
impl<S> EventSampler<S>
where
    S: Clone + Debug,
{
    fn stop_playing(&mut self, output: &mut Vec<NoteEvent<S>>) {
        if let Some(playing) = self.playing.take() {
            playing.voices.gen_events_to_stop_voices(output);
        }
    }

    fn start_recording(&mut self) {
        nih_warn!("time={} START RECORDING", self.time,);
        let mut clip = Clip::default();
        self.recording = Some(clip);
    }

    fn finish_recording(&mut self) -> bool {
        if let Some(clip) = self.recording.take() {
            nih_warn!(
                "time={} STOP RECORDING: events={} duration={}",
                self.time,
                clip.events.len(),
                clip.duration
            );
            dump_events(&clip.events);
            self.stored = Some(Arc::new(clip));
            true
        } else {
            false
        }
    }

    /**
     *
     * Note that any events that occur at the same time as "stop_recording" will not be recorded.
     * Necessary note-offs are inserted during playback.
     */
    fn process_recording(&mut self, frame_actions: &FrameActions, events: &[NoteEvent<S>]) {
        if frame_actions.start_recording && frame_actions.stop_recording {
            if self.finish_recording() {
                self.start_recording();
            } else {
                nih_warn!("time={} RESET RECORDING: nothing to reset", self.time);
            }
        } else if frame_actions.start_recording {
            self.finish_recording();
            self.start_recording();
        } else if frame_actions.stop_recording {
            self.finish_recording();
        }
        if let Some(recording) = self.recording.as_mut() {
            recording.push_events(self.time, events);
            recording.duration += 1;
        }
    }

    fn finish_playing(&mut self, output: &mut Vec<NoteEvent<S>>) -> bool {
        if let Some(playing) = self.playing.take() {
            eprintln!("time={} STOP PLAYING", self.time);
            playing.voices.gen_events_to_stop_voices(output);
            true
        } else {
            false
        }
    }

    fn start_playing(&mut self, output: &mut Vec<NoteEvent<S>>) {
        if let Some(clip) = &self.stored {
            self.playing = Some(Playing {
                clip: clip.clone(),
                time: 0,
                clip_events_index: 0,
                voices: Voices::default(),
            });
        }
    }

    fn process_playback(&mut self, frame_actions: &FrameActions, output: &mut Vec<NoteEvent<S>>) {
        if frame_actions.play && frame_actions.stop {
            if self.finish_playing(output) {
                self.start_playing(output);
            }
        } else if frame_actions.play {
            self.finish_playing(output);
            self.start_playing(output);
        } else if frame_actions.stop {
            self.finish_playing(output);
        }

        if let Some(playing) = self.playing.as_mut() {
            let normalized_time = playing.time % playing.clip.duration;
            if let Ok(index) = playing
                .clip
                .events
                .binary_search_by(|te| te.time.cmp(&normalized_time))
            {
                let note_events = &playing.clip.events[index].note_events;
                for e in note_events.iter() {
                    playing.voices.handle_event(e);
                    output.push(e.clone());
                }
            }
            if normalized_time == playing.clip.duration - 1 {
                let mut note_offs = vec![];
                playing.voices.gen_events_to_stop_voices(&mut note_offs);
                note_offs
                    .iter()
                    .for_each(|e| playing.voices.handle_event(e));
                output.append(&mut note_offs);
            }
            playing.time += 1;
        }
    }

    pub fn process_sample(
        &mut self,
        events: Vec<NoteEvent<S>>,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let mut output = vec![];
        let (events, actions) = partition_actions(events);
        events
            .iter()
            .for_each(|e| nih_warn!("time={} EVENT {:?}", self.time, e));
        self.process_recording(&actions, &events);
        self.process_playback(&actions, &mut output);
        output.iter().enumerate().for_each(|e| {});
        self.time += 1;
        output
    }
}
