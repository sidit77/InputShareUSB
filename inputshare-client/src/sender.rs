use std::collections::VecDeque;
use std::io::{Result, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use inputshare_common::{ConsumerDeviceCode, HidButtonCode, HidKeyCode, MessageType, MouseType, Vec2};

#[derive(Debug)]
pub struct InputSender {
    local_sequence: u64,
    remote_sequence: u64,
    packet_buffer: Vec<u8>,
    local_mouse_pos: Vec2<MouseType>,
    local_mouse_pos_raw: Vec2<MouseType>,
    mouse_speed_factor: f64,
    remote_mouse_pos: Vec2<MouseType>,
    message_queue: VecDeque<[u8; 2]>,
    last_message: u64
}

impl InputSender {
    pub fn new(mouse_speed_factor: f32) -> Self {
        Self {
            local_sequence: 1,
            remote_sequence: 0,
            packet_buffer: Vec::new(),
            local_mouse_pos: Vec2::new(0, 0),
            local_mouse_pos_raw: Vec2::new(0, 0),
            mouse_speed_factor: mouse_speed_factor.into(),
            remote_mouse_pos: Vec2::new(0, 0),
            message_queue: VecDeque::new(),
            last_message: 0
        }
    }

    pub fn move_mouse(&mut self, x: MouseType, y: MouseType) {
        if (self.mouse_speed_factor - 1.0).abs() > f64::EPSILON {
            self.local_mouse_pos_raw.x += x;
            self.local_mouse_pos_raw.y += y;
            self.local_mouse_pos.x = f64::round(self.local_mouse_pos_raw.x as f64 * self.mouse_speed_factor) as MouseType;
            self.local_mouse_pos.y = f64::round(self.local_mouse_pos_raw.y as f64 * self.mouse_speed_factor) as MouseType;
        } else {
            self.local_mouse_pos.x += x;
            self.local_mouse_pos.y += y;
        }
    }

    pub fn shutdown_remote(&mut self) {
        self.message_queue
            .push_back([MessageType::Shutdown.into(), 0])
    }

    pub fn reset(&mut self) {
        self.message_queue.push_back([MessageType::Reset.into(), 0])
    }

    pub fn press_key(&mut self, key: HidKeyCode) {
        self.message_queue
            .push_back([MessageType::KeyPress.into(), key.into()])
    }

    pub fn release_key(&mut self, key: HidKeyCode) {
        self.message_queue
            .push_back([MessageType::KeyRelease.into(), key.into()])
    }

    pub fn press_mouse_button(&mut self, button: HidButtonCode) {
        self.message_queue
            .push_back([MessageType::MouseButtonPress.into(), button.into()])
    }

    pub fn release_mouse_button(&mut self, button: HidButtonCode) {
        self.message_queue
            .push_back([MessageType::MouseButtonRelease.into(), button.into()])
    }

    pub fn press_consumer_device(&mut self, button: ConsumerDeviceCode) {
        self.message_queue
            .push_back([MessageType::ConsumerDevicePress.into(), button.into()])
    }

    pub fn release_consumer_device(&mut self, button: ConsumerDeviceCode) {
        self.message_queue
            .push_back([MessageType::ConsumerDeviceRelease.into(), button.into()])
    }

    pub fn scroll_horizontal(&mut self, amount: i8) {
        self.message_queue
            .push_back([MessageType::HorizontalScrolling.into(), amount as u8])
    }

    pub fn scroll_vertical(&mut self, amount: i8) {
        self.message_queue
            .push_back([MessageType::VerticalScrolling.into(), amount as u8])
    }

    pub fn in_sync(&self) -> bool {
        self.local_mouse_pos == self.remote_mouse_pos && self.message_queue.is_empty()
    }

    pub fn read_packet(&mut self, mut packet: &[u8]) -> Result<()> {
        let sequence = packet.read_u64::<LittleEndian>()?;
        if sequence <= self.remote_sequence {
            return Ok(());
        }
        self.remote_sequence = sequence;
        self.remote_mouse_pos.x = packet.read_i64::<LittleEndian>()?;
        self.remote_mouse_pos.y = packet.read_i64::<LittleEndian>()?;
        let received_index = packet.read_u64::<LittleEndian>()?;
        let diff = received_index.saturating_sub(self.last_message);
        self.message_queue.drain(..(diff as usize));
        self.last_message = received_index;

        Ok(())
    }

    pub fn write_packet(&mut self) -> Result<&[u8]> {
        self.packet_buffer.clear();
        self.packet_buffer
            .write_u64::<LittleEndian>(self.local_sequence)?;
        self.packet_buffer
            .write_i64::<LittleEndian>(self.local_mouse_pos.x)?;
        self.packet_buffer
            .write_i64::<LittleEndian>(self.local_mouse_pos.y)?;
        self.packet_buffer
            .write_u64::<LittleEndian>(self.last_message)?;
        let size_index = self.packet_buffer.len();
        self.packet_buffer.write_u8(0)?;
        for i in 0..usize::min(self.message_queue.len(), u8::MAX as usize) {
            self.packet_buffer[size_index] += 1;
            self.packet_buffer
                .write_all(self.message_queue.get(i).unwrap())?;
        }
        self.local_sequence += 1;
        Ok(self.packet_buffer.as_slice())
    }
}
