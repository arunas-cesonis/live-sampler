use atomic_float::AtomicF32;
use nih_plug::prelude::{util, Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::LiveSamplerParams;

#[derive(Lens)]
struct Data {
    params: Arc<LiveSamplerParams>,
    //peak_meter: Arc<AtomicF32>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (500, 240))
}

pub(crate) fn create(
    params: Arc<LiveSamplerParams>,
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

        VStack::new(cx, |cx| {
            Label::new(cx, "Gain");
            ParamSlider::new(cx, Data::params, |params| &params.gain);
            Label::new(cx, "Speed");
            ParamSlider::new(cx, Data::params, |params| &params.speed);
            Label::new(cx, "Fade time");
            ParamSlider::new(cx, Data::params, |params| &params.fade_time);
            ParamButton::new(cx, Data::params, |params| &params.passthru);
        })
        .row_between(Pixels(10.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    })
}
