#![windows_subsystem = "windows"]

mod sender;
mod windows;
mod conversions;

use native_windows_gui as nwg;
use std::cell::{RefCell, RefMut};
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::net::{ToSocketAddrs, UdpSocket};
use std::ops::Deref;
use std::path::Path;
use std::ptr::null_mut;
use std::rc::Rc;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use anyhow::Result;
use native_windows_derive::NwgUi;
use native_windows_gui::{CharEffects, MessageButtons, MessageIcons, MessageParams, NativeUi};
use udp_connections::{Client, ClientDisconnectReason, ClientEvent, Endpoint, MAX_PACKET_SIZE};
use winapi::um::winuser::{DispatchMessageW, GA_ROOT, GetAncestor, GetCursorPos, IsDialogMessageW, TranslateMessage, WM_QUIT, WM_USER, PostMessageW};
use inputshare_common::IDENTIFIER;
use winsock2_extensions::{NetworkEvents, WinSockExt};
use yawi::{HookType, InputEvent, InputHook, KeyState, ScrollDirection, VirtualKey};
use crate::conversions::{f32_to_i8, vk_to_mb, vk_to_mod, wsc_to_hkc};
use crate::sender::InputSender;
use crate::windows::{get_message, wait_message_timeout};

const SOCKET: u32 = WM_USER + 1;
const CONNECT: u32 = WM_USER + 2;
const TOGGLED: u32 = WM_USER + 3;

fn main() -> Result<()>{
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    match client() {
        Ok(_) => Ok(()),
        Err(e) => {
            nwg::error_message("Error", &format!("The following error occured:\n{}", e));
            Err(e)
        }
    }
}

fn client() -> Result<()> {
    let config = Config::load(concat!(env!("CARGO_BIN_NAME"), ".json"))?;

    let mut input_transmitter = None;

    let app = InputShareApp::build_ui(Default::default()).expect("Failed to build UI");
    app.set_status(StatusText::NotConnected);

    let socket = UdpSocket::bind(Endpoint::remote_any())?;
    socket.notify(app.window.handle.hwnd().unwrap(), SOCKET, NetworkEvents::Read)?;

    let mut socket = Client::new(socket, IDENTIFIER);
    println!("Running on {}", socket.local_addr()?);

    let mut last_send = Instant::now();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    'outer: loop {
        wait_message_timeout(Some(Duration::from_millis(100)))?;
        while let Some(mut msg) = get_message() {
            unsafe {
                if IsDialogMessageW(GetAncestor(msg.hwnd, GA_ROOT), &mut msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            match msg.message {
                WM_QUIT => break 'outer,
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
                            Ok(addrs) => match addrs.filter(|x| x.is_ipv4()).next() {
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
                                app.show_error(&format!("{}", e));
                            },
                        }
                    }
                    if socket.is_connected() {
                        socket.disconnect()?;
                        app.connect_button.set_text("Disconnecting...");
                        app.connect_button.set_enabled(false);
                    }
                }
                //SOCKET => println!("dfgd"),
                //    println!("FINALLY");
                //    let mut buf = [0u8; 1000];
                //    loop {
                //        match socket.recv_from(&mut buf) {
                //            Ok((size, src)) => println!("Got {:?} from {}", &buf[..size], src),
                //            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                //            Err(e) => Err(e)?
                //        }
//
                //    }
                //}
                _ => {}
            }
        }

        socket.update();
        while let Some(event) = socket.next_event(&mut buffer).unwrap() {
            match event {
                ClientEvent::Connected(id) => {
                    println!("Connected as {}", id);
                    input_transmitter = Some(InputTransmitter::new(&config.hotkey, &config.backlist)?);
                    app.connect_button.set_text("Disconnect");
                    app.connect_button.set_enabled(true);
                    app.set_status(StatusText::Local);
                },
                ClientEvent::Disconnected(reason) => {
                    println!("Disconnected: {:?}", reason);
                    input_transmitter = None;
                    if !matches!(reason, ClientDisconnectReason::Disconnected) {
                        app.show_error(&format!("Disconnected: {:?}", reason));
                    }
                    app.connect_button.set_text("Connect");
                    app.connect_button.set_enabled(true);
                    app.set_status(StatusText::NotConnected);
                },
                ClientEvent::PacketReceived(latest, payload) => {
                    if latest {
                        if let Some(ref mut transmitter) = input_transmitter {
                            transmitter.sender().read_packet(payload)?;
                        }
                    }
                    //println!("Packet {:?}", payload);
                },
                _ => {}
            }
        }
