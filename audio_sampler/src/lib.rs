#![allow(unused)]
#![feature(extend_one)]

use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_vizia::vizia::entity;
use nih_plug_vizia::vizia::prelude::Role::Time;
use nih_plug_vizia::ViziaState;
use num_traits::ToPrimitive;

use crate::common_types::{
    ClipVersion, Info, InitParams, LoopModeParam, MIDIChannelParam, Note, Params as SamplerParams,
    RecordingMode, VersionedWaveformSummary,
};
use crate::editor_vizia::DebugData;
use crate::sampler::{LoopMode, Sampler};
use crate::time_value::{calc_samples_per_bar, TimeOrRatio, TimeOrRatioUnit, TimeUnit, TimeValue};
use crate::utils::normalize_offset;

// mod editor;
mod clip;
mod clip2;
mod common_types;
mod editor_vizia;
mod recorder;
mod sampler;
mod test_sampler;
mod time_value;
mod utils;
mod voice;
mod volume;

type SysEx = ();

pub struct AudioSampler {
    audio_io_layout: AudioIOLayout,
    params: Arc<AudioSamplerParams>,
    sample_rate: f32,
    sampler: Sampler,
    peak_meter: Arc<AtomicF32>,
    debug_data_in: Arc<parking_lot::Mutex<triple_buffer::Input<DebugData>>>,
    debug_data_out: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    peak_meter_decay_weight: f32,
    waveform_summary: Arc<VersionedWaveformSummary>,
    last_frame_recorded: usize,
    last_waveform_updated: usize,
    active_notes: [[i16; 256]; 16],
    reversing: bool,
}

const PEAK_METER_DECAY_MS: f64 = 150.0;

impl Plugin for AudioSampler {
    const NAME: &'static str = "Audio Sampler";
    const VENDOR: &'static str = "seunje";
    const URL: &'static str = "https://github.com/arunas-cesonis/live-sampler";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = SysEx;

    type BackgroundTask = ();

    fn reset(&mut self) {
        self.last_waveform_updated = 0;
        self.last_frame_recorded = 0;
        self.sampler.reset();
        self.active_notes.iter_mut().for_each(|v| v.fill(0));
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Using vizia as Iced doesn't support drawing bitmap images under OpenGL

        let data = editor_vizia::Data {
            params: self.params.clone(),
            debug_data_out: self.debug_data_out.clone(),
            xy: (0.0, 0.0),
            y: 0.0,
            x: 0.0,
            peak_meter: self.peak_meter.clone(),
        };

        editor_vizia::create(self.params.editor_state.clone(), data)
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.audio_io_layout = audio_io_layout.clone();
        self.sample_rate = buffer_config.sample_rate;
        self.sampler = Sampler::new(self.channel_count(), &InitParams::default());
        self.peak_meter_decay_weight = 0.25f64
            .powf((buffer_config.sample_rate as f64 * PEAK_METER_DECAY_MS / 1000.0).recip())
            as f32;
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            let mut params = self.sampler_params(sample_id, &context.transport());
            let channel: Option<u8> = self.params.midi_channel.value().try_into().ok();
            let params = &mut params;
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                #[cfg(debug_assertions)]
                nih_warn!("event: {:?}", event);
                match event {
                    NoteEvent::NoteOn {
                        velocity,
                        note,
                        channel: note_channel,
                        ..
                    } if channel.iter().all(|x| *x == note_channel) => {
                        let note = Note::new(note, note_channel);
                        match note.note {
                            0 => {
                                self.set_note_active(&note, true);
                                self.sampler.start_recording(params);
                            }
                            1 => {
                                self.set_note_active(&note, true);
                                self.reversing = true;
                                params.reverse_speed = -1.0;
                            }
                            12..=27 => {
                                self.set_note_active(&note, true);
                                let pos = (note.note - 12) as f32 / 16.0;
                                self.sampler.start_playing(pos, note, velocity, params);
                            }
                            _ => (),
                        };
                    }
                    NoteEvent::NoteOff {
                        note,
                        channel: note_channel,
                        ..
                    } => {
                        let note = Note::new(note, note_channel);
                        if self.is_note_active(&note) {
                            match note.note {
                                0 => self.sampler.stop_recording(params),
                                1 => {
                                    self.reversing = false;
                                    params.reverse_speed = 1.0;
                                }
                                12..=27 => self.sampler.stop_playing(note, params),
                                _ => (),
                            }
                            self.set_note_active(&note, false);
                        }
                    }
                    _ => (),
                }

                #[cfg(debug_assertions)]
                self.verify_active_notes();

                next_event = context.next_event();
            }

