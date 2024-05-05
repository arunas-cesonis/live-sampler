use crate::sampler::WaveformSummary;
use crate::time_value::{TimeOrRatio, TimeValue};

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NoteOffBehaviour {
    Decay,
    ZeroCrossing,
    DecayAndZeroCrossing,
}

impl EnumIndex for NoteOffBehaviour {
    const LENGTH: usize = 3;

    fn to_index(&self) -> usize {
        match self {
            NoteOffBehaviour::Decay => 0,
            NoteOffBehaviour::ZeroCrossing => 1,
            NoteOffBehaviour::DecayAndZeroCrossing => 2,
        }
    }
    fn from_index(index: usize) -> Self {
        match index {
            0 => NoteOffBehaviour::Decay,
            1 => NoteOffBehaviour::ZeroCrossing,
            2 => NoteOffBehaviour::DecayAndZeroCrossing,
            _ => panic!("Invalid index for NoteOffBehaviour"),
        }
    }
}

pub trait EnumIndex: Sized {
    const LENGTH: usize;
    fn to_index(&self) -> usize;
    fn from_index(index: usize) -> Self;
    fn to_f32(&self) -> f32 {
        let i = self.to_index() as f32;
        let l = Self::LENGTH as f32;
        ((i / l) + ((i + 1.0) / l)) * 0.5
    }
    fn from_f32(x: f32) -> Self {
        Self::from_index((x * Self::LENGTH as f32).floor() as usize)
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RecordingMode {
    NoteTriggered,
    AlwaysOn,
}
impl EnumIndex for RecordingMode {
    const LENGTH: usize = 2;
    fn to_index(&self) -> usize {
        match self {
            RecordingMode::NoteTriggered => 0,
            RecordingMode::AlwaysOn => 1,
        }
    }
    fn from_index(index: usize) -> Self {
        match index {
            0 => RecordingMode::NoteTriggered,
            1 => RecordingMode::AlwaysOn,
            _ => panic!("Invalid index for RecordingMode"),
        }
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LoopMode {
    PlayOnce,
    Loop,
    PingPong,
}

impl EnumIndex for LoopMode {
    const LENGTH: usize = 3;
    fn to_index(&self) -> usize {
        match self {
            LoopMode::PlayOnce => 0,
            LoopMode::Loop => 1,
            LoopMode::PingPong => 2,
        }
    }
    fn from_index(index: usize) -> Self {
        match index {
            0 => LoopMode::PlayOnce,
            1 => LoopMode::Loop,
            2 => LoopMode::PingPong,
            _ => panic!("Invalid index for LoopMode"),
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     #[test]
//     fn float_conversions() {
//         for i in 0..100 {
//             let x = i as f32 / 100.0;
//             eprintln!(
//                 "x={} loopmode={:?} {:?}",
//                 x,
//                 LoopMode::from_f32(x),
//                 LoopMode::from_f32(x).to_f32()
//             );
//             eprintln!(
//                 "x={} nob={:?} {:?}",
//                 x,
//                 RecordingMode::from_f32(x),
//                 RecordingMode::from_f32(x).to_f32()
//             );
//         }
//     }
// }

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Params {
    pub attack_samples: usize,
    pub decay_samples: usize,
    pub auto_passthru: bool,
    pub loop_mode: LoopMode,
    pub loop_length: TimeOrRatio,
    pub start_offset_percent: f32,
    pub speed: f32,
    pub recording_mode: RecordingMode,
    pub fixed_size_samples: usize,
    pub sample_id: usize,
    pub transport: Transport,
    pub reverse_speed: f32,
    pub note_off_behavior: NoteOffBehaviour,
    pub volume: f32,
}

impl Params {
    pub fn speed(&self) -> f32 {
        self.speed * self.reverse_speed
    }

    pub fn loop_length(&self, data_len: usize) -> f32 {
        let t = &self.transport;
        let length = match self.loop_length {
            TimeOrRatio::Time(time) => match time {
                TimeValue::Samples(samples) => samples,
                TimeValue::Seconds(seconds) => seconds as f32 * t.sample_rate as f32,
                TimeValue::QuarterNotes(quarter_notes) => {
                    let samples_per_quarter_note = t.sample_rate as f32 * 60.0 / t.tempo;
                    quarter_notes as f32 * samples_per_quarter_note
                }
                TimeValue::Bars(bars) => {
                    let samples_per_bar = t.sample_rate as f32 * 60.0 / t.tempo
                        * t.time_sig_numerator as f32
                        / t.time_sig_denominator as f32;
                    bars as f32 * samples_per_bar
                }
            },
            TimeOrRatio::Ratio(ratio) => {
                let len_f32 = data_len as f32;
                len_f32 * ratio
            }
        };
        debug_assert!(
            length > 0.0 || data_len == 0,
            "length={} self={:?}",
            length,
            self
        );
        length.max(1.0)
    }
}

pub const DEFAULT_AUTO_PASSTHRU: bool = true;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Transport {
    pub sample_rate: f32,
    pub tempo: f32,
    pub pos_samples: f32,
    pub time_sig_numerator: u32,
    pub time_sig_denominator: u32,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            tempo: 120.0,
            pos_samples: 0.0,
            time_sig_numerator: 4,
            time_sig_denominator: 4,
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            auto_passthru: DEFAULT_AUTO_PASSTHRU,
            attack_samples: 100,
            loop_mode: LoopMode::Loop,
            loop_length: TimeOrRatio::Ratio(1.0),
            start_offset_percent: 0.0,
            decay_samples: 100,
            speed: 1.0,
            reverse_speed: 1.0,
            recording_mode: RecordingMode::NoteTriggered,
            fixed_size_samples: 0,
            sample_id: 0,
            transport: Transport::default(),
            note_off_behavior: NoteOffBehaviour::DecayAndZeroCrossing,
            volume: 1.0,
        }
    }
}

#[repr(C)]
pub struct InitParams {
    pub auto_passthru: bool,
}

impl Default for InitParams {
    fn default() -> Self {
        InitParams {
            auto_passthru: DEFAULT_AUTO_PASSTHRU,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct VersionedWaveformSummary {
    pub version: usize,
    pub waveform_summary: WaveformSummary,
}

#[derive(Hash, PartialEq, Clone, Copy, Default, Debug)]
pub struct Note {
    pub note: u8,
    pub channel: u8,
}

impl Note {
    pub fn new(note: u8, channel: u8) -> Self {
        debug_assert!(channel <= 15);
        Self { note, channel }
    }

    pub fn into_u64(self) -> u64 {
        (self.note as u64) << 8 | self.channel as u64
    }

    pub fn from_u64(value: u64) -> Self {
        Self {
            note: (value >> 8) as u8,
            channel: value as u8,
        }
    }
}
