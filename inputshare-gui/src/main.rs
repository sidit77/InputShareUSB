use iced::{button, Align, Button, Column, Element, Sandbox, Settings, Text, Row, Length, VerticalAlignment, Color, Background, Container, Space, HorizontalAlignment, Scrollable, scrollable, TextInput};
use iced::container::Style;
use iced::scrollable::Scrollbar;

pub fn main() -> iced::Result {
    Counter::run(Settings::default())
}

enum ContainerStyle {
    Colored(Color),
    Border,
    Text
}

impl iced::container::StyleSheet for ContainerStyle {
    fn style(&self) -> Style {
        match self {
            ContainerStyle::Colored(color) => Style {
                background: Some(Background::from(*color)),
                ..Default::default()
            },
            ContainerStyle::Border => Style{
                border_width: 2.0,
                border_radius: 4.0,
                border_color: Color::from_rgb8(230,230,230),
                ..Default::default()
            },
            ContainerStyle::Text => Style {
                background: Some(Background::from(Color::from_rgb8(230,230,230))),
                ..Default::default()
            }
        }
    }
}

#[derive(Default)]
struct Counter {
    value: i32,
    lines: Vec<String>,
    refresh_button: button::State,
    shutdown_button: button::State,
    settings_button: button::State,
    start_client_button: button::State,
    start_server_button: button::State,
    server_output: scrollable::State
}

#[derive(Debug, Clone)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Sandbox for Counter {
    type Message = Message;

    fn new() -> Self {
        Self{
            lines: std::fs::read_to_string("test.log")
                .expect("file not found!")
                .lines()
                .map(|s|String::from(s))
                .collect(),
            ..Default::default()
        }
    }

    fn title(&self) -> String {
        String::from("InputShareUSB")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::IncrementPressed => {
                self.value += 1;
            }
            Message::DecrementPressed => {
                self.value -= 1;
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(10)
            .spacing(8)
            .push(
                Container::new(
                    Text::new("Online")
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .vertical_alignment(VerticalAlignment::Center))
                    .style(ContainerStyle::Border)
                    .width(Length::Fill)
                    .padding(5)
            )
            .push(
                Row::new()
                    .width(Length::Fill)
                    .spacing(8)
                    .align_items(Align::Center)
                    .push(
                        Button::new(&mut self.refresh_button,
                                    Text::new("Refresh")
                                        .width(Length::Fill)
                                        .horizontal_alignment(HorizontalAlignment::Center))
                            .on_press(Message::IncrementPressed)
                            .width(Length::Fill)
                    )
                    .push(
                        Button::new(&mut self.shutdown_button,
                                    Text::new("Power Off")
                                        .width(Length::Fill)
                                        .horizontal_alignment(HorizontalAlignment::Center))
                            .on_press(Message::IncrementPressed)
                            .width(Length::Fill)
                    )
                    .push(
                        Button::new(&mut self.settings_button,
                                    Text::new("Settings")
                                        .width(Length::Fill)
                                        .horizontal_alignment(HorizontalAlignment::Center))
                            .on_press(Message::IncrementPressed)
                            .width(Length::Fill)
                    )
            )
            .push(
                Container::new(
                    Space::new(Length::Fill, Length::Fill))
                    .style(ContainerStyle::Border)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .into()
    }
}