#[macro_use]
extern crate bitflags;

use std::time::Duration;
use std::net::{ToSocketAddrs, SocketAddr, TcpStream};
use std::borrow::{Borrow, Cow};
use inputshare_common::{WriteExt, ReadExt};
use std::io;
use crate::client::{WritePacket, Packet, Client};
use std::io::{stdin, Read};
use mio::{PollOpt, Ready, Poll, Token, Events};
use mio_extras::channel::{channel, Receiver, Sender};
use std::sync::mpsc::TryRecvError;
use mio::tcp::Shutdown;

mod hid;
mod config;
mod client;

const CLIENT: Token = Token(0);
const QUIT: Token = Token(1);
const SERVER: Token = Token(2);

fn main() -> anyhow::Result<()>{

    println!("Starting client");
    let cfg = config::Config::load().unwrap();

    println!("Resolving {}", cfg.merged_address());
    let address = resolve_address(cfg.merged_address())?;

    println!("Connecting to {}", address);
    let mut server = TcpStream::connect(&address)?;
    do_handshake(&mut server)?;

    println!("Successfully connected to server");

    let quit_signal = get_quit_signals();
    let client = Client::start(cfg.hotkey, &cfg.backlist)?;
    let mut server = mio::net::TcpStream::from_stream(server)?;
    //server.set_nodelay(true)?;

    let poll = Poll::new()?;
    poll.register(&client, CLIENT, Ready::readable(), PollOpt::edge())?;
    poll.register(&quit_signal, QUIT, Ready::readable(), PollOpt::edge())?;
    poll.register(&server, SERVER, Ready::readable(), PollOpt::edge())?;

    let mut events = Events::with_capacity(1024);
    loop {
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            match event.token() {
                CLIENT => loop {
                    match client.try_recv() {
                        Ok(packet) => {
                            if let Packet::SwitchDevice(side) = &packet {
                                println!("Switching sides: {:?}", side)
                            }
                            server.write_packet(packet)?
                        },
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => return Err(anyhow::anyhow!("The client stopped working"))
                    }
                }
                QUIT => {
                    println!("Quitting");
                    server.shutdown(Shutdown::Both)?;
                    return Ok(())
                },
                SERVER => {
                    let mut buf = [0u8; 100];
                    loop {
                        match server.try_read_string(&mut buf[..]){
                            Ok(str) => println!("SERVER: {}", str),
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                            Err(e) => return Err(anyhow::Error::from(e))
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }

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

struct Quit;

fn get_quit_signals() -> Receiver<Quit>{
    let (tx, rx): (Sender<Quit>, Receiver<Quit>) = channel();

    {
        let tx_t = tx.clone();
        ctrlc::set_handler(move ||{
            tx_t.send(Quit).unwrap()
        }).expect("Cant set ctrl c handler!");
    }

    {
        let tx_t = tx.clone();
        std::thread::spawn(move || {
            let mut s = String::new();
            loop {
                stdin().read_line(&mut s).expect("Cant read stdin!");
                if s.trim().eq("stop") {
                    tx_t.send(Quit).unwrap()
                }
            }
        });
    }

    rx
}

trait NonBlockingReadExt : Read {
    fn peek(&self, buf: &mut [u8]) -> std::io::Result<usize>;

    fn peek_u16(&self) -> std::io::Result<u16> {
        let mut buf = [0; 2];
        let size = self.peek(&mut buf)?;
        if size != buf.len() {
            Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "Can not peek this far"))
        } else {
            Ok(u16::from_be_bytes(buf))
        }
    }

    fn try_read_string<'a>(&mut self, buf: &'a mut [u8]) -> std::io::Result<Cow<'a, str>> {
        let size = self.peek_u16()? as usize;
        self.read_exact(&mut buf[0..(size+2)])?;
        Ok(String::from_utf8_lossy(&buf[2..(size+2)]))
    }
}

impl NonBlockingReadExt for mio::net::TcpStream {
    fn peek(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.peek(buf)
    }
}