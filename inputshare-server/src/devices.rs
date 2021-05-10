use std::fs::{File, OpenOptions};
use std::io::Write;
use crate::Packet;
use crate::args::BackendType;
use std::io;
use std::path::Path;

#[derive(Debug)]
enum Backend {
    LinuxDevice(File),
    Console,
}

impl Backend {
    fn from_device_path<P: AsRef<Path>>(path: P) -> io::Result<Backend> {
        let device = OpenOptions::new().write(true).append(true).open(path)?;
        Ok(Backend::LinuxDevice(device))
    }
}

#[derive(Debug)]
pub struct Devices {
    keyboard: Backend,
    mouse: Backend
}

impl Devices {

    pub fn from_backend_type(backend_type: BackendType) -> io::Result<Self> {
        let result = match backend_type {
            BackendType::Hardware => Self {
                keyboard: Backend::from_device_path("/dev/hidg0")?,
                mouse: Backend::from_device_path("/dev/hidg1")?
            },
            BackendType::Console => Self {
                keyboard: Backend::Console,
                mouse: Backend::Console
            }
        };
        Ok(result)
    }

    pub fn handle_packet(&mut self, packet: Packet) -> std::io::Result<()> {
        match packet {
            Packet::Keyboard(msg) => match self.keyboard {
                Backend::LinuxDevice(ref mut device) => device.write_all(msg),
                Backend::Console => Ok(println!("Keyboard: {:?}", msg))
            },
            Packet::Mouse(msg) => match self.mouse {
                Backend::LinuxDevice(ref mut device) => device.write_all(msg),
                Backend::Console => match has_movement(&msg) {
                    false => Ok(println!("Mouse: {:?}", msg)),
                    true => Ok(())
                }
            }
        }
    }
}

fn has_movement(msg: &[u8]) -> bool {
    msg[1..4].iter().any(|x|*x !=0 )
}