use std::sync::Arc;

use nih_plug::prelude::Enum;

use crate::sampler::{VoiceInfo, WaveformSummary};

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopMode {
    PlayOnce,
    PingPong,
    Loop,
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum RecordingMode {
    #[name = "Only when C-2 is held"]
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
}

impl LoopMode {
    pub fn from_param(param: LoopModeParam) -> Self {
        match param {
            LoopModeParam::PlayOnce => LoopMode::PlayOnce,
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
    pub loop_length_percent: f32,
    pub loop_length_samples: f32,
    pub start_offset_percent: f32,
    pub speed: f32,
    pub recording_mode: RecordingMode,
    pub fixed_size_samples: usize,
    pub transport_pos_samples: Option<i64>,
    pub sample_id: usize,
}

pub const DEFAULT_AUTO_PASSTHRU: bool = true;

impl Default for Params {
    fn default() -> Self {
        Self {
            auto_passthru: DEFAULT_AUTO_PASSTHRU,
            attack_samples: 100,
            loop_mode: LoopMode::Loop,
            loop_length_percent: 1.0,
            loop_length_samples: 0.0,
            start_offset_percent: 0.0,
            decay_samples: 100,
            speed: 1.0,
            recording_mode: RecordingMode::default(),
            fixed_size_samples: 0,
            transport_pos_samples: None,
            sample_id: 0,
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
