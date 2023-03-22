use std::collections::VecDeque;
use std::convert::TryFrom;
use inputshare_common::{ConsumerDeviceCode, HidButtonCode, HidKeyCode, MessageType, MouseType, Vec2};
use std::io::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    MouseMove(MouseType, MouseType),
    KeyPress(HidKeyCode),
    KeyRelease(HidKeyCode),
    MouseButtonPress(HidButtonCode),
    MouseButtonRelease(HidButtonCode),
    ConsumerDevicePress(ConsumerDeviceCode),
    ConsumerDeviceRelease(ConsumerDeviceCode),
    HorizontalScrolling(i8),
    VerticalScrolling(i8),
    Reset,
    Shutdown
}

#[derive(Debug)]
pub struct InputReceiver {
    local_sequence: u64,
    remote_sequence: u64,
    packet_buffer: Vec<u8>,
    local_mouse_pos: Vec2<MouseType>,
    last_message: u64,
    events: VecDeque<InputEvent>
}

impl InputReceiver {

    pub fn new() -> Self {
        Self {
            local_sequence: 1,
            local_mouse_pos: Vec2::new(0, 0),
            packet_buffer: Vec::new(),
            events: VecDeque::new(),
            last_message: 0,
            remote_sequence: 0,
        }
    }

    pub fn get_event(&mut self) -> Option<InputEvent> {
        self.events.pop_front()
    }

    pub fn process_packet(&mut self, mut packet: &[u8]) -> Result<Option<&[u8]>> {
        let sequence = packet.read_u64::<LittleEndian>()?;
        if sequence <= self.remote_sequence {
            return Ok(None);
        }
        self.remote_sequence = sequence;
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
        let diff = self.last_message.saturating_sub(start_message);
        let len = packet.read_u8()? as u64;
        packet = &packet[(2 * diff as usize)..];
        for i in diff..len {
            let msg_id = packet.read_u8()?;
            let msg_arg = packet.read_u8()?;
            match MessageType::try_from(msg_id) {
                Ok(MessageType::KeyPress) => self.events.push_back(InputEvent::KeyPress(HidKeyCode::from(msg_arg))),
                Ok(MessageType::KeyRelease) => self.events.push_back(InputEvent::KeyRelease(HidKeyCode::from(msg_arg))),
                Ok(MessageType::MouseButtonPress) => self.events.push_back(InputEvent::MouseButtonPress(HidButtonCode::from(msg_arg))),
                Ok(MessageType::MouseButtonRelease) => self.events.push_back(InputEvent::MouseButtonRelease(HidButtonCode::from(msg_arg))),
                Ok(MessageType::ConsumerDevicePress) => self.events.push_back(InputEvent::ConsumerDevicePress(ConsumerDeviceCode::from(msg_arg))),
                Ok(MessageType::ConsumerDeviceRelease) => self.events.push_back(InputEvent::ConsumerDeviceRelease(ConsumerDeviceCode::from(msg_arg))),
                Ok(MessageType::HorizontalScrolling) => self.events.push_back(InputEvent::HorizontalScrolling(msg_arg as i8)),
                Ok(MessageType::VerticalScrolling) => self.events.push_back(InputEvent::VerticalScrolling(msg_arg as i8)),
                Ok(MessageType::Reset) => self.events.push_back(InputEvent::Reset),
                Ok(MessageType::Shutdown) => self.events.push_back(InputEvent::Shutdown),
                Err(e) => tracing::warn!("Invalid message: {}", e)
            }
            self.last_message = start_message + i + 1;
        }

        self.packet_buffer.clear();
        self.packet_buffer.write_u64::<LittleEndian>(self.local_sequence)?;
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.x)?;
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.y)?;
        self.packet_buffer.write_u64::<LittleEndian>(self.last_message)?;
        self.local_sequence += 1;
        Ok(Some(self.packet_buffer.as_slice()))
    }

}