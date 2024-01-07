use std::sync::Arc;

use nih_plug::prelude::*;

use crate::sampler::Sampler;

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

mod sampler;
mod volume;

type SysEx = ();

pub struct AudioSampler {
    audio_io_layout: AudioIOLayout,
    params: Arc<AudioSamplerParams>,
    sample_rate: f32,
    sampler: Sampler,
    //    debug: Arc<Mutex<Option<std::fs::File>>>,
}

#[derive(Params)]
struct AudioSamplerParams {
    #[id = "auto_passthru"]
    pub auto_passthru: BoolParam,
    #[id = "speed"]
    pub speed: FloatParam,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
}

impl Default for AudioSamplerParams {
    fn default() -> Self {
        Self {
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
                    factor: 2.0,
                },
            )
            .with_unit(" ms"),
            decay: FloatParam::new(
                "decay",
                0.1,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1000.0,
                    factor: 2.0,
                },
            )
            .with_unit(" ms"),
        }
    }
}

impl Default for AudioSampler {
    fn default() -> Self {
        Self {
            audio_io_layout: AudioIOLayout::default(),
            params: Arc::new(AudioSamplerParams::default()),
            sample_rate: -1.0,
            sampler: Sampler::new(0, &sampler::Params::default()),
            //debug: Arc::new(Mutex::new(None)),
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
    fn sampler_params(&self) -> sampler::Params {
        let params_speed = self.params.speed.smoothed.next();
        let params_passthru = self.params.auto_passthru.value();
        let attack_millis = self.params.attack.smoothed.next();
        let attack_samples = (attack_millis * self.sample_rate / 1000.0) as usize;
        let decay_millis = self.params.decay.smoothed.next();
        let decay_samples = (decay_millis * self.sample_rate / 1000.0) as usize;
        let params = sampler::Params {
            auto_passthru: params_passthru,
            attack_samples,
            decay_samples,
            speed: params_speed,
        };
        params
    }
}

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

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.audio_io_layout = audio_io_layout.clone();
        self.sample_rate = buffer_config.sample_rate;
        self.sampler = Sampler::new(self.channel_count(), &self.sampler_params());
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

            self.sampler.process_sample(channel_samples, params);
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
