#![windows_subsystem = "windows"]

mod sender;
mod windows;
mod conversions;

use native_windows_gui as nwg;
use std::cell::RefCell;
use std::net::{ToSocketAddrs, UdpSocket};
use std::ptr::null_mut;
use std::rc::Rc;
use std::time::{Duration, Instant};
use anyhow::Result;
use native_windows_derive::NwgUi;
use native_windows_gui::{CharEffects, NativeUi};
use udp_connections::{Client, ClientDisconnectReason, ClientEvent, Endpoint, MAX_PACKET_SIZE};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{DispatchMessageW, GA_ROOT, GetAncestor, GetCursorPos, IsDialogMessageW, PostThreadMessageW, TranslateMessage, WM_QUIT, WM_USER, PostMessageW};
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


    let server = "raspberrypi.local:12345";


    {
        let thread_id = unsafe {GetCurrentThreadId()};
        ctrlc::set_handler(move || {
            unsafe {PostThreadMessageW(thread_id, WM_QUIT, 0 ,0)};
        })?;
    }

    let input_events:Rc<RefCell<Option<InputSender>>> = Rc::new(RefCell::new(None));

    let hook = {
        let input_events = input_events.clone();
        let mut old_mouse_pos = unsafe {
            let mut point = std::mem::zeroed();
            GetCursorPos(&mut point);
            (point.x, point.y)
        };

        let mut captured = false;
        let hotkey = VirtualKey::Apps;
        let mut pressed_keys = Vec::new();

        InputHook::new(move |event|{
            let should_handle = match event.to_key_event() {
                Some(event) => match (pressed_keys.contains(&event.key), event.state) {
                    (false, KeyState::Pressed) => {
                        pressed_keys.push(event.key);
                        true
                    },
                    (true, KeyState::Released) => {
                        pressed_keys.retain(|k| *k != event.key);
                        true
                    },
                    _ => false
                }
                None => true
            };

            if !captured {
                if let InputEvent::MouseMoveEvent(x, y) = event {
                    old_mouse_pos = (x,y);
                }
            }

            if let Some(sender) = (*input_events).borrow_mut().as_mut(){
                if should_handle {
                    match event.to_key_event() {
                        Some(event) if event.key == hotkey => {
                            if event.state == KeyState::Pressed {
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
                    }
                }

                !captured
            } else {
                captured = false;
                true
            }
        }, true, HookType::KeyboardMouse)?
    };

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
                        match server.to_socket_addrs() {
                            Ok(addrs) => match addrs.filter(|x| x.is_ipv4()).next() {
                                Some(addrs) => {
                                    socket.connect(addrs)?;
                                    app.connect_button.set_text("Connecting...");
                                    app.connect_button.set_enabled(false);
                                },
                                None => {
                                    nwg::error_message("Error", "Could not find address");
                                }
                            }
                            Err(e) => {
                                nwg::error_message("Error", &format!("{}", e));
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

        socket.update().unwrap();
        while let Some(event) = socket.next_event(&mut buffer).unwrap() {
            match event {
                ClientEvent::Connected(id) => {
                    println!("Connected as {}", id);
                    *(*input_events).borrow_mut() = Some(InputSender::new());
                    app.connect_button.set_text("Disconnect");
                    app.connect_button.set_enabled(true);
                    app.set_status(StatusText::Local);
                },
                ClientEvent::Disconnected(reason) => {
                    println!("Disconnected: {:?}", reason);
                    *(*input_events).borrow_mut() = None;
                    if !matches!(reason, ClientDisconnectReason::Disconnected) {
                        nwg::simple_message("Disconnected", &format!("Disconnected: {:?}", reason));
                    }
                    app.connect_button.set_text("Connect");
                    app.connect_button.set_enabled(true);
                    app.set_status(StatusText::NotConnected);
                },
                ClientEvent::PacketReceived(latest, payload) => {
                    if latest {
                        if let Some(sender) = (*input_events).borrow_mut().as_mut() {
                            sender.read_packet(payload)?;
                        }
                    }
                    //println!("Packet {:?}", payload);
                },
                ClientEvent::PacketAcknowledged(_) => {
                    //println!("{} got acknowledged", seq);
                }
            }
        }
//
//
        if let Some(sender) = (*input_events).borrow_mut().as_mut() {
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

    hook.remove();

    Ok(())
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

}