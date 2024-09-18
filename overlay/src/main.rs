use iced::widget::{button, column, row, text, text_input};
use iced::{event, Alignment, Command, Element, Event, Length, Theme};
use iced_layershell::actions::LayershellCustomActions;
use iced_layershell::reexport::Anchor;
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::Application;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), iced_layershell::Error> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    Counter::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 400)),
            anchor: Anchor::Bottom | Anchor::Right | Anchor::Left,
            keyboard_interactivity: iced_layershell::reexport::KeyboardInteractivity::None,
            ..Default::default()
        },
        flags: Some(rx),
        ..Default::default()
    })?;
    println!("test");

    Ok(())
}

struct Counter {
    value: i32,
    rx: RwLock<Receiver<()>>,
}

#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
    IcedEvent(Event),
}

impl Application for Counter {
    type Message = Message;
    type Flags = Option<Receiver<()>>;
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(flags: Option<Receiver<()>>) -> (Self, Command<Message>) {
        (
            Self {
                rx: RwLock::new(flags.unwrap()),
                value: 0,
            },
            Command::none(),
        )
    }

    fn namespace(&self) -> String {
        String::from("Counter - Iced")
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::subscription::unfold("led changes", (), move |_| async move {
            let num = self.rx.read().await.recv().await;
            (Message::IncrementPressed, ())
        })
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::IcedEvent(event) => {
                println!("hello {event:?}");
                Command::none()
            }
            Message::IncrementPressed => {
                self.value += 1;
                Command::none()
            }
            Message::DecrementPressed => {
                self.value -= 1;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let center = column![
            button("Increment").on_press(Message::IncrementPressed),
            text(self.value).size(50),
            button("Decrement").on_press(Message::DecrementPressed)
        ]
        .padding(20)
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill);
        row![column![center,].width(Length::Fill),]
            .padding(20)
            .spacing(10)
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
