use iced::{Color, Element};
use iced::container::Style;
use iced::Background;
use crate::Message;

pub mod default;
pub mod settings;

pub enum ContainerStyle {
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

pub trait Page {
    fn view(&mut self) -> Element<Message>;
}