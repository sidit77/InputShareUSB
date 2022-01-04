mod receiver;
mod configfs;

use std::convert::{TryFrom, TryInto};
use std::fmt::{Debug, Formatter};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::{SocketAddr};
use std::num::NonZeroU8;
use std::time::{Duration, Instant};
use mio::{Events, Interest, Poll, Token};
use anyhow::Result;
use clap::Parser;
use mio::net::UdpSocket;
use mio_signals::{Signal, Signals, SignalSet};
use udp_connections::{MAX_PACKET_SIZE, Server, ServerEvent, Transport};
use vec_map::VecMap;
use inputshare_common::{HidButtonCode, HidKeyCode, IDENTIFIER};
use crate::receiver::{InputEvent, InputReceiver};

/// The server for inputshare
#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    /// When set automatically moves the mouse every x seconds without input
    #[clap(short, long)]
    auto_movement_timeout: Option<u64>,

    /// Split each mouse movement command in up to x usb packets
    /// Higher values mean smoother movement but carry a higher risk of saturating the usb connection
    #[clap(short, long, default_value_t = 5)]
    mouse_tesselation_factor: u8,

    /// The interface that should be bound
    #[clap(short, long, default_value = "0.0.0.0:60067")]
    interface: String,
}

fn main() -> Result<()>{
    let args = Args::parse();

    configfs::enable_hid()?;

    let result = server(args);

    configfs::disable_hid()?;

    result
}

fn server(args: Args) -> Result<()> {
    println!("Opening HID devices");

    let mut mouse = Mouse::new(args.mouse_tesselation_factor.try_into()?)?;
    let mut keyboard = Keyboard::new()?;
    let mut consumer_device = ConsumerDevice::new()?;

    const SERVER: Token = Token(0);
    const SIGNAL: Token = Token(1);
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut signals = Signals::new(SignalSet::all())?;
    poll.registry().register(&mut signals, SIGNAL, Interest::READABLE)?;

    let mut socket = UdpSocket::bind(args.interface.parse()?)?;
    poll.registry().register(&mut socket, SERVER, Interest::READABLE)?;
    let mut socket = Server::new(MioSocket::from(socket), IDENTIFIER, 1);

    println!("Started server on {}", socket.local_addr()?);

    let mut last_input = Instant::now();
    let mut idle_move_x = -10;
    let mut receivers = VecMap::new();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    'outer: loop {
        let mut timeout = match receivers.is_empty() {
            true => None,
            false => Some(Duration::from_millis(550))
        };
        if let Some(secs) = args.auto_movement_timeout {
            let mut remaining = (last_input + Duration::from_secs(secs)).saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                mouse.move_by(idle_move_x, 0)?;
                idle_move_x *= -1;
                last_input = Instant::now();
                remaining = Duration::from_secs(secs);
            }
            timeout = match timeout {
                None => Some(remaining),
                Some(timout) => Some(timout.min(remaining))
            };
            timeout = timeout.map(|t|Duration::min(t, remaining)).or(Some(remaining))
        }

        poll.poll(&mut events, timeout)?;

        for event in events.iter() {
            if event.token() == SIGNAL {
                loop {
                    match signals.receive()? {
                        Some(Signal::Interrupt) => break 'outer,
                        Some(Signal::Quit) => break 'outer,
                        Some(Signal::Terminate) => break 'outer,
                        Some(_) => continue,
                        None => break
                    }
                }
            }
        }

        consumer_device.press_key(1 << 5)?;
        consumer_device.release_key(1 << 5)?;
        socket.update();
        loop {
            match socket.next_event(&mut buffer) {
                Ok(Some(event)) => match event {
                    ServerEvent::ClientConnected(client_id) => {
                        println!("Client {} connected", client_id);
                        receivers.insert(client_id.into(), InputReceiver::new());
                    },
                    ServerEvent::ClientDisconnected(client_id, reason) => {
                        println!("Client {} disconnected: {:?}", client_id, reason);
                        mouse.reset()?;
                        keyboard.reset()?;
                        consumer_device.reset()?;
                        receivers.remove(client_id.into());
                    },
                    ServerEvent::PacketReceived(client_id, latest, payload) => if latest {
                        if let Some(receiver) = receivers.get_mut(client_id.into()) {
                            if let Ok(packet) = receiver.process_packet(payload) {
                                socket.send(client_id, packet).unwrap_or_default();
                            }
                            while let Some(event) = receiver.get_event() {
                                match event {
                                    InputEvent::MouseMove(x, y) => mouse.move_by(x as i16, y as i16)?,
                                    InputEvent::KeyPress(key) => keyboard.press_key(key)?,
                                    InputEvent::KeyRelease(key) => keyboard.release_key(key)?,
                                    InputEvent::MouseButtonPress(button) => mouse.press_button(button)?,
                                    InputEvent::MouseButtonRelease(button) => mouse.release_button(button)?,
                                    InputEvent::HorizontalScrolling(amount) => mouse.scroll_horizontal(amount)?,
                                    InputEvent::VerticalScrolling(amount) => mouse.scroll_vertical(amount)?,
                                    InputEvent::Reset => {
                                        keyboard.reset()?;
                                        mouse.reset()?;
                                    }
                                }
                                last_input = Instant::now();
                                // println!("{:?}", event);
                            }
                        }

                    },
                    _ => {}
                }
                Ok(None) => break,
                Err(e) => {
                    println!("Receive error: {}", e);
                    break;
                }
            }
        }

    }

    mouse.reset()?;
    keyboard.reset()?;
    consumer_device.reset()?;

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
    pressed_modifiers: HidModifierKeys,
}

