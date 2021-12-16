use inputshare_common::Vec2;
use std::io::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

type MouseType = i64;

#[derive(Debug)]
pub struct InputSender {
    packet_buffer: Vec<u8>,
    local_mouse_pos: Vec2<MouseType>,
    remote_mouse_pos: Vec2<MouseType>
}

impl InputSender {
    
    pub fn new() -> Self {
        Self{
            packet_buffer: Vec::new(),
            local_mouse_pos: Vec2::new(0, 0),
            remote_mouse_pos: Vec2::new(0, 0)
        }
    }

    pub fn move_mouse<X: Into<MouseType>, Y: Into<MouseType>>(&mut self, x: X, y: Y) {
        self.local_mouse_pos.x += x.into();
        self.local_mouse_pos.y += y.into();
    }

    pub fn in_sync(&self) -> bool{
        self.local_mouse_pos == self.remote_mouse_pos
    }

    pub fn read_packet(&mut self, mut packet: &[u8]) -> Result<()> {
        self.remote_mouse_pos.x = packet.read_i64::<LittleEndian>()?;
        self.remote_mouse_pos.y = packet.read_i64::<LittleEndian>()?;

        Ok(())
    }

    pub fn write_packet(&mut self) -> Result<&[u8]> {
        self.packet_buffer.clear();
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.x)?;
        self.packet_buffer.write_i64::<LittleEndian>(self.local_mouse_pos.y)?;

        Ok(self.packet_buffer.as_slice())
    }

}