//
//
        if let Some(mut sender) = input_transmitter.as_mut().map(|t|t.sender()) {
            if socket.is_connected() && last_send.elapsed() >= Duration::from_millis(10) && !sender.in_sync() {
                let _ = socket.send(sender.write_packet()?)?;
                last_send = Instant::now();
                //println!("sending {}", i);
            }
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

    fn new(hotkey: &Hotkey, blacklist: &HashSet<VirtualKey>) -> Result<Self> {
        let sender = Rc::new(RefCell::new(InputSender::new()));
        let hook = {
            let input_events = sender.clone();
            let mut old_mouse_pos = unsafe {
                let mut point = std::mem::zeroed();
                GetCursorPos(&mut point);
                (point.x, point.y)
            };

            let blacklist = blacklist.clone();

            let mut captured = false;
            let hotkey = hotkey.clone();
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
                            InputEvent::KeyboardKeyEvent(vk, sc, ks) => match vk_to_mod(vk) {
                                Some(modifier) => match ks {
                                    KeyState::Pressed => sender.press_modifier(modifier),
                                    KeyState::Released => sender.release_modifier(modifier)
                                }
                                None => match wsc_to_hkc(sc) {
                                    Some(key) => match ks {
                                        KeyState::Pressed => sender.press_key(key),
                                        KeyState::Released => sender.release_key(key)
                                    },
                                    None => println!("Unknown key: {}", vk)
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
                    } else {
                        if let InputEvent::MouseMoveEvent(x, y) = event {
                            old_mouse_pos = (x,y);
                        }
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

    fn sender(&mut self) -> RefMut<'_, InputSender> {
        self.sender.deref().borrow_mut()
    }

}


#[derive(Default, NwgUi)]
pub struct InputShareApp {
    #[nwg_control(size: (300, 133), position: (300, 300), title: "InputShare Client", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

    #[nwg_control(text: "Not Connected", size: (280, 45), position: (10, 10), flags: "VISIBLE|DISABLED")]
    name_edit: nwg::RichLabel,

    #[nwg_control(text: "Connect", size: (280, 60), position: (10, 60))]
    #[nwg_events( OnButtonClick: [InputShareApp::connect] )]
    connect_button: nwg::Button

}

#[derive(Debug, Copy, Clone)]
enum StatusText {
    Local,
    Remote,
    NotConnected
}

impl StatusText {

    fn text(self) -> &'static str{
        match self {
            StatusText::Local => "Local",
            StatusText::Remote => "Remote",
            StatusText::NotConnected => "Not Connected",
        }
    }

    fn color(self) -> [u8; 3] {
        match self {
            StatusText::Local => [60, 140, 255],
            StatusText::Remote => [255, 80, 100],
            StatusText::NotConnected => [150, 150, 150],
        }
    }

}

impl InputShareApp {

    fn connect(&self) {
        //nwg::simple_message("Hello", &format!("Hello {}", self.name_edit.text()));
        unsafe {
            PostMessageW(null_mut(), CONNECT, 0, 0);
        }
    }

    fn set_status(&self, status: StatusText) {
        self.name_edit.set_text(status.text());
        self.name_edit.set_para_format(0..100,&nwg::ParaFormat {
            alignment: Some(nwg::ParaAlignment::Center),
            ..Default::default()
        });
        self.name_edit.set_char_format(0..100, &nwg::CharFormat {
            height: Some(500),
            effects: Some(CharEffects::BOLD),
            text_color: Some(status.color()),
            //font_face_name: Some("Comic Sans MS".to_string()),
            ..Default::default()
        });
    }

    fn show_error(&self, msg: &str) {
        nwg::modal_message(&self.window, &MessageParams {
            title: "Error",
            content: msg,
            buttons: MessageButtons::Ok,
            icons: MessageIcons::Error
        });
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
    pub backlist: HashSet<VirtualKey>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host_address: String::from("raspberrypi.local:60067"),
            hotkey: Hotkey::from([VirtualKey::Apps]),
            backlist: HashSet::from([
                VirtualKey::VolumeDown,
                VirtualKey::VolumeUp,
                VirtualKey::VolumeMute,
                VirtualKey::MediaStop,
                VirtualKey::MediaPrevTrack,
                VirtualKey::MediaPlayPause,
                VirtualKey::MediaNextTrack
            ])
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