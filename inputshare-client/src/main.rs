#![windows_subsystem = "windows"]

mod sender;
mod windows;
mod conversions;
mod ui;

use native_windows_gui as nwg;
use std::cell::{Ref, RefCell, RefMut};
use std::convert::TryFrom;
use std::collections::HashSet;
use std::fmt::Arguments;
use std::fs;
use std::io::{Cursor, ErrorKind, Write};
use std::net::{ToSocketAddrs, UdpSocket};
use std::ops::Deref;
use std::path::Path;
use std::ptr::null_mut;
use std::rc::Rc;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use native_windows_gui::NativeUi;
use udp_connections::{Client, ClientDisconnectReason, ClientEvent, Endpoint, MAX_PACKET_SIZE};
use winapi::um::winuser::{GetCursorPos, PostMessageW, WM_KEYDOWN, WM_QUIT, WM_USER};
use inputshare_common::IDENTIFIER;
use winsock2_extensions::{NetworkEvents, WinSockExt};
use yawi::{HookType, Input, InputEvent, InputHook, KeyState, ScrollDirection, send_inputs, VirtualKey};
use crate::conversions::{f32_to_i8, vk_to_mb, wsc_to_cdc, wsc_to_hkc};
use crate::sender::InputSender;
use crate::ui::{InputShareApp, run_key_tester, StatusText};
use crate::windows::{get_message, wait_message_timeout};

const SOCKET: u32 = WM_USER + 1;
const CONNECT: u32 = WM_USER + 2;
const TOGGLED: u32 = WM_USER + 3;

fn main() -> Result<()>{
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    if std::env::args().any(|arg| arg == "print_keys") {
        return run_key_tester()
    }

    match client() {
        Ok(_) => Ok(()),
        Err(e) => {
            nwg::error_message("Error", &format!("The following error occured:\n{}", e));
            Err(e)
        }
    }
}

fn client() -> Result<()> {
    let mut config = Config::load(concat!(env!("CARGO_BIN_NAME"), ".json"))?;

    let mut input_transmitter: Option<InputTransmitter> = None;

    let app = InputShareApp::build_ui(Default::default())?;

    app.set_status(StatusText::NotConnected);
    app.info_label.set_visible(config.show_network_info);

    let socket = UdpSocket::bind(Endpoint::remote_any())?;
    socket.notify(app.window.handle.hwnd().unwrap(), SOCKET, NetworkEvents::Read)?;

    let mut socket = Client::new(socket, IDENTIFIER);
    println!("Running on {}", socket.local_addr()?);

    let mut last_network_label_update = Instant::now();
    let mut last_socket_update = Instant::now();
    let mut last_send = Instant::now();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    'outer: loop {
        wait_message_timeout(Some(match input_transmitter {
            Some(ref transmitter) if !transmitter.sender().in_sync() => Duration::from_secs_f32(1.0 / config.network_send_rate as f32),
            _ => Duration::from_millis(500)
        }))?;
        let mut socket_message = false;
        while let Some(msg) = get_message() {
            match msg.message {
                WM_QUIT => break 'outer,
                WM_KEYDOWN => if matches!(VirtualKey::try_from(msg.wParam as u8), Ok(VirtualKey::F1)) && (msg.lParam & (1 << 30)) == 0{
                    config.show_network_info = !config.show_network_info;
                    app.info_label.set_visible(config.show_network_info);
                },
                TOGGLED => {
                    match msg.wParam {
                        0 => app.set_status(StatusText::Remote),
                        1 => app.set_status(StatusText::Local),
                        _ => {}
                    }
                }
                CONNECT => {
                    if !socket.is_connected() {
                        match config.host_address.to_socket_addrs() {
                            Ok(mut addrs) => match addrs.find(|x| x.is_ipv4()) {
                                Some(addrs) => {
                                    socket.connect(addrs);
                                    app.connect_button.set_text("Connecting...");
                                    app.connect_button.set_enabled(false);
                                },
                                None => {
                                    app.show_error("Could not find address");
                                }
                            }
                            Err(e) => {
                                app.show_error(format_buf!(&mut buffer, "{}", e)?);
                            },
                        }
                    }
                    if socket.is_connected() {
                        socket.disconnect()?;
                        app.connect_button.set_text("Disconnecting...");
                        app.connect_button.set_enabled(false);
                    }
                    socket_message = true;
                }
                SOCKET => socket_message = true,
                _ => {}
            }
        }

        if socket_message || last_socket_update.elapsed() > Duration::from_millis(500) {
            socket.update();
            while let Some(event) = socket.next_event(&mut buffer)? {
                match event {
                    ClientEvent::Connected(id) => {
                        println!("Connected as {}", id);
                        input_transmitter = Some(InputTransmitter::new(&config)?);
                        app.connect_button.set_text("Disconnect");
                        app.connect_button.set_enabled(true);
                        app.set_status(StatusText::Local);
                    },
                    ClientEvent::Disconnected(reason) => {
                        println!("Disconnected: {:?}", reason);
                        input_transmitter = None;
                        if !matches!(reason, ClientDisconnectReason::Disconnected) {
                            app.show_error(format_buf!(&mut buffer, "Disconnected: {:?}", reason)?);
                        }
                        app.connect_button.set_text("Connect");
                        app.connect_button.set_enabled(true);
                        app.set_status(StatusText::NotConnected);
                    },
                    ClientEvent::PacketReceived(latest, payload) => {
                        if latest {
                            if let Some(ref mut transmitter) = input_transmitter {
                                transmitter.sender_mut().read_packet(payload)
                                    .unwrap_or_else(|e|println!("Packet decode error: {}", e));
                            }
                        }
                    },
                    _ => {}
                }
            }
            last_socket_update = Instant::now();
        }


        if let Some(mut sender) = input_transmitter.as_mut().map(|t|t.sender_mut()) {
            if socket.is_connected()  && !sender.in_sync() && last_send.elapsed() >= Duration::from_secs_f32(1.0 / config.network_send_rate as f32) {
                let _ = socket.send(sender.write_packet()?)?;
                last_send = Instant::now();
            }
        }

        if config.show_network_info && last_network_label_update.elapsed() >= Duration::from_millis(500) {
            let (rtt, pl) = match socket.connection() {
                Ok(connection) => (connection.rtt(), f32::round(100.0 * connection.packet_loss()) as u32),
                Err(_) => (0, 0)
            };
            app.info_label.set_text(format_buf!(&mut buffer, "{:>3}% {}ms", pl, rtt)?);
            last_network_label_update = Instant::now();
        }
    }

    if socket.is_connected() {
        socket.disconnect()?;
    }

    println!("Shutdown");

    Ok(())
}

