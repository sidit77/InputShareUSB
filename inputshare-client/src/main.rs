#[macro_use]
extern crate bitflags;
extern crate native_windows_gui as nwg;

use crate::hid::{HidModifierKeys, convert_win2hid, HidScanCode, HidMouseButtons};
use crate::gui::SystemTray;
use nwg::NativeUi;
use std::time::Duration;
use std::net::{ToSocketAddrs, SocketAddr, TcpStream, Shutdown};
use std::io::{Write, Read};
use std::convert::TryFrom;
use yawi::{VirtualKey, InputEvent, KeyState, ScrollDirection, Input, InputHook};

mod gui;
mod hid;
mod config;

fn main(){
    println!("Hello client!");

    let cfg = config::Config::load();

    nwg::init().expect("Failed to init Native Windows GUI");
    let _ui = SystemTray::build_ui(Default::default()).expect("Failed to build UI");

    let server = match std::env::var("REMOTE_OVERRIDE") {
        Ok(s) => s.parse().expect("Can not parse address given with REMOTE_OVERRIDE"),
        Err(_) => cfg.remote_address
            .to_socket_addrs()
            .expect("Unable to resolve domain")
            .filter(|x|match x {
                SocketAddr::V4(_) => true,
                SocketAddr::V6(_) => false
            })
            .next()
            .expect("Can not find suitable address!")
    };

    println!("Connecting to {}", server);

    match TcpStream::connect(server) {
        Ok(mut stream) => {
            match do_handshake(&mut stream){
                Ok(_) => {
                    println!("Successfully connected to server");
                    run(&mut stream);
                    stream.shutdown(Shutdown::Both).unwrap();
                },
                Err(_) => {
                    println!("An handshake error occurred, terminating connection");
                    stream.shutdown(Shutdown::Both).unwrap();
                }
            }
        }
        Err(e) => println!("Failed to connect: {}", e)
    }
}

fn do_handshake(stream: &mut TcpStream) -> anyhow::Result<()> {
    println!("Starting handshake");
    let mut data = [0 as u8; 50];
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    //stream.read_line(&mut buffer)?;
    let buffer = read_string(stream, &mut data)?;
    println!("Got: {}", buffer.trim());
    if buffer.trim() != "Authenticate" {
        anyhow::bail!("Wrong protocol!");
    }
    stream.write_all(b"secretPassword\n")?;
    //stream.read_line(&mut buffer)?;
    let buffer = read_string(stream, &mut data)?;
    println!("Got: {}", buffer.trim());
    if buffer.trim() != "Ok" {
        anyhow::bail!("Wrong protocol!");
    }
    Ok(())
}

fn read_string(stream: &mut TcpStream, data: &mut [u8]) -> anyhow::Result<String> {
    let read = stream.read(data)?;
    Ok(String::from_utf8_lossy(&data[0..read]).to_string())
}

const BLACKLIST: [VirtualKey; 7] = [
    VirtualKey::VolumeDown,
    VirtualKey::VolumeUp,
    VirtualKey::VolumeMute,
    VirtualKey::MediaStop,
    VirtualKey::MediaPrevTrack,
    VirtualKey::MediaPlayPause,
    VirtualKey::MediaNextTrack
];

