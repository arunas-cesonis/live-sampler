use atomic_float::AtomicF32;
use std::cell::Cell;

use nih_plug::prelude::Editor;

use nih_plug_vizia::vizia::prelude::*;

use nih_plug_vizia::vizia::vg::imgref::{Img, ImgRef};
use nih_plug_vizia::vizia::vg::rgb::RGBA8;
use nih_plug_vizia::vizia::vg::{Color, ImageId};
use nih_plug_vizia::vizia::vg::{ImageFlags, ImageSource, Paint, Path, PixelFormat, RenderTarget};
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, vizia, ViziaState, ViziaTheming};

use log::warn;
use nih_plug::nih_warn;
use nih_plug::wrapper::vst3::vst3_sys::vst::get_red;
use nih_plug_vizia::assets::register_noto_sans_bold;
use nih_plug_vizia::vizia::vg;
use std::sync::Arc;

use crate::common_types::Info;
use crate::AudioSamplerParams;

#[derive(Debug, Clone, Default)]
pub struct DebugData {
    pub(crate) info: Info,
}

#[derive(Clone, Lens)]
pub struct Data {
    pub(crate) params: Arc<AudioSamplerParams>,
    pub(crate) peak_meter: Arc<AtomicF32>,
    pub(crate) debug_data_out: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    pub(crate) xy: (f32, f32),
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|editor_event, _| match editor_event {
            EditorEvent::UpdateX(x) => {
                self.xy = (*x, self.xy.1);
                self.x = *x;
            }
            EditorEvent::UpdateY(y) => {
                self.xy = (self.xy.0, *y);
                self.y = *y;
            }
        });
    }
}

const WINDOW_SIZE: (u32, u32) = (640 + 320, 400);
const WINDOW_SIZEF: (f32, f32) = (WINDOW_SIZE.0 as f32, WINDOW_SIZE.1 as f32);

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| WINDOW_SIZE)
}

struct WaveformView<X> {
    debug_data: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    x_lens: X,
    image: Cell<Option<(usize, ImageId)>>,
}

const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn display_notes(cx: &mut Context) {
    let c = vizia::style::Color::red();
    let g = vizia::style::Color::green();
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
    //    .background_color(g)
    .child_left(Stretch(1.0))
    .child_right(Stretch(1.0))
    .child_top(Pixels(-20.0))
    //.child_space(Stretch(1.0))
    .width(Percentage(100.0))
    .height(Percentage(100.0));
}

