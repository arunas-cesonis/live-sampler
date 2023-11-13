use nih_plug::nih_warn;
use nih_plug_vizia::vizia::image::flat::Error::ChannelCountMismatch;
use std::collections::HashMap;

fn calc_sample_pos(data_len: usize, read: f32) -> usize {
    let len_f32 = (data_len as f32);
    let i = read % len_f32;
    let i = if i < 0.0 { i + len_f32 } else { i };
    let i = i as usize;
    i
}

#[derive(Clone, Debug, Default)]
struct Voice {
    read: f32,
    volume: f32,
    speed: f32,
    note_on_count: usize,
}

#[derive(Clone, Debug, Default)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    read: f32,
    voices: HashMap<u8, Voice>,
    recording: bool,
    playing: bool,
    now: usize,
    note_on_count: usize,
}

#[derive(Debug, Clone)]
pub struct Params {
    pub auto_passthru: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            auto_passthru: true,
        }
    }
}

impl Channel {
    pub fn start_playing(&mut self, pos: f32, note: u8, velocity: f32) {
        // NoteOn for same key being sent twice or more before NoteOff
        // resets the current voice of that key.
        // It could be useful to spawn another voice instead for to allow delay-like effects,
        // but not sure if its even possible to enter such notation in piano-roll like editors.
        let current_voice_note_on_count =
            self.voices.get(&note).map(|v| v.note_on_count).unwrap_or(0);
        self.voices.insert(
            note,
            Voice {
                read: (pos * self.data.len() as f32).round(),
                volume: velocity,
                speed: 1.0,
                // note on count is used to detect if the note is still being played
                // in stop_playing()
                note_on_count: current_voice_note_on_count + 1,
            },
        );
        //self.read = (pos * self.data.len() as f32).round() as f32;
        self.note_on_count += 1;
        nih_warn!(
            "** START PLAYING voice #{} for {}",
            current_voice_note_on_count + 1,
            note
        );
    }

    pub fn stop_playing(&mut self, note: u8) {
        // Can be done in some DAW's when opening a project and playhead was saved
        // positioned within a note
        // assert!(self.voices.contains_key(&note));

        let should_remove = if let Some(voice) = self.voices.get_mut(&note) {
            if voice.note_on_count == 1 {
                true
            } else {
                assert!(voice.note_on_count > 1);
                voice.note_on_count -= 1;
                false
            }
        } else {
            nih_warn!("** STOP PLAYING: voice {note} not found");
            false
        };
        if should_remove {
            self.voices.remove(&note);
        }
        self.note_on_count -= 1;
        let tmp = self
            .voices
            .iter()
            .map(|v| format!("note={} n={}", v.0, v.1.note_on_count))
            .collect::<Vec<_>>();
        nih_warn!(
            "** STOP PLAYING: voice {note}: voices remaining: {:?}",
            tmp.join(", ")
        );
        //if !voice.is_some() {
        //}
        //if self.playing {
        //    self.playing = false;
        //    nih_warn!("** STOP PLAYING");
        //} else {
        //    nih_warn!("** STOP PLAYING: ALREADY");
        //}
    }

    pub fn start_recording(&mut self) {
        assert!(!self.recording);
        assert!(self.write == 0);
        self.recording = true;
        nih_warn!("** START RECORIDNG");
    }

    pub fn stop_recording(&mut self) {
        if self.recording {
            self.recording = false;
            self.data.truncate(self.write);
            self.write = 0;
            nih_warn!("** STOP RECORIDNG (wrote {} samples)", self.data.len());
        } else {
            nih_warn!("** STOP RECORDING: ALREADY STOPPED");
        }
    }

    pub fn process_sample<'a>(&mut self, sample: &mut f32, params: &Params) {
        let value = *sample;

        if self.recording {
            assert!(self.write <= self.data.len());
            if self.write == self.data.len() {
                self.data.push(value);
            } else {
                self.data[self.write] = value;
            }
            self.write += 1;
        }

        let mut output = 0.0;
        if !self.voices.is_empty() {
            for (_, voice) in self.voices.iter_mut() {
                if !self.data.is_empty() {
                    let y = self.data[calc_sample_pos(self.data.len(), voice.read)];
                    output += y * voice.volume;
                    voice.read += voice.speed;
                };
            }
        } else {
            if params.auto_passthru {
                output += value;
            }
        }
        //if self.playing {
        //    let data_value = if self.data.is_empty() {
        //        0.0
        //    } else {
        //        self.data[calc_sample_pos(self.data.len(), self.read)]
        //    };
        //    output += data_value;
        //    self.read += 1.0;
        //} else {
        //    output += value;
        //}
        self.now += 1;
        *sample = output;
    }
}

#[derive(Debug)]
pub struct Sampler {
    channels: Vec<Channel>,
}

impl Sampler {
    pub fn new(channel_count: usize) -> Self {
        Self {
            channels: vec![Channel::default(); channel_count],
        }
    }
    fn each<F>(&mut self, f: F)
    where
        F: FnMut(&mut Channel),
    {
        self.channels.iter_mut().for_each(f)
    }
    pub fn start_playing(&mut self, pos: f32, note: u8, velocity: f32) {
        self.each(|ch| ch.start_playing(pos, note, velocity));
    }

    pub fn stop_playing(&mut self, note: u8) {
        self.each(|ch| ch.stop_playing(note));
    }

    pub fn start_recording(&mut self) {
        self.each(Channel::start_recording);
    }

    pub fn stop_recording(&mut self) {
        self.each(Channel::stop_recording);
    }

    pub fn process_sample<'a>(
        &mut self,
        iter: impl IntoIterator<Item = &'a mut f32>,
        params: &Params,
    ) {
        for (i, sample) in iter.into_iter().enumerate() {
            self.channels[i].process_sample(sample, params);
        }
    }
}
