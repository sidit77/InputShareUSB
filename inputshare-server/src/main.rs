use std::fs::{OpenOptions, File};
use std::io::{Write, Read, stdout};
use std::time::Duration;
use std::net::{TcpListener, TcpStream, Shutdown, SocketAddr, IpAddr};
use std::str::FromStr;
use std::thread;
use std::sync::{Mutex, TryLockError, Arc};
use std::borrow::Cow;

//TODO write zero into devices on error

#[derive(Debug, Copy, Clone)]
enum MousePacketType {
    Movement, Default
}

impl MousePacketType {
    fn from_u8(id: u8) -> Option<Self>{
        match id {
            0x1 => Some(MousePacketType::Default),
            0x2 => Some(MousePacketType::Movement),
            _   => None
        }
    }
}

#[derive(Debug)]
enum PacketType<'a> {
    Keyboard(&'a[u8]), Mouse(MousePacketType, &'a[u8])
}

impl<'a> PacketType<'a> {
    fn from_packet(packet: &'a[u8]) -> Option<Self> {
        match packet[0] {
            0x1 => Some(PacketType::Keyboard(&packet[1..9])),
            0x2 => match MousePacketType::from_u8(packet[1]){
                Some(mpt) => Some(PacketType::Mouse(mpt, &packet[2..9])),
                None => None
            },
            _   => None
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum BackendType {
    Hardware,
    Console
}

impl FromStr for BackendType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hardware" => Ok(BackendType::Hardware),
            "console" => Ok(BackendType::Console),
            _ => Err(anyhow::anyhow!("[{}] is a viable backend type. Supported types: [hardware, console]", s))
        }

    }
}

#[derive(Debug)]
enum Backend {
    LinuxDevice(File),
    Console(&'static str)
}

impl Write for Backend {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Backend::LinuxDevice(device) => device.write(buf),
            Backend::Console(name) => {println!("{}: {:?}", name, buf); Ok(buf.len())}
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Backend::LinuxDevice(device) => device.flush(),
            Backend::Console(_) => stdout().flush()
        }
    }
}

fn main() -> anyhow::Result<()>{

    let port: u16 = std::env::args()
        .nth(1)
        .ok_or(anyhow::anyhow!("Wrong number of arguments (USAGE: {} <PORT> <hardware | console>)", env!("CARGO_BIN_NAME")))?
        .parse()?;

    let backend_type: BackendType = std::env::args()
        .nth(2)
        .ok_or(anyhow::anyhow!("Wrong number of arguments (USAGE: {} <PORT> <hardware | console>)", env!("CARGO_BIN_NAME")))?
        .parse()?;

    run_server(port, backend_type)

}

#[derive(Debug)]
struct Devices {
    keyboard: Backend,
    mouse: Backend
}

impl Devices {
    fn handle_packet(&mut self, packet: PacketType) -> anyhow::Result<()> {
        match packet {
            PacketType::Keyboard(msg) => {
                self.keyboard.write(msg)?;
                Ok(())
            },
            PacketType::Mouse(mpt, msg) => {
                if matches!(mpt, MousePacketType::Movement) && matches!(self.mouse, Backend::Console(_)) {
                    return Ok(())
                }
                self.mouse.write(msg)?;
                Ok(())
            }
        }
    }
}

fn run_server(port: u16, backend_type: BackendType) -> anyhow::Result<()>{
    println!("Opening backends");
    let devices = Arc::new(Mutex::new(
        match backend_type {
            BackendType::Hardware => Devices {
                keyboard: Backend::LinuxDevice(OpenOptions::new().write(true).append(true).open("/dev/hidg0")?),
                mouse: Backend::LinuxDevice(OpenOptions::new().write(true).append(true).open("/dev/hidg1")?)
            },
            BackendType::Console => Devices {
                keyboard: Backend::Console("Keyboard"),
                mouse: Backend::Console("Mouse")
            }
        }
    ));

    let listener = TcpListener::bind(SocketAddr::new(IpAddr::from_str("0.0.0.0")?, port))?;

    println!("Listening on {}", listener.local_addr()?);

    loop {
        match listener.accept(){
            Ok((mut stream, addr)) => {
                let devices = Arc::clone(&devices);
                thread::spawn(move || {
                    println!("Got connection from {}", addr);
                    match handle_connection(&mut stream, devices.as_ref()) {
                        Ok(_) => println!("{} disconnected!", addr),
                        Err(err) => {
                            println!("{}\nDisconnecting {}!", err, addr);
                            match disconnect(&mut stream, err){
                                Ok(_) => {}
                                Err(err) => println!("{}", err)
                            }
                        }
                    }
                });
            },
            Err(e) => println!("{}", e)
        };
    }
}

fn disconnect(stream: &mut TcpStream, error: anyhow::Error) -> anyhow::Result<()>{
    stream.write_all(std::format!("{}", error).as_bytes())?;
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn read_string<'a>(stream: &mut TcpStream, data: &'a mut [u8]) -> anyhow::Result<Cow<'a, str>> {
    let read = stream.read(data)?;
    Ok(String::from_utf8_lossy(&data[0..read]))
}

fn handle_connection(stream: &mut TcpStream, devices: &Mutex<Devices>) -> anyhow::Result<()>{
    let mut data = [0 as u8; 50];
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    anyhow::ensure!(read_string(stream, &mut data)?.trim() == "Authenticate: InputShareUSB", "Handshake error");
    match devices.try_lock() {
        Ok(mut devices) => {
            stream.set_read_timeout(None)?;
            stream.write_all(b"Ok\n")?;
            loop {
                const PACKET_SIZE: usize = 9;
                let size = stream.read(&mut data[0..PACKET_SIZE])?;
                if size == 0 {
                    break;
                }
                anyhow::ensure!(size == PACKET_SIZE, "Package to small");
                let packet = PacketType::from_packet(&data[0..size]).ok_or(anyhow::anyhow!("Unknown packet type"))?;
                devices.handle_packet(packet)?;
            }
            Ok(())
        }
        Err(err) => match err {
            TryLockError::Poisoned(_) => Err(anyhow::anyhow!("Mutex poisoned. Restart the server")),
            TryLockError::WouldBlock => Err(anyhow::anyhow!("Device already in use!"))
        }
    }
}