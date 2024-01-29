#![allow(unused)]
use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

use crate::sampler::{Info, LoopMode, Sampler};

use crate::common_types::{LoopModeParam, Params as SamplerParams};
use crate::editor_vizia::DebugData;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// mod editor;
mod common_types;
mod editor_vizia;
mod sampler;
mod test_sampler;
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
    info_queue: Arc<ArrayQueue<Info>>,
    debug: Arc<parking_lot::Mutex<String>>,
    debug_data_in: Arc<parking_lot::Mutex<triple_buffer::Input<DebugData>>>,
    debug_data_out: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    peak_meter_decay_weight: f32,
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
    #[id = "loop_length"]
    pub loop_length: FloatParam,
    #[id = "volume"]
    pub volume: FloatParam,
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

const MILLISECONDS_PARAM_SKEW_FACTOR: f32 = 0.25;

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
                    factor: MILLISECONDS_PARAM_SKEW_FACTOR,
                },
            )
            .with_unit(" ms"),
            decay: FloatParam::new(
                "Decay",
                0.1,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1000.0,
                    factor: MILLISECONDS_PARAM_SKEW_FACTOR,
                },
            )
            .with_unit(" ms"),
            loop_mode: EnumParam::new("Loop mode", LoopModeParam::PlayOnce),
            loop_length: FloatParam::new(
                "Loop length",
                1.0,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 1.0,
                    factor: 0.5,
                },
            )
            .with_unit(" %"),
            volume: FloatParam::new("Gain", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
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
            sampler: Sampler::new(0, &SamplerParams::default()),
            info_queue: Arc::new(ArrayQueue::new(1)),
            peak_meter: Default::default(), //debug: Arc::new(Mutex::new(None)),
            debug: Default::default(),
            debug_data_in: Arc::new(parking_lot::Mutex::new(debug_data_in)),
            debug_data_out: Arc::new(parking_lot::Mutex::new(debug_data_out)),
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
    //fn debug_println(&mut self, fmt: fmt::Arguments) {
    //    let f = self.debug.lock();
    //    let binding = f.unwrap();
    //    let mut file = binding.as_ref().unwrap();
    //    file.write_fmt(fmt).unwrap();
    //    file.write(&[b'\n']).unwrap();
    //    file.flush().unwrap();
    //}
    fn sampler_params(&self) -> SamplerParams {
        let params_speed = self.params.speed.smoothed.next();
        let params_passthru = self.params.auto_passthru.value();
        let attack_millis = self.params.attack.smoothed.next();
        let attack_samples = (attack_millis * self.sample_rate / 1000.0) as usize;
        let decay_millis = self.params.decay.smoothed.next();
        let decay_samples = (decay_millis * self.sample_rate / 1000.0) as usize;
        let params = SamplerParams {
            auto_passthru: params_passthru,
            attack_samples,
            loop_mode: LoopMode::from_param(self.params.loop_mode.value()),
            loop_length_percent: self.params.loop_length.smoothed.next(),
            decay_samples,
            speed: params_speed,
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

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Using vizia as Iced doesn't support drawing bitmap images under OpenGL
        let info_queue = Arc::new(ArrayQueue::new(1));
        self.info_queue = info_queue.clone();
        self.debug = Default::default();
        self.debug = Arc::new(parking_lot::Mutex::new(format!("{:?}", self.sampler)));

        let data = editor_vizia::Data {
            params: self.params.clone(),
            peak_meter: self.peak_meter.clone(),
            debug: self.debug.clone(),
            debug_data_out: self.debug_data_out.clone(),
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
        self.sampler = Sampler::new(self.channel_count(), &self.sampler_params());
        self.peak_meter_decay_weight = 0.25f64
            .powf((buffer_config.sample_rate as f64 * PEAK_METER_DECAY_MS / 1000.0).recip())
            as f32;
        true
    }

    fn reset(&mut self) {
        self.sampler = Sampler::new(self.channel_count(), &self.sampler_params());
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();

        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            let params = self.sampler_params();
            let params = &params;
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }
                //self.debug_println(format_args!("{:?}", event));
                //nih_warn!("event {:?}", event);
                // assert!(event.voice_id().is_none());
                match event {
                    NoteEvent::NoteOn { velocity, note, .. } => match note {
                        0 => self.sampler.start_recording(params),
                        1 => self.sampler.reverse(params),
                        12..=27 => {
                            let pos = (note - 12) as f32 / 16.0;
                            self.sampler.start_playing(pos, note, velocity, params);
                        }
                        _ => (),
                    },
                    NoteEvent::NoteOff { note, .. } => match note {
                        0 => self.sampler.stop_recording(params),
                        1 => self.sampler.unreverse(params),
                        12..=27 => self.sampler.stop_playing(note, params),
                        _ => (),
                    },
                    _ => (),
                }
                next_event = context.next_event();
            }

            let mut frame = channel_samples.into_iter().collect::<Vec<_>>();
            self.sampler.process_frame(&mut frame, params);

            //for sample in channel_samples {
            //    amplitude += *sample;
            //}
            if self.params.editor_state.is_open() {
                self.update_peak_meter(&mut frame);
                let info = self.sampler.get_info(params);
                self.debug_data_in.lock().write(DebugData { info });
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for AudioSampler {
    const CLAP_ID: &'static str = "com.audiosampler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Audio Sampler");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
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
