use crate::utils::{note_from_event, Note, NoteState};
use intmap::IntMap;

use nih_plug::midi::NoteEvent;
use nih_plug::nih_warn;

use std::fmt::Debug;

use std::sync::Arc;

// 'time 'in this module is calculated as number of audio frames processed

pub struct Params {
    pub sample_rate: f32, // for better messaging / debugging purposes
    pub passthru: bool,
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
    fn to_notes_with_durations(&self) -> Vec<(Note, usize)> {
        let mut out = vec![];
        let mut voices = Voices::default();
        let mut last_time = 0;
        for e in &self.events {
            match e {
                TimedEvents { note_events, time } => note_events.iter().for_each(|e| {
                    last_time = *time;
                    match note_from_event(e) {
                        Some((note, NoteState::On)) => voices.voice_on(*time, note),
                        Some((note, NoteState::Off)) => {
                            if let Some(voice) = voices.voice_off(*time, note) {
                                out.push((note, *time - voice.start));
                            }
                        }
                        None => {}
                    }
                }),
            }
        }
        for x in voices.gen_events_to_stop_voices::<()>() {
            let time = last_time + 1;
            for v in voices.handle_event(time, &x) {
                match note_from_event(&x) {
                    Some((note, NoteState::On)) => voices.voice_on(time, note),
                    Some((note, NoteState::Off)) => {
                        if let Some(voice) = voices.voice_off(time, note) {
                            out.push((note, time - voice.start));
                        }
                    }
                    None => {}
                }
            }
        }
        out
    }
    fn count_events(&self) -> usize {
        self.events
            .iter()
            .map(|e| e.note_events.len())
            .sum::<usize>()
    }
    fn push_events<'a, I>(&mut self, _time: usize, note_events: I)
    where
        I: IntoIterator<Item = &'a NoteEvent<S>> + 'a,
        S: 'a + Clone + Debug,
    {
        let note_events: Vec<_> = note_events.into_iter().cloned().collect();
        if note_events.is_empty() {
            return;
        }
        self.events.push(TimedEvents {
            note_events,
            time: self.duration,
        });
    }
}

#[derive(Debug)]
struct Playing<S> {
    clip: Arc<Clip<S>>,
    time: usize,
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
    play_from: Option<f32>,
    stop: bool,
}

const BASE_NOTE: u8 = 12;

fn partition_actions<S>(ev: Vec<NoteEvent<S>>) -> (Vec<NoteEvent<S>>, FrameActions) {
    let mut out_events = vec![];
    let mut frame_actions = FrameActions::default();
    for e in ev {
        match e {
            NoteEvent::NoteOn { note, .. } if note == 0 => frame_actions.start_recording = true,
            NoteEvent::NoteOff { note, .. } if note == 0 => frame_actions.stop_recording = true,
            NoteEvent::NoteOn { note, .. } if note >= BASE_NOTE && note < BASE_NOTE + 16 => {
                frame_actions.play_from = Some((note - BASE_NOTE) as f32 / 16.0)
            }
            NoteEvent::NoteOff { note, .. } if note >= BASE_NOTE && note < BASE_NOTE + 16 => {
                frame_actions.stop = true;
            }
            _ => {
                out_events.push(e);
            }
        };
    }
    (out_events, frame_actions)
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
    fn voice_off(&mut self, time: usize, note: Note) -> Option<Voice> {
        if let Some(voice) = self.map.get_mut(note.into()) {
            voice.count -= 1;
            if voice.count == 0 {
                self.map.remove(note.into())
            } else {
                None
            }
        } else {
            None
        }
    }
    fn handle_event<S>(&mut self, time: usize, e: &NoteEvent<S>) -> Vec<Voice> {
        let mut out = vec![];
        match note_from_event(e) {
            Some((note, NoteState::On)) => {
                self.voice_on(time, note);
            }
            Some((note, NoteState::Off)) => {
                self.voice_off(time, note)
                    .into_iter()
                    .for_each(|v| out.push(v));
            }
            None => (),
        };
        out
    }
    fn gen_events_to_stop_voices<S>(&mut self) -> Vec<NoteEvent<S>> {
        let mut events = vec![];
        for (key, voice) in self.map.iter() {
            let note: Note = (*key).into();
            assert!(voice.count > 0);
            events.push(NoteEvent::NoteOff {
                timing: 0, // this value is set in the Plugin implementation
                voice_id: None,
                channel: note.channel,
                note: note.note,
                velocity: 0.0,
            });
        }
        events
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
    fn start_recording(&mut self) {
        let clip = Clip::default();
        self.recording = Some(clip);
    }

    fn finish_recording(&mut self) -> bool {
        if let Some(clip) = self.recording.take() {
            nih_warn!(
                "{:<8} FINISHED RECORDING duration={}",
                self.time,
                clip.duration
            );
            nih_warn!(
                "{:<8} FINISHED RECORDING events={:?}",
                self.time,
                clip.to_notes_with_durations()
            );
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
            nih_warn!(
                "{:<8} FINISHED PLAYING clip.time={}",
                self.time,
                playing.time
            );
            for x in playing.voices.gen_events_to_stop_voices() {
                playing.voices.handle_event(self.time, &x);
                output.push(x);
            }
            true
        } else {
            false
        }
    }

    fn start_playing(&mut self, play_from: f32) {
        if let Some(clip) = &self.stored {
            let time = (clip.duration as f32 * play_from).round() as usize;
            nih_warn!(
                "{:<8} START PLAYING clip.duration={} clip.time={}",
                self.time,
                clip.duration,
                time
            );
            self.playing = Some(Playing {
                clip: clip.clone(),
                time,
                voices: Voices::default(),
            });
        }
    }

    fn process_playback(&mut self, frame_actions: &FrameActions, output: &mut Vec<NoteEvent<S>>) {
        if frame_actions.play_from.is_some() && frame_actions.stop {
            let play_from = frame_actions.play_from.unwrap();
            if self.finish_playing(output) {
                self.start_playing(play_from);
            }
        } else if let Some(play_from) = frame_actions.play_from {
            self.finish_playing(output);
            self.start_playing(play_from);
        } else if frame_actions.stop {
            self.finish_playing(output);
        }

        if let Some(playing) = self.playing.as_mut() {
            let normalized_time = playing.time % playing.clip.duration;
            // TODO: instead of binary search keep track of last index and only search from there
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
                for x in playing.voices.gen_events_to_stop_voices() {
                    playing.voices.handle_event(self.time, &x);
                    output.push(x);
                }
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
        for e in &events {
            nih_warn!("{:<8} IN  EVENT {:?}", self.time, note_from_event(e));
        }
        let (mut events, actions) = partition_actions(events);
        self.process_recording(&actions, &events);
        self.process_playback(&actions, &mut output);
        if params.passthru {
            output.append(&mut events);
        }
        for e in &output {
            nih_warn!("{:<8} OUT EVENT {:?}", self.time, note_from_event(e));
        }
        self.time += 1;
        output
    }
}
