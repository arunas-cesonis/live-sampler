use atomic_float::AtomicF32;
use crossbeam::atomic::AtomicCell;
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::{util, Editor, GuiContext};
use nih_plug_iced::widgets as nih_widgets;
use nih_plug_iced::*;
//use std::intrinsics::mir::Len;
// use std::marker::ConstParamTy;
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::sampler::Info;
use crate::AudioSamplerParams;

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<IcedState> {
    IcedState::from_size(800, 600)
}

#[derive(Clone)]
pub struct InfoBuffer {
    queue: Arc<ArrayQueue<Info>>,
    current: Info,
}

impl InfoBuffer {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(1)),
            current: Default::default(),
        }
    }
    pub fn publish(&mut self, info: Info) {
        self.queue.force_push(info);
    }
    pub fn update(&mut self) {
        if let Some(info) = self.queue.pop() {
            self.current = info;
        }
    }
    pub fn get(&self) -> &Info {
        &self.current
    }
}

pub(crate) fn create(
    params: Arc<AudioSamplerParams>,
    peak_meter: Arc<AtomicF32>,
    editor_state: Arc<IcedState>,
    info: Arc<ArrayQueue<Info>>,
) -> Option<Box<dyn Editor>> {
    create_iced_editor::<AudioSamplerEditor>(editor_state, (params, peak_meter, info))
}

struct AudioSamplerEditor {
    params: Arc<AudioSamplerParams>,
    context: Arc<dyn GuiContext>,

    peak_meter: Arc<AtomicF32>,
    info_queue: Arc<ArrayQueue<Info>>,
    info_current: Info,

    gain_slider_state: nih_widgets::param_slider::State,
    peak_meter_state: nih_widgets::peak_meter::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    /// Update a parameter's value.
    ParamUpdate(nih_widgets::ParamMessage),
}

impl IcedEditor for AudioSamplerEditor {
    type Executor = executor::Default;
    type Message = Message;
    type InitializationFlags = (
        Arc<AudioSamplerParams>,
        Arc<AtomicF32>,
        Arc<ArrayQueue<Info>>,
    );

    fn new(
        (params, peak_meter, info): Self::InitializationFlags,
        context: Arc<dyn GuiContext>,
    ) -> (Self, Command<Self::Message>) {
        let editor = AudioSamplerEditor {
            params,
            context,

            peak_meter,
            info_queue: info,
            info_current: Info::default(),

            gain_slider_state: Default::default(),
            peak_meter_state: Default::default(),
        };

        (editor, Command::none())
    }

    fn context(&self) -> &dyn GuiContext {
        self.context.as_ref()
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        message: Self::Message,
    ) -> Command<Self::Message> {
        match message {
            Message::ParamUpdate(message) => self.handle_param_message(message),
        }

        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        if let Some(info) = self.info_queue.pop() {
            self.info_current = info;
        }
        let info = &self.info_current;
        Column::new()
            .align_items(Alignment::Center)
            .push(
                Text::new("Audio Sampler")
                    .font(assets::NOTO_SANS_LIGHT)
                    .size(40)
                    .height(50.into())
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .vertical_alignment(alignment::Vertical::Bottom),
            )
            .push(
                Text::new("Gain")
                    .height(20.into())
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .vertical_alignment(alignment::Vertical::Center),
            )
            .push(
                nih_widgets::ParamSlider::new(&mut self.gain_slider_state, &self.params.volume)
                    .map(Message::ParamUpdate),
            )
            .push(Space::with_height(10.into()))
            .push(
                nih_widgets::PeakMeter::new(
                    &mut self.peak_meter_state,
                    util::gain_to_db(self.peak_meter.load(std::sync::atomic::Ordering::Relaxed)),
                )
                .hold_time(Duration::from_millis(600)),
            )
            .push(
                Container::new(
                    Text::new(format!("info: {:?}", info))
                        .height(40.into())
                        .width(Length::Fill)
                        .horizontal_alignment(alignment::Horizontal::Left)
                        .vertical_alignment(alignment::Vertical::Top),
                )
                .height(Length::Fill)
                .width(Length::Fill),
            )
            .push(
                Container::new(ProgressBar::new(0.0..=100.0, 35.0).height(20.into()))
                    .width(Length::Fill)
                    //.horizontal_alignment(alignment::Horizontal::Center)
                    //.vertical_alignment(alignment::Vertical::Center)
                    .padding(Padding::new(10)),
            )
            .into()
    }

    fn background_color(&self) -> nih_plug_iced::Color {
        nih_plug_iced::Color {
            r: 0.98,
            g: 0.98,
            b: 0.98,
            a: 1.0,
        }
    }
}