            if self.sampler.is_recording() {
                self.last_frame_recorded = self.sampler.get_frames_processed();
            }

            let mut frame = channel_samples.into_iter().collect::<Vec<_>>();
            self.sampler.process_frame(&mut frame, params);

            //for sample in channel_samples {
            //    amplitude += *sample;
            //}
            if self.params.editor_state.is_open() {
                self.update_peak_meter(&mut frame);

                if self.last_frame_recorded > self.last_waveform_updated + self.sample_rate as usize
                {
                    self.update_waveform();
                    self.last_waveform_updated = self.last_frame_recorded;
                }
                let voice_info = self.sampler.get_voice_info(params);
                let info = Info {
                    voices: voice_info,
                    last_recorded_indices: self.sampler.get_last_recorded_offsets(),
                    data_len: self.sampler.get_data_len(),
                    waveform_summary: self.waveform_summary.clone(),
                };
                self.debug_data_in.lock().write(DebugData { info });

                let amplitude = (frame.iter().fold(0.0, |z, x| z + **x) / frame.len() as f32).abs();

                let current_peak_meter = self.peak_meter.load(std::sync::atomic::Ordering::Relaxed);
                let new_peak_meter = if amplitude > current_peak_meter {
                    amplitude
                } else {
                    current_peak_meter * self.peak_meter_decay_weight
                        + amplitude * (1.0 - self.peak_meter_decay_weight)
                };

                self.peak_meter
                    .store(new_peak_meter, std::sync::atomic::Ordering::Relaxed)
            }
        }

        ProcessStatus::Normal
    }
}

#[derive(Params)]
pub struct AudioSamplerParams {
    #[id = "auto_passthru"]
    pub auto_passthru: BoolParam,
    #[id = "speed"]
    pub speed: FloatParam,
    #[id = "clip_version"]
    pub clip_version: EnumParam<ClipVersion>,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "loop_mode"]
    pub loop_mode: EnumParam<LoopModeParam>,
    #[id = "loop_length_percent"]
    pub loop_length_percent: FloatParam,
    #[id = "loop_length_time"]
    pub loop_length_time: FloatParam,
    #[id = "loop_length_sync"]
    pub loop_length_sync: FloatParam,
    #[id = "start_offset"]
    pub loop_length_unit: EnumParam<TimeOrRatioUnit>,
    #[id = "loop_length_unit"]
    pub start_offset: FloatParam,
    #[id = "volume"]
    pub volume: FloatParam,
    #[id = "recording_mode"]
    pub recording_mode: EnumParam<RecordingMode>,
    #[id = "midi_channel"]
    pub midi_channel: EnumParam<MIDIChannelParam>,
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

const ATTACK_DECAY_SKEW_FACTOR: f32 = 0.25;
const LOOP_LENGTH_SKEW_FACTOR: f32 = 0.5;
const LOOP_LENGTH_SKEW_SYNC: f32 = 0.25;

