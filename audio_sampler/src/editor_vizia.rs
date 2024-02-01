use atomic_float::AtomicF32;
use std::cell::Cell;

use nih_plug::prelude::Editor;

use nih_plug_vizia::vizia::prelude::*;

use nih_plug_vizia::vizia::vg::imgref::Img;
use nih_plug_vizia::vizia::vg::rgb::RGBA8;
use nih_plug_vizia::vizia::vg::{Color, ImageId};
use nih_plug_vizia::vizia::vg::{ImageFlags, ImageSource, Paint, Path, PixelFormat, RenderTarget};
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

use nih_plug_vizia::assets::register_noto_sans_bold;
use nih_plug_vizia::vizia::vg;
use std::sync::Arc;

use crate::sampler::Info;

use crate::AudioSamplerParams;

#[derive(Debug, Clone, Default)]
pub struct DebugData {
    pub(crate) info: Info,
}

#[derive(Clone, Lens)]
pub struct Data {
    pub(crate) params: Arc<AudioSamplerParams>,
    pub(crate) peak_meter: Arc<AtomicF32>,
    pub(crate) debug: Arc<parking_lot::Mutex<String>>,
    pub(crate) debug_data_out: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
}

impl Model for Data {
    //fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
    //    todo!()
    //}
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (640 + 320, 370))
}

struct WaveformView {
    debug_data: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    image: Cell<Option<ImageId>>,
}

impl WaveformView {
    pub fn new<LDebugData>(cx: &mut Context, debug_data_lens: LDebugData) -> Handle<Self>
    where
        LDebugData: Lens<Target = Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>>,
    {
        Self {
            debug_data: debug_data_lens.get(cx),
            image: Cell::new(None),
        }
        .build(cx, |_| {})
    }

    fn get_image(&self, canvas: &mut Canvas) -> ImageId {
        if let Some(image_id) = self.image.get() {
            return image_id;
        }
        let grid_size: usize = 16;
        let image_id = canvas
            .create_image_empty(
                32 * grid_size + 1,
                26 * grid_size + 1,
                PixelFormat::Rgba8,
                ImageFlags::empty(),
            )
            .unwrap();
        canvas.save();
        canvas.reset();
        if let Ok(size) = canvas.image_size(image_id) {
            eprintln!("{:?}", size);
        }
        if let Ok(size) = canvas.image_size(image_id) {
            canvas.set_render_target(RenderTarget::Image(image_id));
            canvas.clear_rect(0, 0, size.0 as u32, size.1 as u32, vg::Color::rgb(0, 0, 0));
            let x_max = (size.0 / grid_size) - 1;
            let y_max = (size.1 / grid_size) - 1;
            for x in 0..(size.0 / grid_size) {
                for y in 0..(size.1 / grid_size) {
                    canvas.clear_rect(
                        (x * grid_size + 1) as u32,
                        (y * grid_size + 1) as u32,
                        (grid_size - 1) as u32,
                        (grid_size - 1) as u32,
                        if x == 0 || y == 0 || x == x_max || y == y_max {
                            vg::Color::rgb(40, 80, 40)
                        } else {
                            match (x % 2, y % 2) {
                                (0, 0) => vg::Color::rgb(125, 125, 125),
                                (1, 0) => vg::Color::rgb(155, 155, 155),
                                (0, 1) => vg::Color::rgb(155, 155, 155),
                                (1, 1) => vg::Color::rgb(105, 105, 155),
                                _ => vg::Color::rgb(255, 0, 255),
                            }
                        },
                    );
                }
            }
        }
        canvas.restore();
        self.image.set(Some(image_id));
        image_id
    }

    fn draw_image(&self, _cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut path = Path::new();
        let width = 100.0;
        let height = 100.0;
        let x = 10.0;
        let y = 10.0;
        path.rect(x - width / 2.0, y - height / 2.0, width, height);
        let img = self.get_image(canvas);
        canvas.fill_path(
            &path,
            //&Paint::color(vg::Color::rgba(255, 0, 0, 128)),
            &Paint::image(img, 0.0, 0.0, 100.0, 100.0, 0f32, 1f32),
        );
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
        let mut path = Path::new();
        let bounds = cx.bounds();
        let border_width = cx.border_width();

        // 940.0 x 50.0
        {
            let x = bounds.x + border_width / 2.0;
            let y = bounds.y + border_width / 2.0;
            let w = bounds.w - border_width;
            let h = bounds.h - border_width;
            path.move_to(x, y);
            path.line_to(x, y + h);
            path.line_to(x + w, y + h);
            path.line_to(x + w, y);
            path.line_to(x, y);
            path.close();
        }
        //let background_color = cx.background_color();
        let color = Color::rgb(200, 200, 200);
        let paint = Paint::color(color.into());
        canvas.fill_path(&path, &paint);

        // loop
        let color = Color::rgb(100, 100, 150);
        let loop_paint = Paint::color(color.into());
        let color = Color::rgb(26, 165, 89);
        let pos_paint = Paint::color(color.into());

        let debug_data = &mut self.debug_data.lock();
        let info = &debug_data.read().info;

        for v in &info.info {
            if v.start < v.end {
                let start = v.start * bounds.w + bounds.x;
                let end = v.end * bounds.w + bounds.x;
                let width = end - start;
                let path = rectangle_path(start, bounds.y, width, bounds.h);
                canvas.fill_path(&path, &loop_paint);
            } else {
                let start = v.start * bounds.w + bounds.x;
                let end = bounds.w + bounds.x;
                let width = end - start;
                let path = rectangle_path(start, bounds.y, width, bounds.h);
                canvas.fill_path(&path, &loop_paint);

                let start = bounds.x;
                let end = v.end * bounds.w + bounds.x;
                let width = end - start;
                let path = rectangle_path(start, bounds.y, width, bounds.h);
                canvas.fill_path(&path, &loop_paint);
            }
            let x = v.pos * bounds.w + bounds.x;
            let width = 5.0;
            let path = rectangle_path(x, bounds.y, width, bounds.h);
            canvas.fill_path(&path, &pos_paint);
        }
        self.draw_image(cx, canvas);
    }
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
                    Label::new(cx, "Loop length").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.loop_length);
                    Label::new(cx, "Loop mode").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.loop_mode);
                })
                .width(Percentage(25.0));
                VStack::new(cx, |cx| {
                    Label::new(cx, "Debug").top(Pixels(10.0));
                    Textbox::new_multiline(
                        cx,
                        Data::debug_data_out.map(|x| {
                            let mut m = x.lock();
                            format!("{:#?}", m.read())
                        }),
                        true,
                    )
                    .font_size(16.0)
                    .width(Percentage(100.0))
                    .height(Percentage(100.0));
                });

                //Element::new(cx).background_color(Color::rgb(255, 0, 0));
                // Element::new(cx);
            });
            WaveformView::new(cx, Data::debug_data_out).height(Pixels(50.0));
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
