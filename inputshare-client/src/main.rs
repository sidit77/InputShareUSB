#[macro_use]
extern crate bitflags;
extern crate native_windows_gui as nwg;

use crate::inputhook::InputEvent;
use crate::keys::{HidModifierKeys, KeyState, convert_win2hid, HidScanCode};
use crate::gui::SystemTray;
use nwg::NativeUi;
use laminar::{Socket, Packet};
use std::time::Instant;
use std::net::ToSocketAddrs;

mod gui;
mod inputhook;
mod keys;

const SERVER: &str = "raspberrypi.local:12351";

fn main() {
    println!("Hello client!");

    nwg::init().expect("Failed to init Native Windows GUI");
    let _ui = SystemTray::build_ui(Default::default()).expect("Failed to build UI");

    let addr = "127.0.0.1:12352";
    let mut socket = Socket::bind(addr).unwrap();
    let (sender, receiver) = (socket.get_packet_sender(), socket.get_event_receiver());
    println!("Connected on {}", addr);

    let server = SERVER
        .to_socket_addrs()
        .expect("Unable to resolve domain")
        .next()
        .unwrap();

    let mut modifiers = HidModifierKeys::None;
    let mut pressed_keys = Vec::<HidScanCode>::new();
    inputhook::set_up_keyboard_hook(move |event|{
        match event {
            InputEvent::KeyboardEvent(key, scancode, state) => {
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
                            KeyState::Pressed => match pressed_keys.contains(&hid) {
                                false => {
                                    pressed_keys.push(hid);
                                    true
                                },
                                true => false
                            }
                            KeyState::Released => match pressed_keys.iter().position(|x| *x == hid) {
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

                if fresh{
                    let mut packet: [u8; 8] = [0; 8];
                    packet[0] = modifiers.to_byte();
                    for i in 0..pressed_keys.len().min(6){
                        packet[2 + i] = pressed_keys[0.max(pressed_keys.len() as i32 - 6) as usize + i];
                    }
                    //println!("{:x?}", packet);
                    sender.send(Packet::reliable_unordered(server, Vec::from(packet))).unwrap();
                    //println!("{:?} - {:x?}", modifiers, pressed_keys);
                }

                false
            }
        }
    });


    nwg::dispatch_thread_events_with_callback(move ||{
        socket.manual_poll(Instant::now());
    });

    inputhook::release_hook();
/*
    let addr = "127.0.0.1:12352";
    let mut socket = Socket::bind(addr).unwrap();
    println!("Connected on {}", addr);

    let server = SERVER.parse().unwrap();

    println!("Type a message and press Enter to send. Send `Bye!` to quit.");

    let stdin = stdin();
    let mut s_buffer = String::new();

    loop {
        s_buffer.clear();
        stdin.read_line(&mut s_buffer).unwrap();
        let line = s_buffer.replace(|x| x == '\n' || x == '\r', "");

        socket.send(Packet::reliable_unordered(
            server,
            line.clone().into_bytes(),
        )).unwrap();

        socket.manual_poll(Instant::now());

        if line == "Bye!" {
            break;
        }

        match socket.recv() {
            Some(SocketEvent::Packet(packet)) => {
                if packet.addr() == server {
                    println!("Server sent: {}", String::from_utf8_lossy(packet.payload()));
                } else {
                    println!("Unknown sender.");
                }
            }
            Some(SocketEvent::Timeout(_)) => {}
            _ => println!("Silence.."),
        }
    }

    */
}
