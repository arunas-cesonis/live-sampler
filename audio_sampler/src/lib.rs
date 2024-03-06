#![allow(unused)]
#![feature(extend_one)]

#[cfg(feature = "use_jemalloc")]
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(feature = "use_jemalloc")]
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "use_mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "use_mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::sync::Arc;

use crate::common_types::{
    Info, LoopModeParam, MIDIChannelParam, NoteOffBehaviourParam, RecordingModeParam,
    TimeOrRatioUnitParam,
};
use audio_sampler_lib::common_types::Params as SamplerParams;
use audio_sampler_lib::common_types::{InitParams, Note, VersionedWaveformSummary};
use audio_sampler_lib::sampler::Sampler;
use audio_sampler_lib::time_value::{TimeOrRatio, TimeValue};
use nih_plug::prelude::*;
#[cfg(feature = "use_vizia")]
use nih_plug_vizia::vizia::entity;
#[cfg(feature = "use_vizia")]
use nih_plug_vizia::vizia::prelude::Role::Time;
#[cfg(feature = "use_vizia")]
use nih_plug_vizia::ViziaState;
use num_traits::ToPrimitive;

#[cfg(feature = "use_vizia")]
use crate::editor_vizia::DebugData;

// mod editor;
mod common_types;

#[cfg(feature = "use_vizia")]
mod editor_vizia;

type SysEx = ();

pub struct AudioSampler {
    audio_io_layout: AudioIOLayout,
    params: Arc<AudioSamplerParams>,
    sample_rate: f32,
    sampler: Sampler,
    peak_meter: Arc<AtomicF32>,

    #[cfg(feature = "use_vizia")]
    debug_data_in: Arc<parking_lot::Mutex<triple_buffer::Input<DebugData>>>,
    #[cfg(feature = "use_vizia")]
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

    #[cfg(feature = "use_vizia")]
    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Using vizia as Iced doesn't support drawing bitmap images under OpenGL

