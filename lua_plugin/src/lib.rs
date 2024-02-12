use mlua::{Error, FromLua, LuaSerdeExt, Result, UserData, UserDataFields, Value};
use mlua::{Lua, UserDataMethods};
use std::arch::aarch64::vld1_u8;
use std::fmt;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::UNIX_EPOCH;

use nih_plug::prelude::*;

type SysEx = ();

pub struct LuaPlugin {
    lua: mlua::Lua,
    next_id: usize,
    params: Arc<LuaPluginParams>,
    buffer_config: BufferConfig,
    audio_layout: AudioIOLayout,
}
#[derive(Clone, Debug)]
struct Block {
    id: usize,
    events: Vec<NoteEvent<()>>,
    audio: Vec<Vec<f32>>,
}
#[derive(Params)]
struct LuaPluginParams {
    #[id = "param0"]
    param0: FloatParam,
    #[id = "param1"]
    param1: FloatParam,
    #[id = "param2"]
    param2: FloatParam,
    #[id = "param3"]
    param3: FloatParam,
    #[id = "param4"]
    param4: FloatParam,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct ParamValues {
    param0: f32,
    param1: f32,
    param2: f32,
    param3: f32,
    param4: f32,
}

impl UserData for ParamValues {}

impl From<&LuaPluginParams> for ParamValues {
    fn from(value: &LuaPluginParams) -> Self {
        ParamValues {
            param0: value.param0.value(),
            param1: value.param1.value(),
            param2: value.param2.value(),
            param3: value.param3.value(),
            param4: value.param4.value(),
        }
    }
}

impl Default for LuaPluginParams {
    fn default() -> Self {
        Self {
            param0: FloatParam::new(
                "Parameter 0",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            param1: FloatParam::new(
                "Parameter 1",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            param2: FloatParam::new(
                "Parameter 2",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            param3: FloatParam::new(
                "Parameter 3",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            param4: FloatParam::new(
                "Parameter 4",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
        }
    }
}

impl Default for LuaPlugin {
    fn default() -> Self {
        nih_warn!("DEFAULT");
        Self {
            lua: mlua::Lua::new(),
            params: Arc::new(LuaPluginParams::default()),
            audio_layout: AudioIOLayout::const_default(),
            next_id: 0,
            buffer_config: BufferConfig {
                sample_rate: -1.0,
                min_buffer_size: None,
                max_buffer_size: 0,
                process_mode: ProcessMode::Offline,
            },
        }
    }
}

#[derive(Default)]
struct LuaEvents {
    events: Vec<NoteEvent<()>>,
}

impl UserData for LuaEvents {}

#[derive(Default)]
struct LuaBuffer {
    data: Vec<Vec<f32>>,
}

impl UserData for LuaBuffer {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("get", |_, this, (channel, index): (usize, usize)| {
            Ok(this.data[channel][index])
        });
        methods.add_method_mut(
            "set",
            |_, this, (channel, index, value): (usize, usize, f32)| {
                this.data[channel][index] = value;
                Ok(())
            },
        );
    }
}

impl<'lua> FromLua<'lua> for LuaBuffer {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> Result<Self> {
        let tmp = value.as_userdata().unwrap();
        let lb = tmp.take::<LuaBuffer>()?;
        Ok(lb)
    }
}

impl LuaPlugin {
    fn process_using_userdata(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let lua_nih_warn = self
            .lua
            .create_function(move |_, text: String| {
                nih_warn!("{}", text);
                Ok(())
            })
            .unwrap();
        let mut lua_buffer = LuaBuffer::default();
        lua_buffer.data = buffer
            .as_slice_immutable()
            .into_iter()
            .map(|v| v.to_vec())
            .collect::<Vec<_>>();
        self.lua.globals().set("buffer", lua_buffer).unwrap();
        let events = std::iter::repeat_with(|| context.next_event())
            .take_while(Option::is_some)
            .filter_map(|x| x)
            .collect::<Vec<_>>();
        let lua_events = LuaEvents { events };
        //self.lua.globals().set("events", lua_buffer).unwrap();
        self.lua.globals().set("events", lua_events).unwrap();
        self.lua.globals().set("nih_warn", lua_nih_warn).unwrap();
        self.lua.globals().set("samples", buffer.samples()).unwrap();
        self.lua
            .globals()
            .set("channels", buffer.channels())
            .unwrap();
        let params = self
            .lua
            .to_value(&ParamValues::from(&*self.params))
            .unwrap();
        self.lua.globals().set("params", params).unwrap();
        self.lua
            .load(
                r#"
            local attenuate = params.param0
            for i = 0, samples - 1 do
                for j = 0, channels - 1 do
                    buffer:set(j, i, buffer:get(j, i) * attenuate)
                end
            end
        "#,
            )
            .exec()
            .unwrap();

        let lua_buffer: LuaBuffer = self.lua.globals().get("buffer").unwrap();
        let dst = buffer.as_slice();
        for (channel, samples) in lua_buffer.data.iter().enumerate() {
            for (sample_id, sample) in samples.iter().enumerate() {
                dst[channel][sample_id] = *sample;
            }
        }
        ProcessStatus::Normal
    }
    fn process_using_callbacks(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let data = buffer
            .as_slice()
            .into_iter()
            .map(|v| v.to_vec())
            .collect::<Vec<_>>();
        let data = Arc::new(parking_lot::Mutex::new(data));

        let data1 = data.clone();
        let rust_print = self
            .lua
            .create_function(move |_, text: String| {
                nih_warn!("{}", text);
                Ok(())
            })
            .unwrap();
        let get_sample = self
            .lua
            .create_function(move |_, (channel, index): (usize, usize)| {
                Ok(data1.lock()[channel][index])
            })
            .unwrap();
        let mut data2 = data.clone();
        let mut data3 = data.clone();
        let set_sample = self
            .lua
            .create_function_mut(move |_, (channel, index, value): (usize, usize, f32)| {
                let mut data = data2.lock();
                data[channel][index] = value;
                Ok(())
            })
            .unwrap();
        self.lua.globals().set("rust_print", rust_print).unwrap();
        self.lua.globals().set("get_sample", get_sample).unwrap();
        self.lua.globals().set("set_sample", set_sample).unwrap();
        self.lua.globals().set("samples", buffer.samples()).unwrap();
        self.lua
            .globals()
            .set("channels", buffer.channels())
            .unwrap();
        let params = self
            .lua
            .to_value(&ParamValues::from(&*self.params))
            .unwrap();
        self.lua.globals().set("params", params).unwrap();
        self.lua
            .load(
                r#"
            local attenuate = params.param0
            for i = 0, samples - 1 do
                for j = 0, channels - 1 do
                    set_sample(j, i, get_sample(j, i) * attenuate)
                end
            end
        "#,
            )
            .exec()
            .unwrap();
        let dst = buffer.as_slice();
        let data = data3.lock();
        for (channel, samples) in data.iter().enumerate() {
            for (sample_id, sample) in samples.iter().enumerate() {
                dst[channel][sample_id] = *sample;
            }
        }
        ProcessStatus::Normal
    }
}

impl Plugin for LuaPlugin {
    const NAME: &'static str = "Lua Plugin";
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
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
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
        nih_warn!("INITIALIZE");
        self.buffer_config = buffer_config.clone();
        self.audio_layout = audio_io_layout.clone();
        self.lua = mlua::Lua::new();

        true
    }

    fn reset(&mut self) {
        self.lua = mlua::Lua::new();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.process_using_userdata(buffer, _aux, context)
    }
}

impl ClapPlugin for LuaPlugin {
    const CLAP_ID: &'static str = "com.luaplugin";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Lua Plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for LuaPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"LuaPlugin.......";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(LuaPlugin);
nih_export_vst3!(LuaPlugin);
