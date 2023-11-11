use atomic_float::AtomicF32;
use nih_plug::prelude::{util, Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc};
use parking_lot::Mutex;
use std::time::Duration;
use crossbeam_queue::ArrayQueue;
use dasp::signal::interpolate::Converter;
use nih_plug::nih_warn;
use nih_plug_vizia::vizia::vg::{Paint, Path};

use crate::LiveSamplerParams;

#[derive(Lens)]
struct Data {
    pub(crate) params: Arc<LiveSamplerParams>,
    pub(crate) stats: Arc<ArrayQueue<Stats>>,
    //peak_meter: Arc<AtomicF32>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (500, 400))
}


#[derive(Lens,  Clone, Default)]
pub struct Stats {
    pub(crate) write_position: f32,
    pub(crate) read_position: f32,
    pub(crate) length: usize,
    pub(crate) sample_rate: usize,
    pub(crate) now: usize
}
pub struct PositionThing {
}
impl PositionThing {
    pub fn new<LStats>(cx: &mut Context, stats: LStats) -> Handle<Self>
    where
        LStats: Lens<Target = Arc<ArrayQueue<Stats>>>
    {
        let mut h = Self {}
        .build(cx, |cx| {
            Label::new(cx, stats.map(|stats| {
                let stats = stats.pop().unwrap_or_default();
                format!("now={}\nread={}\nwrite={}\n",
                        stats.now,
                        stats.write_position,
                        stats.read_position
                )
            }));

        });
        h
    }
}

impl View for PositionThing {
    /*
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        //let now  = {
        //    self.stats.lock().now
        //};
        //canvas.stroke_text(100.0, 100.0, now.to_string(), &Paint::color(Color::rgba(0, 0, 0, 255).into()));
        canvas.stroke_text(100.0, 100.0, "HELLO WORLD", &Paint::color(Color::rgba(0, 0, 0, 255).into()));

        //let position = self.stats.write_position;
        //let write_position = self.write_position.load(Ordering::Relaxed);
        let mut p = Path::new();
        let bounds = cx.bounds();
        p.rect(100.0,100.0, canvas.width(), 100.0);
        //let height = 30.0;
        canvas.stroke_path(&mut p, &Paint::color(Color::rgba(0, 0, 0, 255).into()));
        //p.rect(bounds.x, bounds.y, bounds.w, height);

        //canvas.fill_path(&mut p, &Paint::color(Color::rgba(255, 255, 255, 255).into()));

        //let mut pointer = Path::new();
        //pointer.move_to(bounds.x + position * bounds.w, bounds.y);
        //pointer.line_to(bounds.x + position * bounds.w, bounds.y + height * 0.5);
        //let paint = Paint::color(Color::rgba(0, 0, 255, 255).into()).with_line_width(4.0);
        //canvas.stroke_path(&mut pointer, &paint);

        //let mut pointer = Path::new();
        //pointer.move_to(bounds.x + write_position * bounds.w, bounds.y + height * 0.5);
        //pointer.line_to(bounds.x + write_position * bounds.w, bounds.y + height);
        //let paint = Paint::color(Color::rgba(255, 0, 0, 255).into()).with_line_width(4.0);
        //canvas.stroke_path(&mut pointer, &paint);
        //.with_color(
        //    Paint::color(Color::rgba(255, 255, 255, 255).into()))
        //);
    }

     */
}

pub(crate) fn create(
    params: Arc<LiveSamplerParams>,
    stats: Arc<ArrayQueue<Stats>>,
    //peak_meter: Arc<AtomicF32>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
            stats: stats.clone()
            //peak_meter: peak_meter.clone(),
        }
        .build(cx);

        ResizeHandle::new(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Gain");
            ParamSlider::new(cx, Data::params, |params| &params.gain);
            Label::new(cx, "Speed");
            ParamSlider::new(cx, Data::params, |params| &params.speed);
            Label::new(cx, "Fade time");
            ParamSlider::new(cx, Data::params, |params| &params.fade_time);
            ParamButton::new(cx, Data::params, |params| &params.passthru);
            PositionThing::new(cx, Data::stats);
        });
        //.row_between(Pixels(10.0))
        //.child_left(Stretch(1.0))
        //.child_right(Stretch(1.0));
    })
}