fn run(stream: &mut TcpStream) {
    let mut modifiers = HidModifierKeys::None;
    let mut pressed_buttons = HidMouseButtons::None;
    let mut pressed_keys = Vec::<(VirtualKey, HidScanCode)>::new();
    let mut captured = false;
    let mut pos: Option<(i32, i32)> = None;
    let _hook = InputHook::new(|event|{
        match event {
            InputEvent::KeyboardKeyEvent(key, scancode, state) => {
                if BLACKLIST.contains(&key){
                    return true;
                }
                let fresh = match HidModifierKeys::from_virtual_key(&key) {
                    Some(m) => {
                        let old = modifiers;
                        match state {
                            KeyState::Pressed => modifiers.insert(m),
                            KeyState::Released => modifiers.remove(m)
                        }
                        modifiers != old
                    }
                    None => match convert_win2hid(&scancode) {
                        Some(hid) => match state {
                            KeyState::Pressed => match pressed_keys.iter().position(|(_, x)| *x == hid) {
                                None => {
                                    pressed_keys.push((key, hid));
                                    true
                                },
                                Some(_) => false
                            }
                            KeyState::Released => match pressed_keys.iter().position(|(_, x)| *x == hid) {
                                Some(index) => {
                                    pressed_keys.remove(index);
                                    true
                                },
                                None => false
                            }
                        }
                        None => {
                            println!("Unsupported key: {:?} ({:x?})", key, scancode);
                            false
                        }
                    }
                };

                if fresh && matches!(key, VirtualKey::Apps) && matches!(state, KeyState::Pressed){
                    pressed_keys.retain(|(k, _)|!matches!(k, VirtualKey::Apps));
                    captured = !captured;
                    println!("Captured: {}", captured);
                    let mut k = modifiers.to_virtual_keys();
                    k.extend(pressed_keys.iter().map(|(x, _)|x));
                    if captured {
                        let mut k: Vec<Input> = k.into_iter().map(|key|Input::KeyboardKeyInput(key, KeyState::Released)).collect();
                        k.extend(pressed_buttons.to_virtual_keys().into_iter().map(|key|Input::MouseButtonInput(key, KeyState::Released)));
                        yawi::send_keys(k.iter()).expect("could not send all keys");
                        stream.write_all(&make_kb_packet(modifiers, Some(&pressed_keys))).expect("Error sending packet");
                        stream.write_all(&make_ms_packet(pressed_buttons, 0,0,0,0)).expect("Error sending packet");
                    } else {
                        stream.write_all(&make_kb_packet(HidModifierKeys::None, None)).expect("Error sending packet");
                        stream.write_all(&make_ms_packet(HidMouseButtons::None, 0, 0, 0, 0)).expect("Error sending packet");
                        let mut k: Vec<Input> = k.into_iter().map(|key|Input::KeyboardKeyInput(key, KeyState::Pressed)).collect();
                        k.extend(pressed_buttons.to_virtual_keys().into_iter().map(|key|Input::MouseButtonInput(key, KeyState::Pressed)));
                        yawi::send_keys(k.iter()).expect("could not send all keys");
                    }
                    return false;
                }

                if captured {
                    if fresh {
                        //println!("{:x?}", packet);
                        //sender.send(Packet::reliable_unordered(server, Vec::from(packet))).unwrap();
                        stream.write_all(&make_kb_packet(modifiers, Some(&pressed_keys))).expect("Error sending packet");
                        //println!("{:?} - {:x?}", modifiers, pressed_keys);
                    }
                    false
                }else {
                    true
                }

            }
            InputEvent::MouseButtonEvent(key, state) => {
                match HidMouseButtons::from_virtual_key(&key){
                    Some(mb) => match state {
                        KeyState::Pressed => pressed_buttons.insert(mb),
                        KeyState::Released => pressed_buttons.remove(mb),
                    }
                    None => println!("Unknown mouse button {:?}", key)
                }
                if captured {
                    stream.write_all(&make_ms_packet(pressed_buttons, 0, 0, 0, 0)).expect("Error sending packet");
                }
                !captured
            },
            InputEvent::MouseWheelEvent(dir) => {
                if captured {
                    match dir {
                        ScrollDirection::Horizontal(am) => stream.write_all(&make_ms_packet(pressed_buttons, 0, 0, 0, am as i8)).expect("Error sending packet"),
                        ScrollDirection::Vertical(am) => stream.write_all(&make_ms_packet(pressed_buttons, 0, 0, am as i8, 0)).expect("Error sending packet")
                    }
                }
                !captured
            },
            InputEvent::MouseMoveEvent(px, py) => {
                if pos.is_none() {
                    pos = Some((px, py));
                    return true;
                }
                if captured {
                    let (dx, dy) = match pos {
                        None => (0, 0),
                        Some((ox, oy)) => (px - ox, py - oy)
                    };
                    let (dx, dy) = (i16::try_from(dx).unwrap(), i16::try_from(dy).unwrap());
                    if dx != 0 || dy != 0 {
                        stream.write_all(&make_ms_packet(pressed_buttons, dx, dy, 0, 0)).expect("Error sending packet");
                    }
                } else {
                    pos = Some((px, py));
                }
                !captured
            }
        }
    });

    nwg::dispatch_thread_events();

    //nwg::dispatch_thread_events_with_callback(move ||{
    //    //socket.manual_poll(Instant::now());
    //    //receiver.try_recv();
    //});
}

fn make_kb_packet(mods: HidModifierKeys, keys: Option<&Vec<(VirtualKey, HidScanCode)>>) -> [u8; 9] {
    let mut packet = [0x0 as u8; 9];
    packet[0] = 0x1;
    packet[1] = mods.to_byte();
    if let Some(pressed_keys) = keys{
        for i in 0..pressed_keys.len().min(6) {
            packet[3 + i] = pressed_keys[0.max(pressed_keys.len() as i32 - 6) as usize + i].1;
        }
    }
    packet
}

fn make_ms_packet(buttons: HidMouseButtons, dx: i16, dy: i16, dv: i8, dh: i8) -> [u8; 9] {
    let mut packet = [0x0 as u8; 9];
    packet[0] = 0x2;
    packet[1] = buttons.to_byte();
    let dx = dx.to_le_bytes();
    let dy = dy.to_le_bytes();
    packet[2] = dx[0];
    packet[3] = dx[1];
    packet[4] = dy[0];
    packet[5] = dy[1];
    packet[6] = dv as u8;
    packet[7] = dh as u8;
    packet
}
