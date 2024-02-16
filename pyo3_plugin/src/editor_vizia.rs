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
const WINDOW_SIZE: (u32, u32) = (640 + 600, 380);

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| WINDOW_SIZE)
}

pub enum EditorEvent {
    UpdatePath(String),
    Reload,
}

pub(crate) fn create(editor_state: Arc<ViziaState>, data: Data) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        register_noto_sans_bold(cx);

        data.clone().build(cx);

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
                    .top(Pixels(10.0));
                    Label::new(cx, "Eval status").top(Pixels(10.0));
                    Label::new(
                        cx,
                        Data::status_out.map(|x| {
                            let eval_status = x.lock().read().eval_status.clone();
                            format!("{:?}", eval_status)
                        }),
                    )
                    .top(Pixels(10.0));
                    Button::new(
                        cx,
                        |ctx| ctx.emit(EditorEvent::Reload),
                        |cx| Label::new(cx, "Reload"),
                    )
                    .top(Pixels(10.0));
                });
            });
        })
        .border_width(Pixels(10.0));

        ResizeHandle::new(cx);
    })
}
