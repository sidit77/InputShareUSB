use std::collections::VecDeque;
use inputshare_common::{HidModifierKeys, HidMouseButtons, HidScanCode, MessageType, Vec2};
use std::io::{Result, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

type MouseType = i64;

#[derive(Debug)]
pub struct InputSender {
    packet_buffer: Vec<u8>,
    local_mouse_pos: Vec2<MouseType>,
    remote_mouse_pos: Vec2<MouseType>,
    message_queue: VecDeque<[u8; 2]>,
    last_message: u64
}

impl InputSender {
    
    pub fn new() -> Self {
        Self{
            packet_buffer: Vec::new(),
            local_mouse_pos: Vec2::new(0, 0),
            remote_mouse_pos: Vec2::new(0, 0),
            message_queue: VecDeque::new(),
            last_message: 0
        }
    }

    pub fn move_mouse(&mut self, x: MouseType, y: MouseType) {
        self.local_mouse_pos.x += x;
        self.local_mouse_pos.y += y;
    }

    pub fn press_key(&mut self, key: HidScanCode) {
        self.message_queue.push_back([MessageType::KeyPress as u8, key])
    }

    pub fn release_key(&mut self, key: HidScanCode) {
        self.message_queue.push_back([MessageType::KeyRelease as u8, key])
    }

    pub fn press_modifier(&mut self, key: HidModifierKeys) {
        self.message_queue.push_back([MessageType::ModifierPress as u8, key.bits()])
    }

    pub fn release_modifier(&mut self, key: HidModifierKeys) {
        self.message_queue.push_back([MessageType::ModifierRelease as u8, key.bits()])
    }

    pub fn press_mouse_button(&mut self, button: HidMouseButtons) {
        self.message_queue.push_back([MessageType::MouseButtonPress as u8, button.bits()])
    }

    pub fn release_mouse_button(&mut self, button: HidMouseButtons) {
        self.message_queue.push_back([MessageType::MouseButtonRelease as u8, button.bits()])
    }

    pub fn scroll_horizontal(&mut self, amount: i8) {
        self.message_queue.push_back([MessageType::HorizontalScrolling as u8, amount as u8])
    }

    pub fn scroll_vertical(&mut self, amount: i8) {
        self.message_queue.push_back([MessageType::VerticalScrolling as u8, amount as u8])
    }

    pub fn in_sync(&self) -> bool{
        self.local_mouse_pos == self.remote_mouse_pos && self.message_queue.is_empty()
    }

    pub fn read_packet(&mut self, mut packet: &[u8]) -> Result<()> {
        self.remote_mouse_pos.x = packet.read_i64::<LittleEndian>()?;
        self.remote_mouse_pos.y = packet.read_i64::<LittleEndian>()?;
        let received_index = packet.read_u64::<LittleEndian>()?;
        let diff = received_index.checked_sub(self.last_message).unwrap_or(0);
        self.message_queue.drain(..diff);
        self.last_message = received_index;

        Ok(())
    }

    pub fn write_packet(&mut self) -> Result<&[u8]> {
        self.packet_buffer.clear();
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.x)?;
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.y)?;
        self.packet_buffer.write_u64::<LittleEndian>(self.last_message)?;
        let size_index = self.packet_buffer.len();
        self.packet_buffer.write_u8(0)?;
        for i in 0..usize::min(self.message_queue.len(), u8::MAX as usize) {
            self.packet_buffer[size_index] += 1;
            self.packet_buffer.write_all(self.message_queue.get(i).unwrap())?;
        }

        Ok(self.packet_buffer.as_slice())
    }

}