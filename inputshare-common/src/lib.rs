use std::net::TcpStream;
use std::io::{Read, Write, ErrorKind, BufRead, Error};
use std::borrow::Cow;
use std::convert::TryInto;
use std::num::TryFromIntError;

pub struct PackageIds {}
impl PackageIds {
    pub const KEYBOARD: u8 = 0x1;
    pub const MOUSE: u8 = 0x2;
}


pub trait ReadExt: Read {
    fn read_u8(&mut self) -> std::io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

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