impl Keyboard {

    pub fn new() -> std::io::Result<Self> {
        let device = OpenOptions::new().write(true).append(true).open("/dev/hidg0")?;
        Ok(Self {
            device,
            pressed_keys: Vec::new(),
            pressed_modifiers: HidModifierKeys::empty()
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
        self.pressed_modifiers = HidModifierKeys::empty();
        self.send_report()
    }

    pub fn press_key(&mut self, key: HidKeyCode) -> std::io::Result<()> {
        match key.try_into() {
            Ok(modifier) => self.pressed_modifiers.insert(modifier),
            Err(_) => self.pressed_keys.push(key)
        }
        self.send_report()
    }

    pub fn release_key(&mut self, key: HidKeyCode) -> std::io::Result<()> {
        match key.try_into() {
            Ok(modifier) => self.pressed_modifiers.remove(modifier),
            Err(_) => self.pressed_keys.retain(|k| *k != key)
        }
        self.send_report()
    }

}

#[derive(Debug)]
pub struct ConsumerDevice {
    device: File,
    pressed_keys: u16,
}

impl ConsumerDevice {

    pub fn new() -> std::io::Result<Self> {
        let device = OpenOptions::new().write(true).append(true).open("/dev/hidg2")?;
        Ok(Self {
            device,
            pressed_keys: 0
        })
    }

    fn send_report(&mut self) -> std::io::Result<()> {
        self.device.write_all(&self.pressed_keys.to_le_bytes())
    }

    pub fn reset(&mut self) -> std::io::Result<()> {
        self.pressed_keys = 0;
        self.send_report()
    }

    pub fn press_key(&mut self, key: u16) -> std::io::Result<()> {
        self.pressed_keys |= key;
        self.send_report()
    }

    pub fn release_key(&mut self, key: u16) -> std::io::Result<()> {
        self.pressed_keys &= !key;
        self.send_report()
    }

}

#[derive(Debug)]
pub struct Mouse {
    device: File,
    pressed_buttons: HidMouseButtons,
    tess_factor: i16
}

impl Mouse {

    pub fn new(tess_factor: NonZeroU8) -> std::io::Result<Self> {
        let device = OpenOptions::new().write(true).append(true).open("/dev/hidg1")?;
        Ok(Self {
            device,
            pressed_buttons: HidMouseButtons::empty(),
            tess_factor: i16::from(tess_factor.get())
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
        self.pressed_buttons = HidMouseButtons::empty();
        self.send_report(0,0,0,0)
    }

    pub fn press_button(&mut self, button: HidButtonCode) -> std::io::Result<()> {
        match button.try_into() {
            Ok(button) => {
                self.pressed_buttons.insert(button);
                self.send_report(0, 0,0,0)
            },
            Err(_) => Ok(())
        }
    }

    pub fn release_button(&mut self, button: HidButtonCode) -> std::io::Result<()> {
        match button.try_into() {
            Ok(button) => {
                self.pressed_buttons.remove(button);
                self.send_report(0, 0,0,0)
            },
            Err(_) => Ok(())
        }
    }

    pub fn move_by(&mut self, mut dx: i16, mut dy: i16) -> std::io::Result<()> {
        let sx = abs_max(dx / self.tess_factor, dx.signum());
        let sy = abs_max(dy / self.tess_factor, dy.signum());
        while dx != 0 || dy != 0 {
            let tx = abs_min(dx, sx);
            let ty = abs_min(dy, sy);
            self.send_report(tx, ty, 0,0)?;
            dx -= tx;
            dy -= ty;
        }
        Ok(())
    }

    pub fn scroll_vertical(&mut self, amount: i8) -> std::io::Result<()> {
        self.send_report(0, 0, amount,0)
    }

    pub fn scroll_horizontal(&mut self, amount: i8) -> std::io::Result<()> {
        self.send_report(0, 0, 0, amount)
    }

}

fn abs_max(a: i16, b: i16) -> i16 {
    if a.abs() >= b.abs() {
        a
    } else {
        b
    }
}

fn abs_min(a: i16, b: i16) -> i16 {
    if a.abs() <= b.abs() {
        a
    } else {
        b
    }
}

pub use flags::{HidMouseButtons, HidModifierKeys};

#[allow(non_upper_case_globals)]
pub mod flags {
    use bitflags::bitflags;
    bitflags! {
        pub struct HidModifierKeys: u8 {
            const LCtrl   = 0x01;
            const LShift  = 0x02;
            const LAlt    = 0x04;
            const LMeta   = 0x08;
            const RCtrl   = 0x10;
            const RShift  = 0x20;
            const RAlt    = 0x40;
            const RMeta   = 0x80;
        }

        pub struct HidMouseButtons: u8 {
            const LButton = 0x01;
            const RButton = 0x02;
            const MButton = 0x04;
            const Button4 = 0x08;
            const Button5 = 0x10;
        }
    }
}

impl TryFrom<HidButtonCode> for HidMouseButtons {
    type Error = ();

    fn try_from(value: HidButtonCode) -> Result<Self, Self::Error> {
        match value {
            HidButtonCode::None => Err(()),
            HidButtonCode::LButton => Ok(HidMouseButtons::LButton),
            HidButtonCode::RButton => Ok(HidMouseButtons::RButton),
            HidButtonCode::MButton => Ok(HidMouseButtons::MButton),
            HidButtonCode::Button4 => Ok(HidMouseButtons::Button4),
            HidButtonCode::Button5 => Ok(HidMouseButtons::Button5)
        }
    }
}

impl TryFrom<HidKeyCode> for HidModifierKeys {
    type Error = ();

    fn try_from(value: HidKeyCode) -> Result<Self, Self::Error> {
        match value {
            HidKeyCode::LeftCtrl   => Ok(HidModifierKeys::LCtrl),
            HidKeyCode::LeftShift  => Ok(HidModifierKeys::LShift),
            HidKeyCode::LeftAlt    => Ok(HidModifierKeys::LAlt),
            HidKeyCode::LeftMeta   => Ok(HidModifierKeys::LMeta),
            HidKeyCode::RightCtrl  => Ok(HidModifierKeys::RCtrl),
            HidKeyCode::RightShift => Ok(HidModifierKeys::RShift),
            HidKeyCode::RightAlt   => Ok(HidModifierKeys::RAlt),
            HidKeyCode::RightMeta  => Ok(HidModifierKeys::RMeta),
            _ => Err(())
        }
    }
}