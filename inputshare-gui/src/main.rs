use iced::{button, Align, Button, Column, Element, Sandbox, Settings, Text, Row, Length, VerticalAlignment, Color, Background, Container, Space, HorizontalAlignment, Scrollable};
use iced::container::Style;

pub fn main() -> iced::Result {
    Counter::run(Settings::default())
}

enum ContainerStyle {
    Colored(Color)
}

impl iced::container::StyleSheet for ContainerStyle {
    fn style(&self) -> Style {
        match self {
            ContainerStyle::Colored(color) => Style {
                background: Some(Background::from(*color)),
                ..Default::default()
            }
        }
    }
}

#[derive(Default)]
struct Counter {
    value: i32,
    refresh_button: button::State,
    shutdown_button: button::State,
    settings_button: button::State,
    start_client_button: button::State,
    start_server_button: button::State,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Sandbox for Counter {
    type Message = Message;

    fn new() -> Self {
        Self::default()
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
                Row::new()
                    .width(Length::Fill)
                    .spacing(8)
                    .align_items(Align::Center)
                    .push(Text::new("Online")
                        .width(Length::Fill)
                        .vertical_alignment(VerticalAlignment::Center)
                    )
                    .push(Button::new(&mut self.refresh_button, Text::new("Refresh"))
                        .on_press(Message::IncrementPressed)
                    )
                    .push(Button::new(&mut self.shutdown_button, Text::new("Power Off"))
                        .on_press(Message::IncrementPressed)
                    )
                    .push(Button::new(&mut self.settings_button, Text::new("Settings"))
                        .on_press(Message::IncrementPressed)
                    )
            )
            .push(
                Row::new()
                    .spacing(8)
                    .push(
                        Column::new()
                            .spacing(8)
                            .width(Length::Fill)
                            .push(Button::new(&mut self.start_client_button,
                                              Text::new("Start Client")
                                                  .width(Length::Fill)
                                                  .horizontal_alignment(HorizontalAlignment::Center))
                                    .width(Length::Fill)
                                    .on_press(Message::DecrementPressed)
                            )
                            .push(
                                Container::new(Space::new(Length::Fill, Length::Fill))
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .style(ContainerStyle::Colored(Color::from_rgb8(0, 230, 0)))
                            )
                            //.push(
                            //    Scrollable::new()
                            //)
                    )
                    .push(
                        Column::new()
                            .spacing(8)
                            .width(Length::Fill)
                            .push(Button::new(&mut self.start_server_button,
                                              Text::new("Start Server")
                                                  .width(Length::Fill)
                                                  .horizontal_alignment(HorizontalAlignment::Center))
                                .width(Length::Fill)
                                .on_press(Message::DecrementPressed)
                            )
                            .push(
                                Container::new(Space::new(Length::Fill, Length::Fill))
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .style(ContainerStyle::Colored(Color::from_rgb8(230, 0, 0)))
                            )
                    )
            )
            .into()
        //Column::new()
        //    .padding(20)
        //    .align_items(Align::Center)
        //    .push(
        //        Button::new(&mut self.increment_button, Text::new("Increment"))
        //            .on_press(Message::IncrementPressed),
        //    )
        //    .push(Text::new(self.value.to_string()).size(50))
        //    .push(
        //        Button::new(&mut self.decrement_button, Text::new("Decrement"))
        //            .on_press(Message::DecrementPressed),
        //    )
        //    .into()
    }
}