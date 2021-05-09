use std::fs::{OpenOptions, File};
use std::io::{Write, Read, stdout, Error, ErrorKind};
use std::time::Duration;
use std::net::{TcpListener, TcpStream, Shutdown, SocketAddr, IpAddr};
use std::str::FromStr;
use std::thread;
use std::sync::{Mutex, TryLockError, Arc};
use inputshare_common::{PackageIds, ReadExt, WriteExt};

//TODO write zero into devices on error

#[derive(Debug)]
enum Packet<'a> {
    Keyboard(&'a[u8]), Mouse(&'a[u8])
}

trait ReadPacket: Read {
    fn read_packet<'a>(&mut self, buf: &'a mut [u8]) -> std::io::Result<Packet<'a>> {
        self.read_exact(&mut buf[0..1])?;
        match buf[0] {
            PackageIds::KEYBOARD => {
                let msg = &mut buf[0..8];
                self.read_exact(msg)?;
                Ok(Packet::Keyboard(msg))
            }
            PackageIds::MOUSE => {
                let msg = &mut buf[0..7];
                self.read_exact(msg)?;
                Ok(Packet::Mouse(msg))
            }
            _ => Err(Error::new(ErrorKind::InvalidData, "Unknown package identifier!"))
        }
    }
}

impl ReadPacket for TcpStream {}

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
    fn handle_packet(&mut self, packet: Packet) -> anyhow::Result<()> {
        match packet {
            Packet::Keyboard(msg) => {
                self.keyboard.write(msg)?;
                Ok(())
            },
            Packet::Mouse(msg) => {
                if has_mouse_movement(msg) && matches!(self.mouse, Backend::Console(_)) {
                    return Ok(())
                }
                self.mouse.write(msg)?;
                Ok(())
            }
        }
    }
}

fn has_mouse_movement(msg: &[u8]) -> bool {
    msg[1..4].iter().any(|x|*x !=0 )
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

fn disconnect(stream: &mut TcpStream, error: anyhow::Error) -> std::io::Result<()>{
    stream.write_string(std::format!("{}", error).as_str())?;
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn handle_connection(stream: &mut TcpStream, devices: &Mutex<Devices>) -> anyhow::Result<()>{
    let mut data = [0 as u8; 50];
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    anyhow::ensure!(stream.read_string(&mut data)? == "Authenticate: InputShareUSB", "Handshake error");
    match devices.try_lock() {
        Ok(mut devices) => {
            stream.set_read_timeout(None)?;
            stream.write_string("Ok")?;
            loop {
                match stream.read_packet(&mut data[..]){
                    Ok(packet) => devices.handle_packet(packet)?,
                    Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => Err(e)?
                }
            }
            Ok(())
        }
        Err(err) => match err {
            TryLockError::Poisoned(_) => Err(anyhow::anyhow!("Mutex poisoned. Restart the server")),
            TryLockError::WouldBlock => Err(anyhow::anyhow!("Device already in use!"))
        }
    }
}