use std::convert::TryInto;
use std::sync::Arc;
use nih_plug::prelude::Enum;
use audio_sampler_lib::common_types::{LoopMode, NoteOffBehaviour, VersionedWaveformSummary};
use audio_sampler_lib::sampler::VoiceInfo;

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
pub enum LoopModeParam {
    PlayOnce,
    PingPong,
    Loop,
}

impl From<LoopModeParam> for LoopMode {
    fn from(param: LoopModeParam) -> Self {
        match param {
            LoopModeParam::PlayOnce => LoopMode::PlayOnce,
            LoopModeParam::PingPong => LoopMode::PingPong,
            LoopModeParam::Loop => LoopMode::Loop,
        }
    }
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum TimeUnitParam {
    #[name = "1/16 notes"]
    SixteenthNotes,
    #[name = "1/4 notes"]
    QuarterNotes,
    #[name = "Seconds"]
    Seconds,
    #[name = "Samples"]
    Samples,
    #[name = "Bars"]
    Bars,
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum NoteOffBehaviourParam {
    #[name = "Decay"]
    Decay,
    #[name = "Zero crossing"]
    ZeroCrossing,
    #[name = "Decay and zero crossing"]
    DecayAndZeroCrossing,
}

impl From<NoteOffBehaviourParam> for NoteOffBehaviour {
    fn from(param: NoteOffBehaviourParam) -> Self {
        match param {
            NoteOffBehaviourParam::Decay => NoteOffBehaviour::Decay,
            NoteOffBehaviourParam::ZeroCrossing => NoteOffBehaviour::ZeroCrossing,
            NoteOffBehaviourParam::DecayAndZeroCrossing => NoteOffBehaviour::DecayAndZeroCrossing,
        }
    }
}

#[derive(Debug, Enum, PartialEq, Clone, Copy)]
pub enum TimeOrRatioUnitParam {
    #[name = "1/16 notes"]
    SixteenthNotes,
    #[name = "Seconds"]
    Seconds,
    #[name = "Percentage of length"]
    Ratio,
}


#[derive(Clone, Default, Debug)]
pub struct Info {
    pub voices: Vec<VoiceInfo>,
    pub last_recorded_indices: Vec<Option<usize>>,
    pub data_len: usize,
    pub waveform_summary: Arc<VersionedWaveformSummary>,
}

impl Default for NoteOffBehaviourParam {
    fn default() -> Self {
        NoteOffBehaviourParam::DecayAndZeroCrossing
    }
}
