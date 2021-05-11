use std::net::TcpStream;
use yawi::{VirtualKey, InputHook, KeyState, InputEvent, Input, ScrollDirection};
use crate::hid::{HidScanCode, HidMouseButtons, HidModifierKeys, convert_win2hid};
use std::io::{Write, stdin};
use std::convert::TryFrom;
use inputshare_common::PackageIds;
use byteorder::{WriteBytesExt, LittleEndian};
use std::io;

pub fn run_client(stream: &mut TcpStream, hotkey: VirtualKey, blacklist: &Vec<VirtualKey>) -> anyhow::Result<()> {
    let mut modifiers = HidModifierKeys::None;
    let mut pressed_buttons = HidMouseButtons::None;
    let mut pressed_keys = Vec::<(VirtualKey, HidScanCode)>::new();
    let mut captured = false;
    let mut hotkey = HotKey::new(hotkey);
    let mut pos: Option<(i32, i32)> = None;

    let _hook = InputHook::new(|event|{
        if let Some(triggered) = hotkey.triggered(&event) {
            if triggered {
                captured = !captured;
                println!("Captured: {}", captured);
                if captured {
                    set_local_state(&modifiers, &pressed_buttons, &pressed_keys, KeyState::Released).expect("could not send all keys");
                    stream.write_packet(Packet::from_key_list(modifiers, &pressed_keys)).expect("Error sending packet");
                    stream.write_packet(Packet::Mouse(pressed_buttons, 0,0,0,0)).expect("Error sending packet");
                } else {
                    stream.write_packet(Packet::reset_keyboard()).expect("Error sending packet");
                    stream.write_packet(Packet::reset_mouse()).expect("Error sending packet");
                    set_local_state(&modifiers, &pressed_buttons, &pressed_keys, KeyState::Pressed).expect("could not send all keys");
                }
            }
            return false;
        }
        if let Some(event) = event.to_key_event() {
            if blacklist.contains(&event.key){
                return true;
            }
        }
        match event {
            InputEvent::KeyboardKeyEvent(key, scancode, state) => {
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
                        stream.write_packet(Packet::from_key_list(modifiers, &pressed_keys)).expect("Error sending packet");
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
                    stream.write_packet(Packet::Mouse(pressed_buttons, 0, 0, 0, 0)).expect("Error sending packet");
                }
                !captured
            },
            InputEvent::MouseWheelEvent(dir) => {
                if captured {
                    match dir {
                        ScrollDirection::Horizontal(am) => stream.write_packet(Packet::Mouse(pressed_buttons, 0, 0, 0, am as i8)).expect("Error sending packet"),
                        ScrollDirection::Vertical(am) => stream.write_packet(Packet::Mouse(pressed_buttons, 0, 0, am as i8, 0)).expect("Error sending packet")
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
                        stream.write_packet(Packet::Mouse(pressed_buttons, dx, dy, 0, 0)).expect("Error sending packet");
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

    Ok(())
}

struct HotKey {
    key: VirtualKey,
    available: bool
}

impl HotKey {
    fn new(key: VirtualKey) -> Self {
        Self{
            key,
            available: true
        }
    }
    fn triggered(&mut self, event: &InputEvent) -> Option<bool> {
        if let Some(event) = event.to_key_event() {
            if self.key == event.key {
                match event.state {
                    KeyState::Pressed => {
                        if self.available {
                            self.available = false;
                            return Some(true);
                        }
                    },
                    KeyState::Released => self.available = true
                }
                return Some(false)
            }
        }
        None
    }
}

fn set_local_state(
    modifiers: &HidModifierKeys,
    pressed_buttons: &HidMouseButtons,
    pressed_keys: &Vec<(VirtualKey, HidScanCode)>,
    state: KeyState) -> anyhow::Result<()> {

    let mut k = modifiers.to_virtual_keys();
    k.extend(pressed_keys.iter().map(|(x, _)|x));
    let mut k: Vec<Input> = k.into_iter().map(|key|Input::KeyboardKeyInput(key, state)).collect();
    k.extend(pressed_buttons.to_virtual_keys().into_iter().map(|key|Input::MouseButtonInput(key, state)));
    yawi::send_inputs(k.as_slice())
}

#[derive(Debug)]
enum Packet {
    Keyboard(HidModifierKeys, [Option<HidScanCode>; 6]),
    Mouse(HidMouseButtons, i16, i16, i8, i8)
}

impl Packet {
    fn from_key_list(mods: HidModifierKeys, keys: &Vec<(VirtualKey, HidScanCode)>) -> Self{
        Packet::Keyboard(mods, keys.iter().rev().take(6).rev()
            .fold(([None; 6], 0), |(mut acc, i), n|{
                acc[i] = Some(n.1);
                (acc, i + 1)
            }).0)
    }
    fn reset_keyboard() -> Self {
        Packet::Keyboard(HidModifierKeys::None, [None; 6])
    }
    fn reset_mouse() -> Self {
        Packet::Mouse(HidMouseButtons::None, 0, 0, 0, 0)
    }
}

trait WritePacket: Write {
    fn write_packet(&mut self, packet: Packet) -> io::Result<()>{
        match packet {
            Packet::Keyboard(modifiers, keys) => {
                self.write_u8(PackageIds::KEYBOARD)?;
                self.write_u8(modifiers.to_byte())?;
                self.write_u8(0)?;
                for key in &keys {
                    match key {
                        None => self.write_u8(0)?,
                        Some(kc) => self.write_u8(*kc)?
                    }
                }
                self.flush()
            }
            Packet::Mouse(buttons, dx, dy, dv, dh) => {
                self.write_u8(PackageIds::MOUSE)?;
                self.write_u8(buttons.to_byte())?;
                self.write_i16::<LittleEndian>(dx)?;
                self.write_i16::<LittleEndian>(dy)?;
                self.write_i8(dv)?;
                self.write_i8(dh)?;
                self.flush()
            }
        }
    }
}

impl WritePacket for TcpStream {}
