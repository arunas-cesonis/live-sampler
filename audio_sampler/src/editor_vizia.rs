use std::cell::Cell;
use std::f64::consts::PI;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use atomic_float::AtomicF32;
use nih_plug::nih_warn;
use nih_plug::params::Param;
use nih_plug::prelude::{Editor, Enum};
use nih_plug_vizia::assets::register_noto_sans_bold;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::vizia::vg::imgref::Img;
use nih_plug_vizia::vizia::vg::rgb::RGBA8;
use nih_plug_vizia::vizia::vg::{Color, ImageId};
use nih_plug_vizia::vizia::vg::{ImageFlags, ImageSource, Paint, Path, PixelFormat, RenderTarget};
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

use crate::common_types::TimeOrRatioUnitParam;
use crate::common_types::{Info, NoteOffBehaviourParam};
use crate::AudioSamplerParams;

#[derive(Debug, Clone, Default)]
pub struct DebugData {
    pub(crate) info: Info,
}

#[derive(Clone, Lens)]
pub struct Data {
    pub(crate) params: Arc<AudioSamplerParams>,
    pub(crate) debug_data_out: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    pub(crate) peak_meter: Arc<AtomicF32>,
}

impl Model for Data {
    //     fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
    //         event.map(|editor_event, _| match editor_event {
    //         });
    //     }
}

#[cfg(debug_assertions)]
const WINDOW_SIZE: (u32, u32) = (640 + 600, 380);

#[cfg(not(debug_assertions))]
const WINDOW_SIZE: (u32, u32) = (640 + 320, 380);

const WINDOW_SIZEF: (f32, f32) = (WINDOW_SIZE.0 as f32, WINDOW_SIZE.1 as f32);

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| WINDOW_SIZE)
}

struct WaveformView {
    debug_data: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    image: Cell<Option<(usize, ImageId)>>,
}

const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn display_notes(cx: &mut Context) {
    HStack::new(cx, |cx| {
        for i in 0..16 {
            let ocatve = (i as i32 / 12) - 2;
            let note = NOTES[i % 12].to_string() + ocatve.to_string().as_str();

            Label::new(cx, &note)
                //.background_color(c)
                .text_align(TextAlign::Center)
                .width(Percentage(100.0 / 16.0));
        }
    })
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0))
        .child_top(Pixels(-20.0))
        .width(Percentage(100.0))
        .height(Percentage(100.0));
}

impl WaveformView {
    pub fn new<LDebugData>(cx: &mut Context, debug_data_lens: LDebugData) -> Handle<Self>
    where
        LDebugData: Lens<Target=Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>>,
    {
        Self {
            debug_data: debug_data_lens.get(cx),
            image: Cell::new(None),
        }
            .build(cx, |cx| {
                display_notes(cx);
            })
    }

    fn get_image(&self, canvas: &mut Canvas, info: &Info) -> ImageId {
        if let Some((version, image_id)) = self.image.get() {
            if version == info.waveform_summary.version {
                return image_id;
            } else {
                self.image.set(None);
                canvas.delete_image(image_id);
            }
            nih_warn!(
                "REDRAW version: {:?} -> {:?}",
                version,
                info.waveform_summary.version
            );
        } else {
            nih_warn!("DRAW version: {:?}", info.waveform_summary.version);
        }
        let grid_size: usize = 16;
        let width = 940;
        let height = 50;
        let image_id = canvas
            .create_image_empty(width, height, PixelFormat::Rgba8, ImageFlags::empty())
            .unwrap();
        let c = 0xd0;
        let data = vec![RGBA8::new(c, c, c, 0xff); 4 * width * height];
        let image = Img::new(data.as_slice(), width, height);
        canvas.update_image(image_id, ImageSource::Rgba(image), 0, 0);
        canvas.save();
        canvas.reset();
        let summary = &info.waveform_summary.waveform_summary;
        if let Ok(size) = canvas.image_size(image_id) {
            canvas.set_render_target(RenderTarget::Image(image_id));
            let max_abs = summary.max.abs().max(summary.min.abs());
            let max_abs_sensible = if max_abs < 0.001 { 1.0 } else { max_abs };
            let scale = 1.0 / max_abs_sensible;
            for (i, value) in info
                .waveform_summary
                .waveform_summary
                .data
                .iter()
                .take(size.0)
                .enumerate()
            {
                let (h, y) = {
                    let value = value.abs() * scale;
                    let h = 1.0 * value.abs() * height as f32;
                    let y = 0.0;
                    (h, y)
                };
                canvas.clear_rect(
                    i as u32,
                    y as u32,
                    1,
                    h as u32,
                    vg::Color::rgba(0, 0, 0, 255),
                );
            }
            canvas.flush();
            canvas.set_render_target(RenderTarget::Screen);
        }
        canvas.restore();
        self.image
            .set(Some((info.waveform_summary.version, image_id)));
        image_id
    }

