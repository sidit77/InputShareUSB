mod receiver;
mod configfs;

use std::fmt::{Debug, Formatter};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::{SocketAddr};
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use anyhow::Result;
use clap::Parser;
use mio::net::UdpSocket;
use mio_signals::{Signal, Signals, SignalSet};
use udp_connections::{Endpoint, MAX_PACKET_SIZE, Server, ServerEvent, Transport};
use vec_map::VecMap;
use inputshare_common::{HidKeyCode, HidModifierKey, HidMouseButton, IDENTIFIER};
use crate::receiver::{InputEvent, InputReceiver};

/// The server for inputshare
#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// When set automatically moves the mouse every x seconds without input
    #[clap(short, long)]
    idle_timeout: Option<u64>,

    /// The port that should be used
    #[clap(short, long, default_value_t = 60067)]
    port: u16,
}

fn main() -> Result<()>{
    let args = Args::parse();

    configfs::enable_hid()?;

    let result = server(args);

    configfs::disable_hid()?;

    result
}

fn server(args: Args) -> Result<()> {
    println!("Hello World!");

    let mut mouse = Mouse::new()?;
    let mut keyboard = Keyboard::new()?;

    const SERVER: Token = Token(0);
    const SIGNAL: Token = Token(1);
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut signals = Signals::new(SignalSet::all())?;
    poll.registry().register(&mut signals, SIGNAL, Interest::READABLE)?;

    let mut socket = UdpSocket::bind(Endpoint::remote_port(args.port))?;
    poll.registry().register(&mut socket, SERVER, Interest::READABLE)?;
    let mut socket = Server::new(MioSocket::from(socket), IDENTIFIER, 1);

    println!("running on {}", socket.local_addr()?);

    let mut receivers = VecMap::new();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    'outer: loop {
        poll.poll(&mut events, Some(Duration::from_secs(1)))?;


        for event in events.iter() {
            match event.token() {
                SIGNAL => loop {
                    match signals.receive()? {
                        Some(Signal::Interrupt) => break 'outer,
                        Some(Signal::Quit) => break 'outer,
                        Some(Signal::Terminate) => break 'outer,
                        Some(_) => continue,
                        None => break
                    }
                }
                _ => {}
            }
        }

        socket.update();
        while let Some(event) = socket.next_event(&mut buffer).unwrap() {
            match event {
                ServerEvent::ClientConnected(client_id) => {
                    println!("Client {} connected", client_id);
                    receivers.insert(client_id.into(), InputReceiver::new());
                },
                ServerEvent::ClientDisconnected(client_id, reason) => {
                    println!("Client {} disconnected: {:?}", client_id, reason);
                    mouse.reset()?;
                    keyboard.reset()?;
                    receivers.remove(client_id.into());
                },
                ServerEvent::PacketReceived(client_id, latest, payload) => {
                    if latest {
                        if let Some(receiver) = receivers.get_mut(client_id.into()) {
                            socket.send(client_id, receiver.process_packet(payload)?)?;
                            while let Some(event) = receiver.get_event() {
                                match event {
                                    InputEvent::MouseMove(x, y) => mouse.move_by(x as i16, y as i16)?,
                                    InputEvent::KeyPress(key) => keyboard.press_key(key)?,
                                    InputEvent::KeyRelease(key) => keyboard.release_key(key)?,
                                    InputEvent::ModifierPress(key) => keyboard.press_modifier(key)?,
                                    InputEvent::ModifierRelease(key) => keyboard.release_modifier(key)?,
                                    InputEvent::MouseButtonPress(button) => mouse.press_button(button)?,
                                    InputEvent::MouseButtonRelease(button) => mouse.release_button(button)?,
                                    InputEvent::HorizontalScrolling(amount) => mouse.scroll_horizontal(amount)?,
                                    InputEvent::VerticalScrolling(amount) => mouse.scroll_vertical(amount)?,
                                    InputEvent::Reset => {
                                        keyboard.reset()?;
                                        mouse.reset()?;
                                    }
                                }
                                //println!("{:?}", event);
                            }
                        }

                    }
                },
                _ => {}
            }
        }

    }

    mouse.reset()?;
    keyboard.reset()?;

    for client in socket.connected_clients().collect::<Vec<_>>() {
        socket.disconnect(client).unwrap();
    }

    println!("Shutting down");

    Ok(())
}

