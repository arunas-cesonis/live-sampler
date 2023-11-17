use nih_plug::prelude::{NoteEvent, SysExMessage};
use nih_plug::{nih_error, nih_warn};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct RecordedEvent<S> {
    original: NoteEvent<S>,
    time_from_start: usize,
}

#[derive(Clone, Debug)]
struct RecordedEvents<S> {
    data: Vec<RecordedEvent<S>>,
    duration: usize,
}

#[derive(Clone, Debug)]
struct Voice<S> {
    events: Arc<RecordedEvents<S>>,
    start: usize,
    note: u8,
    i: usize,
}

impl<S> Voice<S>
where
    S: Clone,
{
    fn new(now: usize, note: u8, events: Arc<RecordedEvents<S>>) -> Self {
        assert!(!events.data.is_empty());
        Self {
            start: now,
            note,
            events,
            i: 0,
        }
    }
    fn stop(&self, now: usize) -> Vec<NoteEvent<S>> {
        nih_error!("not pushing note offs on early stop!");
        vec![]
    }
    fn process_sample(
        &mut self,
        now: usize,
        sample_id: usize,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        let mut c = 0;
        let mut output = vec![];
        loop {
            if self.i == self.events.data.len() {
                if now - self.start == self.events.duration {
                    self.i = 0;
                    c += 1;
                    if c > 1000 {
                        panic!("max events per sample reached");
                    }
                    continue;
                }
                break;
            } else {
                assert!(self.i < self.events.data.len());
                if self.events.data[self.i].time_from_start == now - self.start {
                    let mut ev = self.events.data[self.i].original.clone();
                    note_event_set_timing(&mut ev, sample_id as u32);
                    output.push(ev);
                    self.i += 1;
                    c += 1;
                    if c > 1000 {
                        panic!("max events per sample reached");
                    }
                } else {
                    break;
                }
            }
        }
        output
    }
}

#[derive(Clone, Debug, Default)]
pub struct EventSampler<S> {
    data: Vec<RecordedEvent<S>>,
    // The offsets and length are calculated in audio samples
    // A more midi-like approach would be to use midi clock maybe
    last_recording: Option<Arc<RecordedEvents<S>>>,
    voices: Vec<Voice<S>>,
    recording_start: usize,
    recording: bool,
    reverse: bool,
    now: usize,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub sample_rate: f32,
    pub now: usize,
}

fn note_event_set_timing<S>(ev: &mut NoteEvent<S>, value: u32) {
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

impl<S> EventSampler<S>
where
    S: SysExMessage,
{
    pub fn start_recording(&mut self, params: &Params) {
        nih_warn!("now={} start_recording", params.now);
        assert!(self.voices.len() == 0);
        if !self.recording {
            self.recording_start = self.now;
            self.recording = true;
            self.data.clear();
        }
    }
    pub fn stop_recording(&mut self, params: &Params) {
        nih_warn!("now={} stop_recording", params.now);
        // This is not an error as some DAWs will send note off events for notes
        // that were never played, e.g. REAPER
        if self.recording {
            self.recording = false;
            let data = std::mem::take(&mut self.data);
            self.last_recording = Some(Arc::new(RecordedEvents {
                data: self.data.take(),
                duration: self.now - self.recording_start,
            }));
            let r = self.last_recording.clone().unwrap();
            nih_warn!("recorded {} events", r.data.len());
            nih_warn!(
                "recorded {:.3}s duration",
                params.sample_rate * (*(&self.last_recording.as_ref().unwrap().duration) as f32)
            );
            for (i, e) in r.data.iter().enumerate() {
                let seconds = (e.time_from_start as f32 / params.sample_rate);
                nih_warn!(
                    "now={} {:<4} t={:.3}s time={} {:?}",
                    params.now,
                    i,
                    seconds,
                    e.time_from_start,
                    e
                );
            }
        }
    }
    pub fn start_reversing(&mut self, params: &Params) {}
    pub fn stop_reversing(&mut self, params: &Params) {}

    pub fn start_playing(
        &mut self,
        pos: f32,
        note: u8,
        velocity: f32,
        params: &Params,
    ) -> Vec<NoteEvent<S>> {
        nih_warn!(
            "now={} start_playing {} {} {}",
            params.now,
            pos,
            note,
            velocity
        );
        assert!(!self.recording);
        let mut output = vec![];
        if let Some(r) = &self.last_recording {
            if let Some(existing_i) = self.voices.iter().position(|v| v.note == note) {
                output.append(&mut self.voices[existing_i].stop(params.now));
                self.voices.remove(existing_i);
            }
            let voice = Voice::new(self.now, note, r.clone());
            self.voices.push(voice);
        }
        output
    }

    pub fn stop_playing(&mut self, note: u8, params: &Params) -> Vec<NoteEvent<S>> {
        nih_warn!("now={} stop_playing {}", params.now, note);
        let mut output = vec![];
        if let Some(existing_i) = self.voices.iter().position(|v| v.note == note) {
            output.append(&mut self.voices[existing_i].stop(params.now));
            self.voices.remove(existing_i);
        }
        output
    }

    pub fn process_sample<'a>(&mut self, sample_id: usize, params: &Params) -> Vec<NoteEvent<S>> {
        let mut output = vec![];
        for v in &mut self.voices {
            let mut tmp = v.process_sample(self.now, sample_id, params);
            output.append(&mut tmp);
        }
        self.now += 1;
        output
    }
    pub fn handle_event(&mut self, event: &NoteEvent<S>, params: &Params) {
        if self.recording {
            self.data.push(RecordedEvent {
                original: event.clone(),
                time_from_start: self.now - self.recording_start,
            });
        }
    }
}
