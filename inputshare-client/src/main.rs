mod sender;
mod windows;
mod conversions;

use std::cell::RefCell;
use std::io::{Error, ErrorKind};
use std::net::{ToSocketAddrs, UdpSocket};
use std::rc::Rc;
use std::time::{Duration, Instant};
use anyhow::Result;
use udp_connections::{Client, ClientEvent, Endpoint, MAX_PACKET_SIZE};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{DispatchMessageW, PostThreadMessageW, TranslateMessage, WM_QUIT, WM_USER};
use inputshare_common::IDENTIFIER;
use winsock2_extensions::{NetworkEvents, WinSockExt};
use yawi::{HookType, InputEvent, InputHook, KeyState, ScrollDirection, VirtualKey};
use crate::conversions::{f32_to_i8, vk_to_mb, vk_to_mod, wsc_to_hsc};
use crate::sender::InputSender;
use crate::windows::{create_window, get_message, wait_message_timeout};

fn main() -> Result<()>{
    {
        let thread_id = unsafe {GetCurrentThreadId()};
        ctrlc::set_handler(move || {
            unsafe {PostThreadMessageW(thread_id, WM_QUIT, 0 ,0)};
        })?;
    }

    let input_events:Rc<RefCell<Option<InputSender>>> = Rc::new(RefCell::new(None));

    let hook = {
        let input_events = input_events.clone();
        let mut old_mouse_pos = None;
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

            if let Some(sender) = (*input_events).borrow_mut().as_mut(){
                if should_handle {
                    match event.to_key_event() {
                        Some(event) if event.key == hotkey => {
                            if event.state == KeyState::Pressed {
                                captured = !captured;
                                println!("Input captured: {}", captured);
                                old_mouse_pos = None;
                            }
                            return false
                        }
                        _ => {}
                    }
                    if captured {
                        match event {
                            InputEvent::MouseMoveEvent(x, y) => {
                                if let Some((ox, oy)) = old_mouse_pos {
                                    sender.move_mouse((x - ox) as i64, (y - oy) as i64);
                                }
                                old_mouse_pos = Some((x,y))
                            }
                            InputEvent::KeyboardKeyEvent(vk, sc, ks) => match vk_to_mod(vk) {
                                Some(modifier) => match ks {
                                    KeyState::Pressed => sender.press_modifier(modifier),
                                    KeyState::Released => sender.release_modifier(modifier)
                                }
                                None => match wsc_to_hsc(sc) {
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

    let handle = create_window("Dummy Window");//window.handle.hwnd().unwrap();//unsafe{GetConsoleWindow()};
    //let timer = unsafe {SetTimer(handle, 1, 1000, None) };
    //println!("{:?} {:?}", timer, handle);

    const SOCKET: u32 = WM_USER + 1;

    let socket = UdpSocket::bind(Endpoint::remote_any())?;
    socket.notify(handle, SOCKET, NetworkEvents::Read)?;

    let mut socket = Client::new(socket, IDENTIFIER);
    println!("Running on {}", socket.local_addr()?);
    let server = "raspberrypi.local:12345"
        .to_socket_addrs()?
        .filter(|x| x.is_ipv4())
        .next()
        .ok_or(Error::new(ErrorKind::AddrNotAvailable, "Could not find suitable address!"))?;
    println!("Connecting to {}", server);
    socket.connect(server)?;

    let mut last_send = Instant::now();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    'outer: loop {
        wait_message_timeout(Some(Duration::from_millis(100)))?;
        while let Some(msg) = get_message() {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            match msg.message {
                WM_QUIT => {
                    if socket.is_connected() {
                        socket.disconnect()?;
                    } else {
                        break 'outer
                    }

                },
                //SOCKET => {
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
                },
                ClientEvent::Disconnected(reason) => {
                    println!("Disconnected: {:?}", reason);
                    *(*input_events).borrow_mut() = None;
                    break 'outer
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


        if let Some(sender) = (*input_events).borrow_mut().as_mut() {
            if socket.is_connected() && last_send.elapsed() >= Duration::from_millis(10) && !sender.in_sync() {
                let i = socket.send(sender.write_packet()?)?;
                last_send = Instant::now();
                println!("sending {}", i);
            }
        }

    }

    println!("Shutdown");

    hook.remove();

    Ok(())
}

