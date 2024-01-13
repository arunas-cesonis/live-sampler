use atomic_float::AtomicF32;
use crossbeam_queue::ArrayQueue;
use nih_plug::prelude::{util, Editor};
use nih_plug_vizia::vizia::image::DynamicImage;
use nih_plug_vizia::vizia::image::DynamicImage::ImageRgb8;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::vizia::vg::imgref::{Img, ImgVec};
use nih_plug_vizia::vizia::vg::rgb::{RGB, RGB8, RGBA8};
use nih_plug_vizia::vizia::vg::{ImageFlags, ImageId, ImageSource, Paint, Path, PixelFormat};
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::sampler::Info;
use crate::AudioSamplerParams;

#[derive(Lens)]
struct Data {
    params: Arc<AudioSamplerParams>,
    peak_meter: Arc<AtomicF32>,
    info: Arc<ArrayQueue<Info>>,
}

impl Model for Data {}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (640, 320))
}

struct WaveformView<T>
where
    T: Lens<Target = f32>,
{
    image: T,
}

impl<T> WaveformView<T>
where
    T: Lens<Target = f32>,
{
    pub fn new(cx: &mut Context, image: T) -> Handle<Self>
    where
        T: Lens<Target = f32>,
    {
        Self { image }.build(cx, |_| {})
    }

    // The below prints this in stdout:
    // UNSUPPORTED (log once): POSSIBLE ISSUE: unit 0 GLD_TEXTURE_INDEX_2D is unloadable and bound to sampler type (Float) - using zero texture because texture unloadable
    // It may be Apple M1 specific, as quick search reveals
    // TODO: check on other platforms, try loading image file, also check how Vizia renders fonts
    fn draw_image(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let w = 50;
        let h = 20;
        let image_id = canvas
            .create_image_empty(w, h, PixelFormat::Rgba8, ImageFlags::empty())
            .unwrap();

        //let data = vec![RGBA8::new(255u8, 0u8, 0u8, 255u8); w * h];
        let data = vec![RGBA8::new(255u8, 0u8, 0u8, 255u8); w * h];
        let img = Img::new(data.as_slice(), w, h);
        let img = ImageSource::from(img);
        canvas.update_image(image_id, img, 0, 0).unwrap();

        let image_paint = Paint::image(image_id, 0.0, 0.0, w as f32, h as f32, 0.0, 1.0);
        //pub fn image(id: ImageId, cx: f32, cy: f32, width: f32, height: f32, angle: f32, alpha: f32) -> Self {
        let rect = BoundingBox::from_min_max(0.0, 0.0, w as f32, h as f32);
        let path = rectangle_path(0.0, 0.0, w as f32, h as f32);
        canvas.fill_path(&path, &image_paint);
        canvas.delete_image(image_id);
    }
}

fn rectangle_path(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut path = vg::Path::new();
    path.move_to(x, y);
    path.line_to(x, y + h);
    path.line_to(x + w, y + h);
    path.line_to(x + w, y);
    path.line_to(x, y);
    path.close();
    path
}

impl<T> View for WaveformView<T>
where
    T: Lens<Target = f32>,
{
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.draw_image(cx, canvas);
        //let mut path = vg::Path::new();
        //let bounds = cx.bounds();
        //let border_width = cx.border_width();
        //{
        //    let x = bounds.x + border_width / 2.0;
        //    let y = bounds.y + border_width / 2.0;
        //    let w = bounds.w - border_width;
        //    let h = bounds.h - border_width;
        //    path.move_to(x, y);
        //    path.line_to(x, y + h);
        //    path.line_to(x + w, y + h);
        //    path.line_to(x + w, y);
        //    path.line_to(x, y);
        //    path.close();
        //}
        //let background_color = cx.background_color();
        //let color = Color::rgb(255, 0, 0);
        //let paint = vg::Paint::color(color.into());
        //canvas.fill_path(&path, &paint);
    }
}

pub(crate) fn create(
    params: Arc<AudioSamplerParams>,
    peak_meter: Arc<AtomicF32>,
    editor_state: Arc<ViziaState>,
    info: Arc<ArrayQueue<Info>>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
            peak_meter: peak_meter.clone(),
            info: info.clone(),
        }
        .build(cx);

        //ImageSource::
        let w = 50;
        let h = 20;
        let data = vec![RGB8::new(255, 0, 0); w * h];
        let img = Img::new(data.as_slice(), w, h);
        let img = ImageSource::from(img);

        //let img = Image::new(cx, img);
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
                });
                VStack::new(cx, |cx| {
                    Label::new(cx, "Speed").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.speed);
                    Label::new(cx, "Loop length").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.loop_length);
                    Label::new(cx, "Loop mode").top(Pixels(10.0));
                    ParamSlider::new(cx, Data::params, |params| &params.loop_mode);
                });
            });
            WaveformView::new(cx, StaticLens::new(&1.0));
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
