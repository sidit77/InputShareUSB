use std::collections::VecDeque;
use inputshare_common::Vec2;
use std::io::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

type MouseType = i64;

#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    MouseMove(MouseType, MouseType)
}

#[derive(Debug)]
pub struct InputReceiver {
    packet_buffer: Vec<u8>,
    local_mouse_pos: Vec2<MouseType>,
    events: VecDeque<InputEvent>
}

impl InputReceiver {

    pub fn new() -> Self {
        Self {
            local_mouse_pos: Vec2::new(0, 0),
            packet_buffer: Vec::new(),
            events: VecDeque::new()
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

        self.packet_buffer.clear();
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.x)?;
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.y)?;
        Ok(self.packet_buffer.as_slice())
    }

}