        let data = editor_vizia::Data {
            params: self.params.clone(),
            debug_data_out: self.debug_data_out.clone(),
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
            let params_midi_channel: Option<u8> = self.params.midi_channel.value().try_into().ok();
            let params = &mut params;
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                #[cfg(debug_assertions)]
                nih_warn!("event: {:?}", event);

                match event {
                    NoteEvent::PolyTuning {
                        note,
                        channel: note_channel,
                        tuning,
                        ..
                    } if params_midi_channel.is_none()
                        || params_midi_channel == Some(note_channel) =>
                    {
                        let note = Note::new(note, note_channel);
                        let speed = 1.0 + (tuning / 12.0);
                        self.sampler.set_note_speed(note, speed);
                    }
                    NoteEvent::NoteOn {
                        velocity,
                        note,
                        channel: note_channel,
                        ..
                    } if params_midi_channel.is_none()
                        || params_midi_channel == Some(note_channel) =>
                    {
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

            if self.sampler.is_recording(0) {
                self.last_frame_recorded = self.sampler.get_frames_processed(0);
            }

            let mut frame = channel_samples.into_iter().collect::<Vec<_>>();
            self.sampler.process_frame(&mut frame, params);

            //for sample in channel_samples {
            //    amplitude += *sample;
            //}
            #[cfg(feature = "use_vizia")]
            if self.params.editor_state.is_open() {
                self.update_peak_meter(&mut frame);

                if self.last_frame_recorded > self.last_waveform_updated + self.sample_rate as usize
                {
                    self.update_waveform();
                    self.last_waveform_updated = self.last_frame_recorded;
                }
                let voice_info = self.sampler.get_voice_info(0, params);
                let info = Info {
                    voices: voice_info,
                    last_recorded_indices: self.sampler.get_last_recorded_offsets(),
                    data_len: self.sampler.get_data_len(0),
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

    #[id = "attack"]
    pub attack: FloatParam,

    #[id = "decay"]
    pub decay: FloatParam,

    #[id = "loop_mode"]
    pub loop_mode: EnumParam<LoopModeParam>,

    #[id = "note_off_behavior"]
    pub note_off_behavior: EnumParam<NoteOffBehaviourParam>,

    #[id = "loop_length_percent"]
    pub loop_length_percent: FloatParam,

    #[id = "loop_length_time"]
    pub loop_length_time: FloatParam,

    #[id = "loop_length_sync"]
    pub loop_length_sync: FloatParam,

    //    #[id = "start_offset"]
    //   pub start_offset: FloatParam,
    #[id = "loop_length_unit"]
    pub loop_length_unit: EnumParam<TimeOrRatioUnitParam>,

    #[id = "volume"]
    pub volume: FloatParam,

    #[id = "recording_mode"]
    pub recording_mode: EnumParam<RecordingModeParam>,

    #[id = "midi_channel"]
    pub midi_channel: EnumParam<MIDIChannelParam>,
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[cfg(feature = "use_vizia")]
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

const ATTACK_DECAY_SKEW_FACTOR: f32 = 0.25;
const LOOP_LENGTH_SKEW_FACTOR: f32 = 0.5;
const LOOP_LENGTH_SKEW_SYNC: f32 = 0.25;

impl Default for AudioSamplerParams {
    fn default() -> Self {
        Self {
            #[cfg(feature = "use_vizia")]
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
            loop_length_unit: EnumParam::new("Loop length unit", TimeOrRatioUnitParam::Ratio),
            recording_mode: EnumParam::new("Recording mode", RecordingModeParam::default()),
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
            //start_offset: FloatParam::new(
            //    "Start offset",
            //    0.0,
            //    FloatRange::Linear { min: 0.0, max: 1.0 },
            //)
            //    .with_unit(" %"),
            volume: FloatParam::new("Gain", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            note_off_behavior: EnumParam::new(
                "Note off behavior",
                NoteOffBehaviourParam::default(),
            ),
        }
    }
}

impl Default for AudioSampler {
    fn default() -> Self {
        #[cfg(feature = "use_vizia")]
        let (debug_data_in, debug_data_out) = triple_buffer::TripleBuffer::default().split();
        Self {
            audio_io_layout: AudioIOLayout::default(),
            params: Arc::new(AudioSamplerParams::default()),
            sample_rate: -1.0,
            peak_meter_decay_weight: 1.0,
            sampler: Sampler::new(0, &InitParams::default()),
            peak_meter: Default::default(), //debug: Arc::new(Mutex::new(None)),
            #[cfg(feature = "use_vizia")]
            debug_data_in: Arc::new(parking_lot::Mutex::new(debug_data_in)),
            #[cfg(feature = "use_vizia")]
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
            .iter_active_notes(0)
            .filter(|note| !self.is_note_active(note))
            .collect();
        if !ghost_notes.is_empty() {
            self.sampler.dump_crash_info();
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
            TimeOrRatioUnitParam::Ratio => {
                TimeOrRatio::Ratio(self.params.loop_length_percent.value() / 100.0)
            }
            TimeOrRatioUnitParam::Seconds => {
                TimeOrRatio::Time(TimeValue::Seconds(self.params.loop_length_time.value()))
            }
            TimeOrRatioUnitParam::SixteenthNotes => TimeOrRatio::Time(TimeValue::QuarterNotes(
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

        let transport = audio_sampler_lib::common_types::Transport {
            sample_rate: self.sample_rate,
            tempo: transport.tempo.unwrap() as f32,
            pos_samples: transport.pos_samples().unwrap() as f32,
            time_sig_numerator: transport.time_sig_numerator.unwrap() as u32,
            time_sig_denominator: transport.time_sig_denominator.unwrap() as u32,
        };
        let params = SamplerParams {
            auto_passthru: params_passthru,
            attack_samples,
            volume: self.params.volume.value(),
            loop_mode: self.params.loop_mode.value().into(),
            loop_length: self.loop_length(),
            // start_offset_percent: self.params.start_offset.value(),
            start_offset_percent: 0.0,
            decay_samples,
            speed: params_speed,
            recording_mode: self.params.recording_mode.value().into(),
            fixed_size_samples: TimeValue::bars(1.0)
                .as_samples(&transport)
                .to_usize()
                .expect("failed converting value for fixed_size_samples from f32 to usize"),
            transport,
            sample_id,
            reverse_speed: if self.reversing { -1.0 } else { 1.0 },
            note_off_behavior: self.params.note_off_behavior.value().into(),
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
        ClapFeature::Glitch,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for AudioSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"AudioSamplerPlug";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Sampler];
}

nih_export_clap!(AudioSampler);
nih_export_vst3!(AudioSampler);
