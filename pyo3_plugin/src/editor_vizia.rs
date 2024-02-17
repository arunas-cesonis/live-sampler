use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nih_plug::prelude::Editor;
use nih_plug_vizia::assets::register_noto_sans_bold;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

use crate::common_types::Status;
use crate::PyO3PluginParams;

#[derive(Clone, Lens)]
pub struct Data {
    pub(crate) version: Arc<AtomicUsize>,
    pub(crate) params: Arc<PyO3PluginParams>,
    pub(crate) status: Status,
    pub(crate) status_out: Arc<parking_lot::Mutex<triple_buffer::Output<Status>>>,
}

impl Model for Data {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|editor_event, _| match editor_event {
            EditorEvent::Reload => {
                self.version.fetch_add(1, Ordering::Relaxed);
            }
            EditorEvent::UpdatePath(s) => {
                *self.params.source_path.0.lock() = s.to_string();
                self.version.fetch_add(1, Ordering::Relaxed);
            }
        });
    }
}
const WINDOW_SIZE: (u32, u32) = (640, 640);

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| WINDOW_SIZE)
}

pub enum EditorEvent {
    UpdatePath(String),
    Reload,
}

pub struct StatusView {}

impl StatusView {
    pub fn new(cx: &mut Context) -> Handle<Self> {
        Self {}.build(cx, |_| {})
    }
}

impl View for StatusView {
    fn element(&self) -> Option<&'static str> {
        Some("status_view")
    }
}

pub(crate) fn create(editor_state: Arc<ViziaState>, data: Data) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        register_noto_sans_bold(cx);

        data.clone().build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "PyO3")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Bold)
                .font_size(30.0)
                .text_align(TextAlign::Left)
                .height(Pixels(42.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            VStack::new(cx, |cx| {
                Label::new(cx, "File").top(Pixels(10.0));
                Textbox::new(cx, Data::params.map(|x| x.source_path.0.lock().to_string()))
                    .width(Stretch(1.0))
                    .right(Pixels(10.0))
                    .on_edit(|ctx, s| ctx.emit(EditorEvent::UpdatePath(s.to_string())));
                Label::new(cx, "File status").top(Pixels(10.0));
                Label::new(
                    cx,
                    Data::status_out.map(|x| {
                        let files_status = x.lock().read().file_status.clone();
                        format!("{:?}", files_status)
                    }),
                )
                .background_color(Color::white())
                .border_color(Color::lightgrey())
                .border_width(Pixels(1.0))
                .height(Pixels(30.0))
                .width(Stretch(1.0))
                .top(Pixels(10.0));

                Label::new(cx, "Eval status").top(Pixels(10.0));
                Label::new(
                    cx,
                    Data::status_out.map(|x| {
                        let eval_status = x.lock().read().eval_status.clone();
                        format!("{:?}", eval_status)
                    }),
                )
                .background_color(Color::white())
                .border_color(Color::lightgrey())
                .border_width(Pixels(1.0))
                .height(Pixels(30.0))
                .width(Stretch(1.0))
                .top(Pixels(10.0));
                Button::new(
                    cx,
                    |ctx| ctx.emit(EditorEvent::Reload),
                    |cx| Label::new(cx, "Reload"),
                )
                .top(Pixels(10.0));
                Label::new(
                    cx,
                    Data::status_out.map(|x| {
                        let stats = x.lock().read().stats.clone();
                        let total = stats.duration.as_secs_f64();
                        let last = stats.last_duration.as_secs_f64();
                        let last_sec = stats.last_rolling_avg.as_secs_f64();
                        let avg = stats.duration.as_secs_f64() / stats.iterations as f64;
                        vec![
                            format!("avg_last_10sec {:.3}ms", last_sec * 1000.0),
                            format!("avg: {:.3}ms", avg * 1000.0),
                            format!("last: {:.3}ms", last * 1000.0),
                            format!("total: {:.3}ms", total * 1000.0),
                            format!("iter: {}", stats.iterations),
                        ]
                        .join("\n")
                    }),
                )
                .top(Pixels(10.0));
                //Label::new(cx, Data).top(Pixels(10.0));
                StatusView::new(cx);
            });
        })
        .border_width(Pixels(10.0));

        ResizeHandle::new(cx);
    })
}