struct MioSocket(UdpSocket);

impl Debug for MioSocket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Transport for MioSocket {
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.0.send_to(buf, addr)
    }

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl From<UdpSocket> for MioSocket {
    fn from(socket: UdpSocket) -> Self {
        Self(socket)
    }
}

#[derive(Debug)]
pub struct Keyboard {
    device: File,
    pressed_keys: Vec<HidKeyCode>,
    pressed_modifiers: HidModifierKey,
}

impl Keyboard {

    pub fn new() -> std::io::Result<Self> {
        let device = OpenOptions::new().write(true).append(true).open("/dev/hidg0")?;
        Ok(Self {
            device,
            pressed_keys: Vec::new(),
            pressed_modifiers: HidModifierKey::empty()
        })
    }

    fn send_report(&mut self) -> std::io::Result<()> {
        let mut report = [0u8; 8];
        report[0] = self.pressed_modifiers.bits();

        for (i, key) in self.pressed_keys.iter().enumerate().take(6) {
            report[2 + i] = (*key).into()
        }

        self.device.write_all(&report)
    }

    pub fn reset(&mut self) -> std::io::Result<()> {
        self.pressed_keys.clear();
        self.pressed_modifiers = HidModifierKey::empty();
        self.send_report()
    }

    pub fn press_key(&mut self, key: HidKeyCode) -> std::io::Result<()> {
        self.pressed_keys.push(key);
        self.send_report()
    }

    pub fn release_key(&mut self, key: HidKeyCode) -> std::io::Result<()> {
        self.pressed_keys.retain(|k| *k != key);
        self.send_report()
    }

    pub fn press_modifier(&mut self, key: HidModifierKey) -> std::io::Result<()> {
        self.pressed_modifiers.insert(key);
        self.send_report()
    }

    pub fn release_modifier(&mut self, key: HidModifierKey) -> std::io::Result<()> {
        self.pressed_modifiers.remove(key);
        self.send_report()
    }

}

#[derive(Debug)]
pub struct Mouse {
    device: File,
    pressed_buttons: HidMouseButton
}

impl Mouse {

    pub fn new() -> std::io::Result<Self> {
        let device = OpenOptions::new().write(true).append(true).open("/dev/hidg1")?;
        Ok(Self {
            device,
            pressed_buttons: HidMouseButton::empty()
        })
    }

    fn send_report(&mut self, dx: i16, dy: i16, dv: i8, dh: i8) -> std::io::Result<()> {
        let mut report = [0u8; 7];
        report[0] = self.pressed_buttons.bits();

        report[1..=2].copy_from_slice(&dx.to_le_bytes());
        report[3..=4].copy_from_slice(&dy.to_le_bytes());
        report[5..=5].copy_from_slice(&dv.to_le_bytes());
        report[6..=6].copy_from_slice(&dh.to_le_bytes());

        self.device.write_all(&report)
    }

    pub fn reset(&mut self) -> std::io::Result<()> {
        self.pressed_buttons = HidMouseButton::empty();
        self.send_report(0,0,0,0)
    }

    pub fn press_button(&mut self, button: HidMouseButton) -> std::io::Result<()> {
        self.pressed_buttons.insert(button);
        self.send_report(0, 0,0,0)
    }

    pub fn release_button(&mut self, button: HidMouseButton) -> std::io::Result<()> {
        self.pressed_buttons.remove(button);
        self.send_report(0, 0,0,0)
    }

    pub fn move_by(&mut self, dx: i16, dy: i16) -> std::io::Result<()> {
        self.send_report(dx, dy, 0,0)
    }

    pub fn scroll_vertical(&mut self, amount: i8) -> std::io::Result<()> {
        self.send_report(0, 0, amount,0)
    }

    pub fn scroll_horizontal(&mut self, amount: i8) -> std::io::Result<()> {
        self.send_report(0, 0, 0, amount)
    }

}