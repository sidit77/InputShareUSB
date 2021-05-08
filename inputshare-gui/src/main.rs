use crate::pages::default::DefaultPage;
use iced::{Sandbox, Element, Settings};
use crate::pages::Page;
use crate::pages::settings::SettingsPage;

mod pages;
mod config;

pub fn main() -> iced::Result {
    InputShareClient::run(Settings::default())
}

enum InputShareClient {
    DefaultPage(DefaultPage),
    SettingsPage(SettingsPage)
}


#[derive(Debug, Clone)]
pub enum Message {
    OpenSettings,
    SaveSettings,
    DiscardSettings,
    ChangePortNumber(String),
    ChangeHostAddress(String),
    Default
}

impl Sandbox for InputShareClient {
    type Message = Message;

    fn new() -> Self {
        Self::DefaultPage(DefaultPage::new())
    }

    fn title(&self) -> String {
        String::from("InputShareUSB")
    }

    fn update(&mut self, message: Message) {
        match self {
            InputShareClient::DefaultPage(_) => match message {
                Message::OpenSettings => *self = InputShareClient::SettingsPage(SettingsPage::new()),
                _ => {}
            },
            InputShareClient::SettingsPage(page) => match message {
                Message::SaveSettings => {
                    page.config.save();
                    *self = InputShareClient::DefaultPage(DefaultPage::new());
                },
                Message::DiscardSettings => *self = InputShareClient::DefaultPage(DefaultPage::new()),
                Message::ChangePortNumber(mut string) => {
                   string.insert(0, '0');
                    match string.parse() {
                        Ok(port) => page.config.port = port,
                        Err(_) => {}
                    }
                },
                Message::ChangeHostAddress(str) => page.config.host = str,
                _ => {}
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match self {
            InputShareClient::DefaultPage(page) => page.view(),
            InputShareClient::SettingsPage(page) => page.view()
        }
    }
}