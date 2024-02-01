use crate::sampler::VoiceInfo;
use nih_plug::prelude::Enum;
use std::sync::Arc;

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopMode {
    PlayOnce,
    PingPong,
    Loop,
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum LoopModeParam {
    PlayOnce,
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
    pub speed: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            auto_passthru: true,
            attack_samples: 100,
            loop_mode: LoopMode::PlayOnce,
            loop_length_percent: 1.0,
            decay_samples: 100,
            speed: 1.0,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct WaveformSummary {
    pub version: usize,
    pub data: Vec<f32>,
}

#[derive(Clone, Default, Debug)]
pub struct Info {
    pub voices: Vec<VoiceInfo>,
    pub waveform_summary: Arc<WaveformSummary>,
}