    fn draw_image(
        &self,
        _cx: &mut DrawContext,
        canvas: &mut Canvas,
        bounds: &BoundingBox,
        info: &Info,
    ) {
        let img = self.get_image(canvas, info);
        if let Ok((imgw, imgh)) = canvas.image_size(img) {
            let mut path = Path::new();
            //path.rect(bounds.x, bounds.y, imgw as f32, imgh as f32);
            path.rect(bounds.x, bounds.y, imgw as f32, imgh as f32);
            canvas.fill_path(
                &path,
                &Paint::image(
                    img,
                    bounds.x,
                    bounds.y,
                    imgw as f32,
                    imgh as f32,
                    0f32,
                    1f32,
                ),
            );
            canvas.stroke_path(&path, &Paint::color(vg::Color::rgba(0, 0, 0, 255)));
        }
        //canvas.delete_image(img);
    }
}

fn rectangle_path(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut path = Path::new();
    path.move_to(x, y);
    path.line_to(x, y + h);
    path.line_to(x + w, y + h);
    path.line_to(x + w, y);
    path.line_to(x, y);
    path.close();
    path
}

impl View for WaveformView {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        //self.draw_image(cx, canvas);
        let mut background_path = Path::new();
        let bounds = cx.bounds();
        let border_width = cx.border_width();

        // 940.0 x 50.0
        let bg_bounds = BoundingBox {
            x: bounds.x + border_width / 2.0,
            y: bounds.y + border_width / 2.0,
            w: bounds.w - border_width,
            h: bounds.h - border_width,
        };
        {
            let x = bg_bounds.x;
            let y = bg_bounds.y;
            let w = bg_bounds.w;
            let h = bg_bounds.h;
            background_path.move_to(x, y);
            background_path.line_to(x, y + h);
            background_path.line_to(x + w, y + h);
            background_path.line_to(x + w, y);
            background_path.line_to(x, y);
            background_path.close();
        }

        let debug_data = &mut self.debug_data.lock();
        let info = &debug_data.read().info;
        self.draw_image(cx, canvas, &bg_bounds, &info);

        // loop
        let color = Color::rgb(26, 165, 89);
        let loop_paint = Paint::color(color.into());
        let color = Color::rgb(26, 165, 89);
        let pos_paint = Paint::color(color.into());
        let color = Color::rgba(255, 255, 255, 128);
        let slice_paint = Paint::color(color.into());
        let color = Color::rgba(255, 0, 0, 128);
        let rec_paint = Paint::color(color.into());

        canvas.fill_text(0.0, 0.0, "HELLO", &Paint::color(Color::rgb(0, 255, 0)));

        for i in 1..16 {
            let width = 5.0;
            let x = i as f32 * (bounds.w / 16.0) + bounds.x;
            let path = rectangle_path(x, bounds.y + 2.0, width, bounds.h - 4.0);
            canvas.fill_path(&path, &slice_paint);
        }

        //if let Some(x) = info.last_recorded_index {

        for x in &info.last_recorded_indices {
            let width = 5.0;
            let x = if let Some(x) = x {
                *x as f32
            } else {
                continue;
            };

            let x = (x as f32 / info.data_len as f32) * bounds.w + bounds.x;
            let path = rectangle_path(x, bounds.y, width, bounds.h);
            canvas.fill_path(&path, &rec_paint);
        }

        for v in &info.voices {
            let width = 5.0;

            let x = v.start * bounds.w + bounds.x;
            let path = rectangle_path(x, bounds.y, width, bounds.h);
            canvas.fill_path(&path, &loop_paint);

            let x = v.end * bounds.w + bounds.x;
            let path = rectangle_path(x, bounds.y, width, bounds.h);
            canvas.fill_path(&path, &loop_paint);

            let x = v.pos * bounds.w + bounds.x;
            let path = rectangle_path(x, bounds.y, width, bounds.h);
            canvas.fill_path(&path, &pos_paint);
        }
    }
}

pub enum EditorEvent {
    UpdateX(f32),
    UpdateY(f32),
    Choice(usize),
}

