use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use nih_plug::editor::Editor;
use nih_plug::params::Param;
use nih_plug::prelude::Enum;
use nih_plug_vizia::assets::register_noto_sans_bold;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

use crate::common_types::{RuntimeStats, Status};
use crate::params::{ModeParam, PyO3PluginParams2};

#[derive(Clone, Lens)]
pub struct Data {
    pub(crate) version: Arc<AtomicUsize>,
    pub(crate) params: Arc<PyO3PluginParams2>,
    pub(crate) status: Status,
    pub(crate) status_out: Arc<parking_lot::Mutex<triple_buffer::Output<Status>>>,
    pub(crate) runtime_stats_out:
        Arc<parking_lot::Mutex<triple_buffer::Output<Option<RuntimeStats>>>>,
}

impl Model for Data {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|editor_event, _| match editor_event {
            EditorEvent::Reload => {
                self.version.fetch_add(1, Ordering::Relaxed);
            }
            EditorEvent::UpdatePath(s) => {
                *self.params.source_path().0.lock() = s.to_string();
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

fn select_mode(ctx: &mut EventContext, w: &ParamWidgetBase, mode: ModeParam) {
    let value = Data::params.get(ctx).mode.preview_normalized(mode);
    w.begin_set_parameter(ctx);
    w.set_normalized_value(ctx, value);
    w.end_set_parameter(ctx);
}

fn mode_button<'a>(cx: &'a mut Context, title: &'a str, mode: ModeParam) -> Handle<'a, Button> {
    let b = ParamWidgetBase::new(cx, Data::params, |params| &params.mode);
    Button::new(
        cx,
        move |ctx| {
            select_mode(ctx, &b, mode);
        },
        move |cx| {
            Label::new(cx, title).color(Data::params.map(move |params| {
                if params.mode.value() == mode {
                    Color::black()
                } else {
                    Color::darkgray()
                }
            }))
        },
    )
    .top(Pixels(10.0))
}

pub(crate) fn create2(editor_state: Arc<ViziaState>, data: Data) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        register_noto_sans_bold(cx);

        data.clone().build(cx);

        VStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    Label::new(cx, "PyO3")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Bold)
                        .font_size(30.0)
                        .text_align(TextAlign::Left)
                        .height(Pixels(42.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                })
                .height(Pixels(42.0));
                HStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "File").top(Pixels(10.0));
                        Textbox::new(
                            cx,
                            Data::params.map(|x| x.source_path().0.lock().to_string()),
                        )
                        //Textbox::new(cx, Data::params.map(|x| "source_path".to_string()))
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
                        .child_left(Pixels(8.0))
                        .child_top(Pixels(6.0))
                        .height(Pixels(30.0))
                        .width(Stretch(1.0))
                        .top(Pixels(10.0));
                        Button::new(
                            cx,
                            |ctx| ctx.emit(EditorEvent::Reload),
                            |cx| Label::new(cx, "Reload"),
                        )
                        .top(Pixels(10.0));
                        ParamButton::new(cx, Data::params, |params| &params.watch_source_path)
                            .top(Pixels(10.0));
                    });
                })
                .top(Pixels(10.0));
                HStack::new(cx, |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Eval status");
                        Textbox::new(
                            cx,
                            Data::status_out.map(|x| {
                                let mut m = x.lock();
                                let st = m.read();
                                let eval_status = &st.eval_status;
                                format!("{:?}", eval_status)
                            }),
                        )
                        .background_color(Color::white())
                        .border_color(Color::lightgrey())
                        .border_width(Pixels(1.0))
                        .child_left(Pixels(8.0))
                        .child_top(Pixels(6.0))
                        .height(Pixels(60.0))
                        .width(Stretch(1.0))
                        .read_only(true)
                        .top(Pixels(10.0));
                        HStack::new(cx, |cx| {
                            mode_button(cx, "Run", ModeParam::Run);
                            mode_button(cx, "Pause", ModeParam::Pause);
                            mode_button(cx, "Bypass", ModeParam::Bypass);
                        });
                    });
                })
                .top(Pixels(50.0));
                VStack::new(cx, |cx| {
                    Label::new(
                        cx,
                        Data::runtime_stats_out.map(|x| {
                            let stats = match x.lock().read().clone() {
                                Some(stats) => stats,
                                None => return "".to_string(),
                            };
                            let total = stats.total_duration.as_secs_f64();
                            let last = stats.last_duration.as_secs_f64();
                            let last_sec = stats.last_rolling_avg.as_secs_f64();
                            //let loaded = stats.source_loaded.elapsed().as_secs_f64();
                            let avg = stats.total_duration.as_secs_f64() / stats.iterations as f64;
                            let out = vec![
                                ///format!("loaded: {:.1}s ago", loaded),
                                format!("avg_last_10sec {:.3}ms", last_sec * 1000.0),
                                format!("avg: {:.3}ms", avg * 1000.0),
                                format!("last: {:.3}ms", last * 1000.0),
                                format!("total: {:.3}ms", total * 1000.0),
                                format!("window_size: {}", stats.window_size),
                                format!("events_to_pyo3: {}", stats.events_to_pyo3),
                                format!("events_from_pyo3: {}", stats.events_from_pyo3),
                                format!("sample_rate: {}", stats.sample_rate),
                                format!("iter: {}", stats.iterations),
                            ];
                            out.join("\n")
                        }),
                    )
                    .top(Pixels(10.0));
                    //Label::new(cx, Data).top(Pixels(10.0));
                    Label::new(
                        cx,
                        Data::status_out.map(|x| {
                            if x.lock().read().paused_on_error {
                                "Paused on error. Reload to  resume".to_string()
                            } else {
                                "".to_string()
                            }
                        }),
                    );
                });
            });
        })
        .border_width(Pixels(10.0));

        ResizeHandle::new(cx);
    })
}
