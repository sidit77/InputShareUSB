use iced::{Element, button, Sandbox, Length, Align, VerticalAlignment, HorizontalAlignment, Text, Column, Row, Button, Container, Space};
use crate::pages::{ContainerStyle, Page};
use crate::Message;

#[derive(Default)]
pub struct DefaultPage {
    value: i32,
    refresh_button: button::State,
    shutdown_button: button::State,
    settings_button: button::State
}

impl Page for DefaultPage {
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
                            .on_press(Message::Default)
                            .width(Length::Fill)
                    )
                    .push(
                        Button::new(&mut self.shutdown_button,
                                    Text::new("Power Off")
                                        .width(Length::Fill)
                                        .horizontal_alignment(HorizontalAlignment::Center))
                            .on_press(Message::Default)
                            .width(Length::Fill)
                    )
                    .push(
                        Button::new(&mut self.settings_button,
                                    Text::new("Settings")
                                        .width(Length::Fill)
                                        .horizontal_alignment(HorizontalAlignment::Center))
                            .on_press(Message::OpenSettings)
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
