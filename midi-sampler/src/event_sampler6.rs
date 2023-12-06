use crate::count_map::CountMap;
use crate::utils::{is_note_off, is_note_on, note_from_event, Note, NoteState};
use intmap::IntMap;
use nih_plug::log::Record;
use nih_plug::midi::NoteEvent;
use nih_plug::nih_warn;
use nih_plug::prelude::ClapFeature::NoteEffect;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::process::Output;
use std::slice::from_raw_parts;
use std::sync::Arc;

// 'time 'in this module is calculated as number of audio frames processed

pub struct Params {
    pub sample_rate: f32, // for better messaging / debugging purposes
    pub pos_beats: Option<f64>,
    pub pos_samples: Option<i64>,
    pub pos_seconds: Option<f64>,
    pub tempo: Option<f64>,
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
struct Voice {
    start: usize,
    count: usize,
}

#[derive(Debug, Default)]
struct Voices {
    //m: CountMap<Note>,
    map: IntMap<Voice>,
}

impl Voices {
    fn log(&self, time: usize, msg: &str) {
        nih_warn!("time={} {}", time, msg);
    }

    fn voice_on(&mut self, time: usize, note: Note) {
        if let Some(voice) = self.map.get_mut(note.into()) {
            voice.count += 1;
        } else {
            self.map.insert(
                note.into(),
                Voice {
                    start: time,
                    count: 1,
                },
            );
        }
    }
    fn voice_off(&mut self, time: usize, note: Note) {
        if let Some(voice) = self.map.get_mut(note.into()) {
            let debug_duration = time - voice.start;
            log(
                time,
                format!("voice off: lasted {:?} samples", debug_duration).as_str(),
            );
            voice.count -= 1;
            if voice.count == 0 {
                self.map.remove(note.into());
            }
        }
    }
    fn handle_event<S>(&mut self, time: usize, e: &NoteEvent<S>) {
        match note_from_event(e) {
            Some((note, NoteState::On)) => {
                self.voice_on(time, note);
            }
            Some((note, NoteState::Off)) => {
                self.voice_off(time, note);
            }
            None => (),
        };
    }
    fn gen_events_to_stop_voices<S>(&mut self, time: usize, output: &mut Vec<NoteEvent<S>>) {
        let mut tmp = vec![];
        for (key, voice) in self.map.iter() {
            let note: Note = (*key).into();
            assert!(voice.count > 0);
            tmp.push(NoteEvent::NoteOff {
                timing: 0, // this bit is set in the Plugin implementation
                voice_id: None,
                channel: note.channel,
                note: note.note,
                velocity: 0.0,
            });
        }
        tmp.iter().for_each(|e| self.handle_event(time, e));
        output.append(&mut tmp);
    }
}

fn log(time: usize, msg: &str) {
    nih_warn!("time={} {}", time, msg);
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
        if let Some(mut playing) = self.playing.take() {
            playing.voices.gen_events_to_stop_voices(self.time, output);
        }
    }

    fn log(&self, msg: &str) {
        log(self.time, msg);
    }

    fn start_recording(&mut self) {
        let mut clip = Clip::default();
        self.recording = Some(clip);
    }

    fn finish_recording(&mut self) -> bool {
        if let Some(clip) = self.recording.take() {
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
        if let Some(mut playing) = self.playing.take() {
            playing.voices.gen_events_to_stop_voices(self.time, output);
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
                    playing.voices.handle_event(self.time, e);
                    output.push(e.clone());
                }
            }
            if normalized_time == playing.clip.duration - 1 {
                let mut note_offs = vec![];
                playing
                    .voices
                    .gen_events_to_stop_voices(self.time, &mut note_offs);
                output.append(&mut note_offs);
            }
            playing.time += 1;
        }
    }

    fn print_times(&self, params: &Params) -> String {
        format!(
            "time={} bpm={} beats={} smp={} t={}",
            self.time,
            params.tempo.unwrap_or(0.0),
            params.pos_beats.unwrap_or(0.0),
            params.pos_samples.unwrap_or(0),
            params.pos_seconds.unwrap_or(0.0)
        )
    }

    pub fn process_sample(
        &mut self,
        events: Vec<NoteEvent<S>>,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let mut output = vec![];
        for e in &events {
            self.log(format!("{} EVENT {:?}", self.print_times(params), e).as_str());
        }
        let (events, actions) = partition_actions(events);
        self.process_recording(&actions, &events);
        self.process_playback(&actions, &mut output);
        output.iter().enumerate().for_each(|e| {});
        self.time += 1;
        output
    }
}
