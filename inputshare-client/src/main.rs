#[macro_use]
extern crate bitflags;

use std::time::Duration;
use std::net::{ToSocketAddrs, SocketAddr, TcpStream};
use std::borrow::Borrow;
use inputshare_common::{WriteExt, ReadExt};
use std::io;
use crate::client::run_client;

mod hid;
mod config;
mod client;

fn main() -> anyhow::Result<()>{
    println!("Starting client");
    let cfg = config::Config::load().unwrap();

    println!("Resolving {}", cfg.merged_address());

    let address = resolve_address(cfg.merged_address())?;

    println!("Connecting to {}", address);

    let mut server = TcpStream::connect(address)?;

    do_handshake(&mut server)?;

    println!("Successfully connected to server");

    run_client(&mut server, cfg.hotkey, &cfg.backlist)
}

fn resolve_address(address: &str) -> io::Result<SocketAddr> {
    Ok(address
        .to_socket_addrs()?
        .filter(|x|match x {
            SocketAddr::V4(_) => true,
            SocketAddr::V6(_) => false
        })
        .next()
        .ok_or(io::Error::new(io::ErrorKind::AddrNotAvailable, "Could not find suitable address!"))?)
}

fn do_handshake(stream: &mut TcpStream) -> io::Result<()> {
    let mut data = [0 as u8; 50];
    stream.write_string("Authenticate: InputShareUSB")?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    match stream.read_string(&mut data)?.borrow() {
        "Ok" => Ok(()),
        s => Err(io::Error::new(io::ErrorKind::InvalidInput, s))
    }?;
    stream.set_read_timeout(None)
}
