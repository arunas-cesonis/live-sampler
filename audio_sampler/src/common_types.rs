use std::sync::Arc;

use nih_plug::prelude::Enum;

use crate::sampler::{VoiceInfo, WaveformSummary};
use crate::time_value::{TimeOrRatio, TimeUnit, TimeValue};

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopMode {
    PlayOnce,
    PingPong,
    Loop,
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum MIDIChannelParam {
    #[name = "All"]
    All,
    #[name = "1"]
    Channel1,
    #[name = "2"]
    Channel2,
    #[name = "3"]
    Channel3,
    #[name = "4"]
    Channel4,
    #[name = "5"]
    Channel5,
    #[name = "6"]
    Channel6,
    #[name = "7"]
    Channel7,
    #[name = "8"]
    Channel8,
    #[name = "9"]
    Channel9,
    #[name = "10"]
    Channel10,
    #[name = "11"]
    Channel11,
    #[name = "12"]
    Channel12,
    #[name = "13"]
    Channel13,
    #[name = "14"]
    Channel14,
    #[name = "15"]
    Channel15,
    #[name = "16"]
    Channel16,
}

impl TryInto<u8> for MIDIChannelParam {
    type Error = ();
    fn try_into(self) -> Result<u8, ()> {
        Ok(match self {
            MIDIChannelParam::All => return Err(()),
            MIDIChannelParam::Channel1 => 0,
            MIDIChannelParam::Channel2 => 1,
            MIDIChannelParam::Channel3 => 2,
            MIDIChannelParam::Channel4 => 3,
            MIDIChannelParam::Channel5 => 4,
            MIDIChannelParam::Channel6 => 5,
            MIDIChannelParam::Channel7 => 6,
            MIDIChannelParam::Channel8 => 7,
            MIDIChannelParam::Channel9 => 8,
            MIDIChannelParam::Channel10 => 9,
            MIDIChannelParam::Channel11 => 10,
            MIDIChannelParam::Channel12 => 11,
            MIDIChannelParam::Channel13 => 12,
            MIDIChannelParam::Channel14 => 13,
            MIDIChannelParam::Channel15 => 14,
            MIDIChannelParam::Channel16 => 15,
        })
    }
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum RecordingMode {
    #[name = "Only when C-2 is on"]
    NoteTriggered,
    #[name = "Always (last bar)"]
    AlwaysOn,
}

impl Default for RecordingMode {
    fn default() -> Self {
        RecordingMode::NoteTriggered
    }
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopModeParam {
    #[name = "Play once and stop"]
    PlayOnce,
    #[name = "Loop"]
    Loop,
    // #[name = "Ping pong"]
    // PingPong,
}

impl LoopMode {
    pub fn from_param(param: LoopModeParam) -> Self {
        match param {
            LoopModeParam::PlayOnce => LoopMode::PlayOnce,
            //            LoopModeParam::PingPong => LoopMode::PingPong,
            LoopModeParam::Loop => LoopMode::Loop,
        }
    }
}

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
}

pub const DEFAULT_AUTO_PASSTHRU: bool = true;

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
            recording_mode: RecordingMode::default(),
            fixed_size_samples: 0,
            sample_id: 0,
            transport: Transport::default(),
        }
    }
}

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

#[derive(Clone, Default, Debug)]
pub struct Info {
    pub voices: Vec<VoiceInfo>,
    pub last_recorded_indices: Vec<Option<usize>>,
    pub data_len: usize,
    pub waveform_summary: Arc<VersionedWaveformSummary>,
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