impl Default for AudioSamplerParams {
    fn default() -> Self {
        Self {
            editor_state: editor_vizia::default_state(),
            auto_passthru: BoolParam::new("Pass through", true),
            speed: FloatParam::new(
                "Speed",
                1.0,
                FloatRange::Linear {
                    min: -2.0,
                    max: 2.0,
                },
            ),
            attack: FloatParam::new(
                "Attack",
                0.1,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1000.0,
                    factor: ATTACK_DECAY_SKEW_FACTOR,
                },
            )
            .with_unit(" ms"),
            decay: FloatParam::new(
                "Decay",
                0.1,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1000.0,
                    factor: ATTACK_DECAY_SKEW_FACTOR,
                },
            )
            .with_unit(" ms"),
            midi_channel: EnumParam::new("MIDI channel", MIDIChannelParam::All),
            loop_mode: EnumParam::new("Loop mode", LoopModeParam::Loop),
            loop_length_unit: EnumParam::new("Loop length unit", TimeOrRatioUnit::Ratio),
            recording_mode: EnumParam::new("Recording mode", RecordingMode::default()),
            loop_length_percent: FloatParam::new(
                "Loop length (%)",
                100.0,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 100.0,
                    factor: LOOP_LENGTH_SKEW_FACTOR,
                },
            )
            .with_unit("%"),
            loop_length_time: FloatParam::new(
                "Loop length (seconds)",
                1.0,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 100.0,
                    factor: LOOP_LENGTH_SKEW_FACTOR,
                },
            )
            .with_unit("s"),
            loop_length_sync: FloatParam::new(
                "Loop length (16th notes)",
                4.0,
                FloatRange::Skewed {
                    min: 0.125,
                    max: 16.0,
                    factor: LOOP_LENGTH_SKEW_SYNC,
                },
            )
            .with_unit(" 1/16 notes"),
            start_offset: FloatParam::new(
                "Start offset",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %"),
            volume: FloatParam::new("Gain", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            clip_version: EnumParam::new("Clip version", ClipVersion::V1),
        }
    }
}

impl Default for AudioSampler {
    fn default() -> Self {
        let (debug_data_in, debug_data_out) = triple_buffer::TripleBuffer::default().split();
        Self {
            audio_io_layout: AudioIOLayout::default(),
            params: Arc::new(AudioSamplerParams::default()),
            sample_rate: -1.0,
            peak_meter_decay_weight: 1.0,
            sampler: Sampler::new(0, &InitParams::default()),
            peak_meter: Default::default(), //debug: Arc::new(Mutex::new(None)),
            debug_data_in: Arc::new(parking_lot::Mutex::new(debug_data_in)),
            debug_data_out: Arc::new(parking_lot::Mutex::new(debug_data_out)),
            waveform_summary: Arc::new(VersionedWaveformSummary::default()),
            last_frame_recorded: 0,
            last_waveform_updated: 0,
            active_notes: [[0; 256]; 16],
            reversing: false,
        }
    }
}

impl AudioSampler {
    fn channel_count(&self) -> usize {
        let channel_count: usize = self
            .audio_io_layout
            .main_output_channels
            .unwrap()
            .get()
            .try_into()
            .unwrap();
        channel_count
    }

    #[cfg(debug_assertions)]
    fn verify_active_notes(&mut self) {
        let mut ghost_notes: Vec<_> = self
            .sampler
            .iter_active_notes()
            .filter(|note| !self.is_note_active(note))
            .collect();
        if !ghost_notes.is_empty() {
            let data_lengths: Vec<_> = self
                .sampler
                .channels
                .iter()
                .map(|ch| ch.data.len())
                .collect::<Vec<_>>();
            self.sampler
                .channels
                .iter_mut()
                .for_each(|ch| ch.data.clear());
            eprintln!(
                "sampler just before death: {:#?}\ndatas have been clear, had lengths: {:?}",
                self.sampler, data_lengths
            );
            let count = self.sampler.channels[0].voices.len();
            for (i, v) in self.sampler.channels[0].voices.iter().enumerate() {
                eprintln!("voice[{} of {}]: {:?}", i, count, v);
            }
            panic!("Ghost notes: {:?}", ghost_notes);
        }
    }

    fn is_note_active(&self, note: &Note) -> bool {
        self.active_notes[note.channel as usize][note.note as usize] > 0
    }