struct InputTransmitter<'a> {
    _hook: InputHook<'a>,
    sender: Rc<RefCell<InputSender>>
}

impl<'a> InputTransmitter<'a> {

    fn new(config: &Config) -> Result<Self> {
        let sender = Rc::new(RefCell::new(InputSender::new(config.mouse_speed_factor)));
        let hook = {
            let input_events = sender.clone();
            let mut old_mouse_pos = unsafe {
                let mut point = std::mem::zeroed();
                GetCursorPos(&mut point);
                (point.x, point.y)
            };

            let blacklist = config.blacklist.clone();
            let hotkey = config.hotkey.clone();

            let mut captured = false;
            let mut pressed_keys = HashSet::new();

            InputHook::new(move |event|{
                if let Some(event) = event.to_key_event() {
                    if blacklist.contains(&event.key){
                        return true;
                    }
                }

                let should_handle = match event.to_key_event() {
                    Some(event) => match (pressed_keys.contains(&event.key), event.state) {
                        (false, KeyState::Pressed) => {
                            pressed_keys.insert(event.key);
                            true
                        },
                        (true, KeyState::Released) => {
                            pressed_keys.remove(&event.key);
                            true
                        },
                        _ => false
                    }
                    None => true
                };


                if should_handle {
                    match event.to_key_event() {
                        Some(event) if event.key == hotkey.trigger => {
                            if event.state == KeyState::Pressed && pressed_keys.is_superset(&hotkey.modifiers) {
                                if captured {
                                    input_events.deref().borrow_mut().reset();
                                } else {
                                    send_inputs(pressed_keys
                                        .iter()
                                        .copied()
                                        .filter(|k| *k != hotkey.trigger)
                                        .map(|k| match k.is_mouse_button() {
                                            true => Input::MouseButtonInput(k, KeyState::Released),
                                            false => Input::KeyboardKeyInput(k, KeyState::Released),
                                        })).unwrap_or_else(|e| println!("{}", e));
                                }
                                captured = !captured;
                                unsafe {
                                    PostMessageW(null_mut(), TOGGLED, if captured { 0 } else { 1 }, 0);
                                }
                                //println!("Input captured: {}", captured);
                            }
                            return false
                        }
                        _ => {}
                    }
                    if captured {
                        let mut sender = input_events.deref().borrow_mut();
                        match event {
                            InputEvent::MouseMoveEvent(x, y) => {
                                let (ox, oy) = old_mouse_pos;
                                sender.move_mouse((x - ox) as i64, (y - oy) as i64);
                                //old_mouse_pos = Some((x,y))
                            }
                            InputEvent::KeyboardKeyEvent(vk, sc, ks) => match wsc_to_hkc(sc) {
                                Some(kc) => match ks {
                                    KeyState::Pressed => sender.press_key(kc),
                                    KeyState::Released => sender.release_key(kc)
                                },
                                None => match wsc_to_cdc(sc){
                                    Some(cdc) => match ks {
                                        KeyState::Pressed => sender.press_consumer_device(cdc),
                                        KeyState::Released => sender.release_consumer_device(cdc)
                                    },
                                    None => if! matches!(sc, 0x21d) {
                                        println!("Unknown key: {} ({:x})", vk, sc)
                                    }
                                }
                            }
                            InputEvent::MouseButtonEvent(mb, ks) => match vk_to_mb(mb) {
                                Some(button) => match ks {
                                    KeyState::Pressed => sender.press_mouse_button(button),
                                    KeyState::Released => sender.release_mouse_button(button)
                                },
                                None => println!("Unknown mouse button: {}", mb)
                            }
                            InputEvent::MouseWheelEvent(sd) => match sd {
                                ScrollDirection::Horizontal(amount) => sender.scroll_horizontal(f32_to_i8(amount)),
                                ScrollDirection::Vertical(amount) => sender.scroll_vertical(f32_to_i8(amount))
                            }
                        }
                    } else if let InputEvent::MouseMoveEvent(x, y) = event {
                        old_mouse_pos = (x,y);
                    }
                }

                !captured
            }, true, HookType::KeyboardMouse)?
        };
        Ok(Self {
            _hook: hook,
            sender
        })
    }

