mod args;
mod devices;

use std::io::{Read, Error, ErrorKind};
use std::time::Duration;
use std::net::{TcpListener, TcpStream, Shutdown, SocketAddr, IpAddr, Ipv4Addr};
use std::thread;
use std::sync::{Mutex, TryLockError, Arc};
use inputshare_common::{PackageIds, ReadExt, WriteExt};
use crate::args::{BackendType, parse_args};
use crate::devices::Devices;

//TODO write zero into devices on error
//TODO just drop the connection

fn main() -> anyhow::Result<()>{

    let args = parse_args()?;

    run_server(args.port, args.backend)?;

    Ok(())
}

fn run_server(port: u16, backend_type: BackendType) -> std::io::Result<()>{
    println!("Opening backends");
    let devices = Arc::new(Mutex::new(Devices::from_backend_type(backend_type)?));

    let listener = TcpListener::bind(SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port))?;

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
        }
    }
}



#[derive(Debug)]
pub enum Packet<'a> {
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
                    Ok(packet) => match devices.handle_packet(packet) {
                        Ok(_) => {}
                        Err(err) => println!("Could not handle package: {}", err)
                    },
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