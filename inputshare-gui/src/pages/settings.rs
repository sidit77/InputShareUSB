use crate::pages::{Page, ContainerStyle};
use iced::{Element, Text, Column, Button, button, Row, Container, Space, Length, Color, Scrollable, scrollable, TextInput, text_input, Align};
use crate::Message;
use crate::config::Config;

#[derive(Default)]
pub struct SettingsPage {
    pub config: Config,
    save_button: button::State,
    discard_button: button::State,
    scroll_state: scrollable::State,
    port_state: text_input::State,
    host_state: text_input::State
}

impl SettingsPage {
    pub fn new() -> Self {
        Self {
            config: Config::load(),
            ..Default::default()
        }
    }
}

impl Page for SettingsPage {
    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(10)
            .spacing(8)
            .push(Text::new("Settings").size(50))
            .push(
                Container::new(
                    Scrollable::new(&mut self.scroll_state)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(10)
                        .spacing(10)
                        .push(
                            Row::new()
                                .spacing(4)
                                .push(
                                    Text::new("Host:")
                                        .size(25).width(Length::Fill))
                                .push(
                                    TextInput::new(&mut self.host_state, "Enter the hostname",
                                                   self.config.host.to_string().as_str(), Message::ChangeHostAddress)
                                        .size(25).width(Length::Fill)))
                        .push(
                            Row::new()
                                .spacing(4)
                                .push(
                                    Text::new("Port:")
                                        .size(25).width(Length::Fill))
                                .push(
                                    TextInput::new(&mut self.port_state, "Enter the port",
                                                   self.config.port.to_string().as_str(), Message::ChangePortNumber)
                                        .size(25).width(Length::Fill))))
                    .height(Length::Fill)
                    .width(Length::Units(500))
                    .style(ContainerStyle::Border))
            .push(
                Row::new()
                    .spacing(8)
                    .push(Button::new(&mut self.save_button, Text::new("Save")).on_press(Message::SaveSettings))
                    .push(Button::new(&mut self.discard_button, Text::new("Discard")).on_press(Message::SaveSettings))
            )
            .into()
        //
    }
}