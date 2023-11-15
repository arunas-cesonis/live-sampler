use atomic_float::AtomicF32;
use crossbeam_queue::ArrayQueue;
use dasp::signal::interpolate::Converter;
use nih_plug::nih_warn;
use nih_plug::prelude::{util, Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::{FontId, Paint, Path};
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::fmt::Formatter;
use std::ops::DerefMut;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::LiveSamplerParams;

#[derive(Lens)]
struct Data {
    pub(crate) params: Arc<LiveSamplerParams>,
    //peak_meter: Arc<AtomicF32>,
}

impl Model for Data {}

#[derive(Debug, Clone, Default)]
pub struct DebugData {}

impl core::fmt::Display for DebugData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (500, 400))
}

pub struct DebugView {
    output: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
}
impl DebugView {
    pub fn new(output: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>) -> Self {
        Self { output }
    }
}

impl View for DebugView {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut p = Path::new();
        p.rect(0.0, 0.0, 100.0, 100.0);
        let c = Paint::color(Color::rgb(0, 0, 0).into());
        //canvas.fill_path(&mut p, &c);
    }
}

pub(crate) fn create(
    params: Arc<LiveSamplerParams>,
    output: Arc<parking_lot::Mutex<triple_buffer::Output<DebugData>>>,
    //peak_meter: Arc<AtomicF32>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
            //peak_meter: peak_meter.clone(),
        }
        .build(cx);

        ResizeHandle::new(cx);
        DebugView::new(output.clone()).build(cx, |cx| {
            //Label::new(cx, Data::debug)
            //    .width(Pixels(500.0))
            //    .height(Pixels(400.0));
        });

        //VStack::new(cx, |cx| {
        //    //Label::new(cx, "Gain");
        //    //ParamSlider::new(cx, Data::params, |params| &params.gain);
        //    //Label::new(cx, "Speed");
        //    //ParamSlider::new(cx, Data::params, |params| &params.speed);
        //    //Label::new(cx, "Fade time");
        //    //ParamSlider::new(cx, Data::params, |params| &params.fade_time);
        //    //ParamButton::new(cx, Data::params, |params| &params.passthru);
        //});
        //.row_between(Pixels(10.0))
        //.child_left(Stretch(1.0))
        //.child_right(Stretch(1.0));
    })
}
