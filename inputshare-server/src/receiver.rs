use std::collections::VecDeque;
use std::convert::TryFrom;
use inputshare_common::{HidModifierKey, HidMouseButton, HidScanCode, MessageType, Vec2};
use std::io::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

type MouseType = i64;

#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    MouseMove(MouseType, MouseType),
    KeyPress(HidScanCode),
    KeyRelease(HidScanCode),
    ModifierPress(HidModifierKey),
    ModifierRelease(HidModifierKey),
    MouseButtonPress(HidMouseButton),
    MouseButtonRelease(HidMouseButton),
    HorizontalScrolling(i8),
    VerticalScrolling(i8)
}

#[derive(Debug)]
pub struct InputReceiver {
    packet_buffer: Vec<u8>,
    local_mouse_pos: Vec2<MouseType>,
    last_message: u64,
    events: VecDeque<InputEvent>
}

impl InputReceiver {

    pub fn new() -> Self {
        Self {
            local_mouse_pos: Vec2::new(0, 0),
            packet_buffer: Vec::new(),
            events: VecDeque::new(),
            last_message: 0
        }
    }

    pub fn get_event(&mut self) -> Option<InputEvent> {
        self.events.pop_front()
    }

    pub fn process_packet(&mut self, mut packet: &[u8]) -> Result<&[u8]> {
        let remote_mouse_pos = Vec2::new(
            packet.read_i64::<LittleEndian>()?,
            packet.read_i64::<LittleEndian>()?);

        if remote_mouse_pos != self.local_mouse_pos {
            self.events.push_back(InputEvent::MouseMove(
                remote_mouse_pos.x - self.local_mouse_pos.x,
                remote_mouse_pos.y - self.local_mouse_pos.y));
            self.local_mouse_pos = remote_mouse_pos;
        }

        let start_message = packet.read_u64::<LittleEndian>()?;
        let diff = self.last_message.checked_sub(start_message).unwrap_or(0);
        let len = packet.read_u8()? as u64;
        packet = &packet[(2 * diff as usize)..];
        for _ in diff..len {
            let msg_id = packet.read_u8()?;
            let msg_arg = packet.read_u8()?;
            match MessageType::try_from(msg_id) {
                Ok(MessageType::KeyPress) => self.events.push_back(InputEvent::KeyPress(msg_arg)),
                Ok(MessageType::KeyRelease) => self.events.push_back(InputEvent::KeyRelease(msg_arg)),
                Ok(MessageType::ModifierPress) => self.events.push_back(InputEvent::ModifierPress(HidModifierKey::from_bits(msg_arg).unwrap())),
                Ok(MessageType::ModifierRelease) => self.events.push_back(InputEvent::ModifierRelease(HidModifierKey::from_bits(msg_arg).unwrap())),
                Ok(MessageType::MouseButtonPress) => self.events.push_back(InputEvent::MouseButtonPress(HidMouseButton::from_bits(msg_arg).unwrap())),
                Ok(MessageType::MouseButtonRelease) => self.events.push_back(InputEvent::MouseButtonRelease(HidMouseButton::from_bits(msg_arg).unwrap())),
                Ok(MessageType::HorizontalScrolling) => self.events.push_back(InputEvent::HorizontalScrolling(msg_arg as i8)),
                Ok(MessageType::VerticalScrolling) => self.events.push_back(InputEvent::VerticalScrolling(msg_arg as i8)),
                Ok(MessageType::Reset) => {}
                Err(e) => println!("Invalid message: {}", e)
            }
        }
        self.last_message = start_message + len;


        self.packet_buffer.clear();
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.x)?;
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.y)?;
        self.packet_buffer.write_u64::<LittleEndian>(self.last_message)?;
        Ok(self.packet_buffer.as_slice())
    }

}