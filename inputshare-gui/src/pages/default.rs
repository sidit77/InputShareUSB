use iced::{Element, button, Length, Align, VerticalAlignment, HorizontalAlignment, Text, Column, Row, Button, Container, Space};
use crate::pages::{ContainerStyle, Page};
use crate::Message;
use crate::config::Config;
use std::net::{TcpStream, SocketAddr, ToSocketAddrs};
use std::ops::Add;
use anyhow::Error;
use std::path::Display;
use std::io::{Write, Read};
use std::borrow::Cow;
use std::time::Duration;

#[derive(Default)]
pub struct DefaultPage {
    config: Config,
    connection_status: String,
    connection: Option<TcpStream>,
    refresh_button: button::State,
    shutdown_button: button::State,
    settings_button: button::State
}

fn read_string<'a>(stream: &mut TcpStream, data: &'a mut [u8]) -> anyhow::Result<Cow<'a, str>> {
    let read = stream.read(data)?;
    Ok(String::from_utf8_lossy(&data[0..read]))
}

fn connect(host: &str, port: u16) -> anyhow::Result<TcpStream> {
    let address = String::from(host)
        .add(":")
        .add(port.to_string().as_str())
        .to_socket_addrs()?
        .filter(|x|match x {
            SocketAddr::V4(_) => true,
            SocketAddr::V6(_) => false
        })
        .next()
        .ok_or(anyhow::anyhow!("Not suitable address found!"))?;

    let mut stream = TcpStream::connect(address)?;

    let mut data = [0 as u8; 50];
    println!("Starting handshake");
    stream.write_all(b"Authenticate: InputShareUSB\n")?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    match read_string(&mut stream, &mut data)?.trim(){
        "Ok" => Ok(()),
        s => Err(anyhow::anyhow!("{}", s))
    }?;
    stream.set_read_timeout(None)?;

    Ok(stream)
}

impl DefaultPage {
    pub fn new() -> Self {
        let config = Config::load();

        let (connection, connection_status) = match connect(config.host.as_str(), config.port){
            Ok(connection) => (Some(connection), String::from("Connected")),
            Err(err) => (None, err.to_string())
        };

        Self {
            config,
            connection,
            connection_status,
            ..Default::default()
        }
    }
}

impl Page for DefaultPage {
    fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(10)
            .spacing(8)
            .push(
                Container::new(
                    Text::new(self.connection_status.as_str())
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
