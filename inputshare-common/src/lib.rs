use std::net::TcpStream;
use std::io::{Read, Write, ErrorKind, Error};
use std::borrow::Cow;
use std::convert::TryInto;
use std::fmt::Debug;

pub const DEFAULT_PORT: u16 = 60067;

pub const IDENTIFIER: &str = "inputshare-usb";

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2<T> where T: Debug + Copy + PartialEq {
    pub x: T,
    pub y: T
}

impl<T> Vec2<T> where T: Debug + Copy + PartialEq {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}


pub struct PackageIds {}
impl PackageIds {
    pub const KEYBOARD: u8 = 0x1;
    pub const MOUSE: u8 = 0x2;
    pub const SWITCH: u8 = 0x3;
}


pub trait ReadExt: Read {

    fn read_u16(&mut self) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_string<'a>(&mut self, buf: &'a mut [u8]) -> std::io::Result<Cow<'a, str>> {
        let size = self.read_u16()? as usize;
        self.read_exact(&mut buf[0..size])?;
        Ok(String::from_utf8_lossy(&buf[0..size]))
    }

}

pub trait WriteExt: Write {
    fn write_u16(&mut self, value: u16) -> std::io::Result<()> {
        let b = value.to_be_bytes();
        self.write_all(&b[..])
    }
    fn write_string(&mut self, value: &str) -> std::io::Result<()>{
        let b = value.as_bytes();
        match TryInto::<u16>::try_into(b.len()){
            Ok(l) => self.write_u16(l),
            Err(err) => Err(Error::new(ErrorKind::InvalidData, err))
        }?;
        self.write_all(b)
    }
}

impl ReadExt for TcpStream {}
impl WriteExt for TcpStream {}
