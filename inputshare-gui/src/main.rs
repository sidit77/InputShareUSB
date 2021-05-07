use iced::{button, Align, Button, Column, Element, Sandbox, Settings, Text, Row, Length, VerticalAlignment, Color, Background, Container, Space, HorizontalAlignment, Scrollable, scrollable};
use iced::container::Style;
use iced::scrollable::Scrollbar;

pub fn main() -> iced::Result {
    Counter::run(Settings::default())
}

enum ContainerStyle {
    Colored(Color),
    Border
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

#[derive(Debug, Clone, Copy)]
enum Message {
    IncrementPressed,
    DecrementPressed,
}

impl Sandbox for Counter {
    type Message = Message;

    fn new() -> Self {
        Self{
            lines: std::fs::read_to_string("inputshare-server/src/main.rs")
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
                            .push(Container::new(
                               // Scrollable::new(&mut self.server_output)
                               //     .push(Text::new("Test").size(50))
                               //     .push(Text::new("Iced supports scrollable content. Try it out! Find the button further below."))
                               //     .push(Text::new("Tip: You can use the scrollbar to scroll down faster!").size(16))
                               //     .push(Column::new().height(Length::Units(4096)))
                               //     .push(Text::new("You are halfway there!").width(Length::Fill).size(30).horizontal_alignment(HorizontalAlignment::Center))
                               //     .push(Column::new().height(Length::Units(4096)))
                               //     .push(Text::new("You made it!").width(Length::Fill).size(50).horizontal_alignment(HorizontalAlignment::Center))
                               self.lines
                                   .iter()
                                   .map(|l|Text::new(l))
                                   .fold(Scrollable::new(&mut self.server_output), |a, t|a.push(t))
                            ).style(ContainerStyle::Border).padding(8))
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