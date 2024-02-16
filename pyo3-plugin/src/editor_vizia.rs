use atomic_float::AtomicF32;
use std::cell::Cell;
use std::f64::consts::PI;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::common_types::Status;
use crate::PyO3PluginParams;
use nih_plug::nih_warn;
use nih_plug::prelude::Editor;
use nih_plug_vizia::assets::register_noto_sans_bold;
use nih_plug_vizia::vizia::prelude::Role::TextField;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use nih_plug_vizia::vizia::vg::imgref::Img;
use nih_plug_vizia::vizia::vg::rgb::RGBA8;
use nih_plug_vizia::vizia::vg::{Color, ImageId};
use nih_plug_vizia::vizia::vg::{ImageFlags, ImageSource, Paint, Path, PixelFormat, RenderTarget};
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

#[derive(Debug, Default)]
pub struct DebugData {
    pub(crate) message: Option<String>,
}

#[derive(Clone, Lens)]
pub struct Data {
    pub(crate) version: Arc<AtomicUsize>,
    pub(crate) params: Arc<PyO3PluginParams>,
    pub(crate) status_out: Arc<parking_lot::Mutex<triple_buffer::Output<Status>>>,
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|editor_event, _| match editor_event {
            EditorEvent::Choice(i) => {
                eprintln!("Choice: {}", i);
                //self.debug_data_out.lock().write().message = format!("Choice: {}", i);
            }
            EditorEvent::UpdatePath(s) => {
                *self.params.source_path.0.lock() = s.to_string();
                self.version.fetch_add(1, Ordering::Relaxed);
                //std::mem::swap(&mut s, &mut *self.params.source_path.0.lock());
                //*self.params.source_path.0.lock() = s;
                //self.debug_data_out.lock().write().message = format!("UpdatePath: {}", s);
            }
            EditorEvent::UpdateX(x) => {}
            EditorEvent::UpdateY(y) => {}
        });
    }
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

struct WaveformView<X> {
    x_lens: X,
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

pub enum EditorEvent {
    UpdateX(f32),
    UpdateY(f32),
    Choice(usize),
    UpdatePath(String),
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
                    nih_plug_vizia::vizia::views::Textbox::new(
                        cx,
                        Data::params.map(|x| x.source_path.0.lock().to_string()),
                    )
                    .width(Stretch(1.0))
                    .right(Pixels(10.0))
                    .on_edit(|ctx, s| ctx.emit(EditorEvent::UpdatePath(s.to_string())));
                    Label::new(cx, "Status").top(Pixels(10.0));
                    Label::new(
                        cx,
                        Data::status_out.map(|x| {
                            let files_status = x.lock().read().file_status.clone();
                            format!("{:?}", files_status)
                        }),
                    )
                    .top(Pixels(10.0));
                });
            });
        })
        .border_width(Pixels(10.0));

        ResizeHandle::new(cx);
    })
}
