#[macro_use]
extern crate bitflags;

use crate::hid::{HidModifierKeys, convert_win2hid, HidScanCode, HidMouseButtons};
use std::time::Duration;
use std::net::{ToSocketAddrs, SocketAddr, TcpStream, Shutdown};
use std::io::{Write, Read, stdin};
use std::convert::TryFrom;
use yawi::{VirtualKey, InputEvent, KeyState, ScrollDirection, Input, InputHook};
use std::borrow::Cow;

mod hid;
mod config;

fn main(){
    println!("Hello client!");

    let cfg = config::Config::load();

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
                },
                Err(err) => {
                    println!("{}", err);
                    stream.shutdown(Shutdown::Both).unwrap();
                }
            }
        }
        Err(e) => println!("Failed to connect: {}", e)
    }
}

fn do_handshake(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut data = [0 as u8; 50];
    println!("Starting handshake");
    stream.write_all(b"Authenticate: InputShareUSB\n")?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    match read_string(stream, &mut data)?.trim(){
        "Ok" => Ok(()),
        s => Err(anyhow::anyhow!("{}", s))
    }
}

fn read_string<'a>(stream: &mut TcpStream, data: &'a mut [u8]) -> anyhow::Result<Cow<'a, str>> {
    let read = stream.read(data)?;
    Ok(String::from_utf8_lossy(&data[0..read]))
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

const HOTKEY: VirtualKey = VirtualKey::Apps;

fn run(stream: &mut TcpStream) {
    let mut modifiers = HidModifierKeys::None;
    let mut pressed_buttons = HidMouseButtons::None;
    let mut pressed_keys = Vec::<(VirtualKey, HidScanCode)>::new();
    let mut captured = false;
    let mut hk_available = true;
    let mut pos: Option<(i32, i32)> = None;
    let _hook = InputHook::new(|event|{
        match event {
            InputEvent::KeyboardKeyEvent(key, scancode, state) => {
                if BLACKLIST.contains(&key){
                    return true;
                }
                if matches!(key, HOTKEY){
                    match state {
                        KeyState::Pressed => if hk_available {
                            hk_available = false;
                            captured = !captured;
                            println!("Captured: {}", captured);
                            let mut k = modifiers.to_virtual_keys();
                            k.extend(pressed_keys.iter().map(|(x, _)|x));
                            if captured {
                                let mut k: Vec<Input> = k.into_iter().map(|key|Input::KeyboardKeyInput(key, KeyState::Released)).collect();
                                k.extend(pressed_buttons.to_virtual_keys().into_iter().map(|key|Input::MouseButtonInput(key, KeyState::Released)));
                                yawi::send_inputs(k.as_slice()).expect("could not send all keys");
                                stream.write_all(&make_kb_packet(modifiers, Some(&pressed_keys))).expect("Error sending packet");
                                stream.write_all(&make_ms_packet(pressed_buttons, 0,0,0,0)).expect("Error sending packet");
                            } else {
                                stream.write_all(&make_kb_packet(HidModifierKeys::None, None)).expect("Error sending packet");
                                stream.write_all(&make_ms_packet(HidMouseButtons::None, 0, 0, 0, 0)).expect("Error sending packet");
                                let mut k: Vec<Input> = k.into_iter().map(|key|Input::KeyboardKeyInput(key, KeyState::Pressed)).collect();
                                k.extend(pressed_buttons.to_virtual_keys().into_iter().map(|key|Input::MouseButtonInput(key, KeyState::Pressed)));
                                yawi::send_inputs(k.as_slice()).expect("could not send all keys");
                            }
                        }
                        KeyState::Released => hk_available = true
                    }
                    return false;
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

    let quitter = yawi::Quitter::from_current_thread();
    ctrlc::set_handler(move ||{
        quitter.quit();
        println!("Stopping!");
    }).expect("Cant set ctrl c handler!");

    let quitter = yawi::Quitter::from_current_thread();
    std::thread::spawn(move || {
        let mut s = String::new();
        loop {
            stdin().read_line(&mut s).expect("Cant read stdin!");
            if s.trim().eq("stop") {
                break;
            }
        }
        quitter.quit();
    });

    yawi::run();

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
    packet[1] = match (dx, dy) {
        (0, 0) => 0x1,
        _      => 0x2
    };
    packet[2] = buttons.to_byte();
    let dx = dx.to_le_bytes();
    let dy = dy.to_le_bytes();
    packet[3] = dx[0];
    packet[4] = dx[1];
    packet[5] = dy[0];
    packet[6] = dy[1];
    packet[7] = dv as u8;
    packet[8] = dh as u8;
    packet
}