impl<X> WaveformView<X>
where
    X: Lens<Target = (f32, f32)>,
{
    pub fn new<LDebugData>(cx: &mut Context, debug_data_lens: LDebugData, x: X) -> Handle<Self>
    where
        LDebugData: Lens<Target = Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>>,
    {
        Self {
            debug_data: debug_data_lens.get(cx),
            x_lens: x,
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
        let data = vec![RGBA8::new(180u8, 180u8, 200u8, 255u8); 4 * width * height];
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
                    //if value >= 0.0 {
                    (h, y)
                    //    let h = 0.5 * value.abs() * height as f32;
                    //    let y = 0.5 * height as f32;
                    //    (h, y)
                    //} else {
                    //    let y = 0.5 * height as f32 + value * height as f32 * 0.5;
                    //    let h = 0.5 * value.abs() * height as f32;
                    //    (h, y)
                    //}
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
                //&Paint::color(vg::Color::rgba(255, 0, 0, 128)),
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

impl<X> View for WaveformView<X>
where
    X: 'static + Lens<Target = (f32, f32)>,
{
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
        //{//let background_color = cx.background_color();
        //{let color = Color::rgb(200, 200, 200);
        //{let paint = Paint::color(color.into());
        //{canvas.fill_path(&background_path, &paint);

        // loop
        let color = Color::rgb(26, 165, 89);
        let loop_paint = Paint::color(color.into());
        let color = Color::rgb(26, 165, 89);
        let pos_paint = Paint::color(color.into());
        let color = Color::rgba(255, 165, 89, 128);
        let slice_paint = Paint::color(color.into());

        canvas.fill_text(0.0, 0.0, "HELLO", &Paint::color(Color::rgb(0, 255, 0)));

        for i in 0..16 {
            let width = 5.0;
            let x = i as f32 * (bounds.w / 16.0) + bounds.x;
            let path = rectangle_path(x, bounds.y, width, bounds.h);
            canvas.fill_path(&path, &slice_paint);
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
}

pub(crate) fn create(editor_state: Arc<ViziaState>, data: Data) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        register_noto_sans_bold(cx);
        // assets::register_noto_sans_light(cx);
        //        assets::register_noto_sans_thin(cx);

        data.clone().build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Audio Sampler")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Bold)
                .font_size(30.0)
                .text_align(TextAlign::Left)
                .height(Pixels(42.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    Label::new(cx, "Volume").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.volume);
                    Label::new(cx, "Attack").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.attack);
                    Label::new(cx, "Decay").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.decay);
                    Label::new(cx, "Passthru").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.auto_passthru)
                        .top(Pixels(10.0));
                })
                .width(Percentage(25.0));
                VStack::new(cx, |cx| {
                    Label::new(cx, "Speed").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.speed);
                    Label::new(cx, "Start offset").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.start_offset);
                    Label::new(cx, "Loop length").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.loop_length);
                    Label::new(cx, "Loop mode").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.loop_mode);
                })
                .width(Percentage(25.0));
                VStack::new(cx, |cx| {
                    //    Slider::new(cx, Data::y).on_changing(|cx, value| {
                    //        cx.emit(EditorEvent::UpdateY(value));
                    //    });
                    //    Slider::new(cx, Data::x).on_changing(|cx, value| {
                    //        cx.emit(EditorEvent::UpdateX(value));
                    //    });
                    //    .space(Stretch(1.0));
                    //Label::new(cx, Data::x.map(|x| x.to_string())).space(Stretch(1.0));
                    //Label::new(cx, Data::y.map(|y| y.to_string())).space(Stretch(1.0));
                    //Button::new(
                    //    cx,
                    //    |cx| cx.emit(EditorEvent::UpdateX(10.0)),
                    //    |cx| Label::new(cx, "Update X"),
                    //)
                    //.top(Pixels(10.0));
                    //Label::new(cx, "Debug").top(Pixels(10.0));
                    //Textbox::new_multiline(
                    //    cx,
                    //    Data::debug_data_out.map(|x| {
                    //        let mut m = x.lock();
                    //        format!("{:#?}", m.read())
                    //    }),
                    //    true,
                    //)
                    //.font_size(16.0)
                    //.width(Percentage(100.0))
                    //.height(Percentage(100.0));
                });

                //Element::new(cx).background_color(Color::rgb(255, 0, 0));
                // Element::new(cx);
            });
            WaveformView::new(cx, Data::debug_data_out, Data::xy).height(Pixels(50.0));
        })
        .border_width(Pixels(10.0));

        //VStack::new(cx, |cx| {
        //    Label::new(cx, "Audio Sampler")
        //        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
        //        .font_weight(FontWeightKeyword::Bold)
        //        .font_size(30.0)
        //        .height(Pixels(50.0))
        //        .child_top(Stretch(1.0))
        //        .child_bottom(Pixels(0.0));

        //    Label::new(cx, "Gain");
        //    ParamSlider::new(cx, Data::params, |params| &params.volume);

        //    PeakMeter::new(
        //        cx,
        //        Data::peak_meter
        //            .map(|peak_meter| util::gain_to_db(peak_meter.load(Ordering::Relaxed))),
        //        Some(Duration::from_millis(600)),
        //    )
        //    // This is how adding padding works in vizia
        //    .top(Pixels(10.0));

        //    WaveformView::new(cx, StaticLens::new(&1.0));
        //})
        //.row_between(Pixels(0.0))
        //.child_left(Stretch(1.0))
        //.child_right(Stretch(1.0));

        ResizeHandle::new(cx);
    })
}