    fn sender_mut(&mut self) -> RefMut<'_, InputSender> {
        self.sender.deref().borrow_mut()
    }

    fn sender(&self) -> Ref<'_, InputSender> {
        self.sender.deref().borrow()
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotkey {
    pub modifiers: HashSet<VirtualKey>,
    pub trigger: VirtualKey
}

impl<const N: usize> From<[VirtualKey; N]> for Hotkey {
    fn from(values: [VirtualKey; N]) -> Self {
        let trigger = *values.last().expect("hotkey must not be empty!");
        let mut modifiers = HashSet::from(values);
        modifiers.remove(&trigger);

        Self {
            modifiers,
            trigger
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host_address: String,
    pub hotkey: Hotkey,
    pub blacklist: HashSet<VirtualKey>,
    pub show_network_info: bool,
    pub network_send_rate: u32,
    pub mouse_speed_factor: f32
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host_address: String::from("raspberrypi.local:60067"),
            hotkey: Hotkey::from([VirtualKey::Apps]),
            blacklist: HashSet::from([
                VirtualKey::VolumeDown,
                VirtualKey::VolumeUp,
                VirtualKey::VolumeMute,
                VirtualKey::MediaStop,
                VirtualKey::MediaPrevTrack,
                VirtualKey::MediaPlayPause,
                VirtualKey::MediaNextTrack
            ]),
            show_network_info: false,
            network_send_rate: 100,
            mouse_speed_factor: 1.0
        }
    }
}

impl Config {

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        match fs::read_to_string(&path) {
            Ok(cfg) => Ok(serde_json::from_str(&cfg)?),
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                if let Some(parent) = path.as_ref().parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, serde_json::to_string_pretty(&Self::default())?)?;
                Ok(Self::default())
            }
            Err(e) => Err(e.into()),
        }
    }

}

fn inplace_format<'a>(buf: &'a mut [u8], args: Arguments<'_>) -> std::io::Result<&'a str> {
    let slice = {
        let mut cursor = Cursor::new(buf);
        cursor.write_fmt(args)?;
        let len = cursor.position() as usize;
        &cursor.into_inner()[..len]
    };
    Ok(unsafe { std::str::from_utf8_unchecked(slice) })
}

#[macro_export]
macro_rules! format_buf {
    ($dst:expr, $($arg:tt)*) => (inplace_format($dst, format_args!($($arg)*)))
}
