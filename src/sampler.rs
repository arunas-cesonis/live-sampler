use nih_plug::nih_warn;
use nih_plug_vizia::vizia::image::flat::Error::ChannelCountMismatch;

fn calc_sample_pos(data_len: usize, read: f32) -> usize {
    let len_f32 = (data_len as f32);
    let i = read % len_f32;
    let i = if i < 0.0 { i + len_f32 } else { i };
    let i = i as usize;
    i
}

#[derive(Clone, Debug, Default)]
struct Channel {
    data: Vec<f32>,
    write: usize,
    read: f32,
    recording: bool,
    playing: bool,
    now: usize,
}

impl Channel {
    pub fn start_playing(&mut self, pos: f32) {
        assert!(!self.playing);
        self.playing = true;
        self.read = (pos * self.data.len() as f32).round() as f32;
        nih_warn!("** START PLAYING");
    }

    pub fn stop_playing(&mut self) {
        if self.playing {
            self.playing = false;
            nih_warn!("** STOP PLAYING");
        } else {
            nih_warn!("** STOP PLAYING: ALREADY");
        }
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
            nih_warn!("** STOP RECORDING");
        } else {
            nih_warn!("** STOP RECORDING: ALREADY");
        }
        nih_warn!("** STOP RECORIDNG (got {} samples)", self.data.len());
    }

    pub fn process_sample<'a>(&mut self, sample: &mut f32) {
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
        if self.playing {
            let data_value = if self.data.is_empty() {
                0.0
            } else {
                self.data[calc_sample_pos(self.data.len(), self.read)]
            };
            output += data_value;
            self.read += 1.0;
        } else {
            output += value;
        }
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
    pub fn start_playing(&mut self, pos: f32) {
        self.each(|ch| ch.start_playing(pos));
    }

    pub fn stop_playing(&mut self) {
        self.each(Channel::stop_playing);
    }

    pub fn start_recording(&mut self) {
        self.each(Channel::start_recording);
    }

    pub fn stop_recording(&mut self) {
        self.each(Channel::stop_recording);
    }

    pub fn process_sample<'a>(&mut self, iter: impl IntoIterator<Item = &'a mut f32>) {
        for (i, sample) in iter.into_iter().enumerate() {
            self.channels[i].process_sample(sample);
        }
    }
}