    fn set_note_active(&mut self, note: &Note, active: bool) {
        let current = &mut self.active_notes[note.channel as usize][note.note as usize];
        if active {
            *current += 1;
        } else if *current > 0 {
            *current -= 1;
        } else {
            #[cfg(debug_assertions)]
            panic!("note off without note on: {:?}", note)
        }
    }

    fn get_active_notes(&mut self) -> Vec<Note> {
        let mut notes = vec![];
        for channel in 0..16 {
            for note in 0..256 {
                if self.active_notes[channel][note] > 0 {
                    let n = Note::new(note as u8, channel as u8);
                    notes.push(n);
                }
            }
        }
        notes
    }

    fn loop_length(&self) -> TimeOrRatio {
        let unit = self.params.loop_length_unit.value();
        match unit {
            TimeOrRatioUnit::Ratio => {
                TimeOrRatio::Ratio(self.params.loop_length_percent.value() / 100.0)
            }
            TimeOrRatioUnit::Seconds => {
                TimeOrRatio::Time(TimeValue::Seconds(self.params.loop_length_time.value()))
            }
            TimeOrRatioUnit::SixteenthNotes => TimeOrRatio::Time(TimeValue::QuarterNotes(
                self.params.loop_length_sync.value() / 4.0,
            )),
        }
    }

    fn sampler_params(&self, sample_id: usize, transport: &Transport) -> SamplerParams {
        let params_speed = self.params.speed.smoothed.next();
        let params_passthru = self.params.auto_passthru.value();
        let attack_millis = self.params.attack.smoothed.next();
        let attack_samples = (attack_millis * self.sample_rate / 1000.0) as usize;
        let decay_millis = self.params.decay.smoothed.next();
        let decay_samples = (decay_millis * self.sample_rate / 1000.0) as usize;

        let transport = common_types::Transport {
            sample_rate: self.sample_rate,
            tempo: transport.tempo.unwrap() as f32,
            pos_samples: transport.pos_samples().unwrap() as f32,
            time_sig_numerator: transport.time_sig_numerator.unwrap() as u32,
            time_sig_denominator: transport.time_sig_denominator.unwrap() as u32,
        };
        let params = SamplerParams {
            auto_passthru: params_passthru,
            attack_samples,
            loop_mode: LoopMode::from_param(self.params.loop_mode.value()),
            loop_length: self.loop_length(),
            start_offset_percent: self.params.start_offset.value(),
            decay_samples,
            speed: params_speed,
            recording_mode: self.params.recording_mode.value(),
            fixed_size_samples: TimeValue::bars(1.0)
                .as_samples(&transport)
                .to_usize()
                .expect("failed converting value for fixed_size_samples from f32 to usize"),
            transport,
            sample_id,
            reverse_speed: if self.reversing { -1.0 } else { 1.0 },
            clip_version: self.params.clip_version.value(),
        };
        params
    }

    fn update_peak_meter(&mut self, frame: &mut [&mut f32]) {
        let amplitude = (frame.iter().fold(0.0, |z, x| z + **x) / frame.len() as f32).abs();
        let current_peak_meter = self.peak_meter.load(std::sync::atomic::Ordering::Relaxed);
        let new_peak_meter = if amplitude > current_peak_meter {
            amplitude
        } else {
            current_peak_meter * self.peak_meter_decay_weight
                + amplitude * (1.0 - self.peak_meter_decay_weight)
        };

        self.peak_meter
            .store(new_peak_meter, std::sync::atomic::Ordering::Relaxed)
    }

    fn update_waveform(&mut self) {
        self.waveform_summary = Arc::new(VersionedWaveformSummary {
            version: self.waveform_summary.version + 1,
            waveform_summary: self.sampler.get_waveform_summary(940),
        });
    }
}

impl ClapPlugin for AudioSampler {
    const CLAP_ID: &'static str = "com.audiosampler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Audio Sampler");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Instrument,
        ClapFeature::Glitch,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for AudioSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"AudioSamplerPlug";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(AudioSampler);
nih_export_vst3!(AudioSampler);