fn loop_length_slider<P, FMap>(cx: &mut Context, unit: TimeOrRatioUnitParam, lens: FMap)
where
    P: Param + 'static,
    FMap: Fn(&Arc<AudioSamplerParams>) -> &P + Copy + 'static,
{
    let title = <TimeOrRatioUnitParam as Enum>::variants()[unit.to_index()];
    VStack::new(cx, |cx| {
        Label::new(cx, title).top(Pixels(10.0));
        ParamSlider::new(cx, Data::params, lens)
            .width(Stretch(0.75))
            .right(Pixels(10.0));
    })
        .width(Stretch(1.0))
        .right(Pixels(10.0))
        .height(Auto)
        .display(Data::params.map(move |params| params.loop_length_unit.value() == unit));
}

fn param_slider<P, FMap>(cx: &mut Context, title: &str, lens: FMap)
where
    P: Param + 'static,
    FMap: Fn(&Arc<AudioSamplerParams>) -> &P + Copy + 'static,
{
    VStack::new(cx, |cx| {
        Label::new(cx, title).top(Pixels(10.0));
        ParamSlider::new(cx, Data::params, lens)
            .width(Stretch(0.75))
            .right(Pixels(10.0));
    })
        .width(Stretch(1.0))
        .right(Pixels(10.0))
        .height(Auto);
}

fn param_slider1<P, FMap>(cx: &mut Context, title: &str, lens: FMap)
where
    P: Param + 'static,
    FMap: Fn(&Arc<AudioSamplerParams>) -> &P + Copy + 'static,
{
    Label::new(cx, title).top(Pixels(10.0));
    ParamSlider::new(cx, Data::params, lens)
        .width(Stretch(1.0))
        .right(Pixels(10.0));
}

// https://github.com/robbert-vdh/nih-plug/blob/92ce73700005255565c6be45412609ea87eb8b41/src/util.rs#L38
pub const MINUS_INFINITY_GAIN: f32 = 1e-5; // 10f32.powf(MINUS_INFINITY_DB / 20)

/// Convert a voltage gain ratio to decibels. Gain ratios that aren't positive will be treated as
///
/// [`MINUS_INFINITY_DB`].
#[inline]
pub fn gain_to_db(gain: f32) -> f32 {
    f32::max(gain, MINUS_INFINITY_GAIN).log10() * 20.0
}

pub(crate) fn create(editor_state: Arc<ViziaState>, data: Data) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        register_noto_sans_bold(cx);

        data.clone().build(cx);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, "Audio Sampler")
                    .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                    .font_weight(FontWeightKeyword::Bold)
                    .font_size(30.0)
                    .text_align(TextAlign::Left)
                    .height(Pixels(42.0))
                    .child_top(Stretch(1.0))
                    .child_bottom(Pixels(0.0));
                PeakMeter::new(
                    cx,
                    Data::peak_meter
                        .map(|peak_meter| gain_to_db(peak_meter.load(Ordering::Relaxed))),
                    Some(Duration::from_millis(600)),
                )
                    .width(Stretch(0.25))
                    .left(Pixels(20.0))
                    .top(Pixels(19.0));
            })
                .height(Pixels(42.0));
            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    param_slider1(cx, "Volume", |params| &params.volume);
                    param_slider1(cx, "Attack", |params| &params.attack);
                    param_slider1(cx, "Decay", |params| &params.decay);
                    param_slider1(cx, "Passthru", |params| &params.auto_passthru);
                })
                    .width(Percentage(25.0));
                VStack::new(cx, |cx| {
                    param_slider1(cx, "Speed", |params| &params.speed);
                    //param_slider1(cx, "Start offset", |params| &params.start_offset);
                    param_slider1(cx, "Loop mode", |params| &params.loop_mode);
                    param_slider1(cx, "Note off behaviour", |params| &params.note_off_behavior);
                })
                    .width(Percentage(25.0));
                VStack::new(cx, |cx| {
                    param_slider1(cx, "MIDI channel", |params| &params.midi_channel);
                })
                    .width(Percentage(25.0));

                VStack::new(cx, |cx| {
                    loop_length_slider(cx, TimeOrRatioUnitParam::Ratio, |params| {
                        &params.loop_length_percent
                    });
                    loop_length_slider(cx, TimeOrRatioUnitParam::Seconds, |params| {
                        &params.loop_length_time
                    });
                    loop_length_slider(cx, TimeOrRatioUnitParam::SixteenthNotes, |params| {
                        &params.loop_length_sync
                    });
                    param_slider(cx, "Loop length unit", |params| &params.loop_length_unit);
                });
            });
            //HStack::new(cx, |cx| {
            //});
            WaveformView::new(cx, Data::debug_data_out).height(Pixels(50.0));
        })
            .border_width(Pixels(10.0));

        ResizeHandle::new(cx);
    })
}
