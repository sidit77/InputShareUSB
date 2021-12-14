use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use mio::{Events, Interest, Poll, Token};
use anyhow::Result;
use mio::net::UdpSocket;

fn main() -> Result<()>{
    println!("Hello World!");


    let mut poll = Poll::new()?;

    let mut events = Events::with_capacity(128);

    let mut socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 12345))?;

    println!("running on {}", socket.local_addr()?);

    const SERVER: Token = Token(0);
    poll.registry().register(&mut socket, SERVER, Interest::READABLE)?;

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    let mut buffer = [0u8; 1500];
                    loop {
                        match socket.recv_from(&mut buffer) {
                            Ok((size, src)) => println!("Got {:?} from {}", &buffer[..size], src),
                            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                            Err(e) => Err(e)?
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /*
    let mut socket = UdpSocket::bind("0.0.0.0:12345")?;

    println!("running on {}", socket.local_addr()?);

    let mut buffer = [0u8; 1500];
    loop {
        loop {
            match socket.recv_from(&mut buffer) {
                Ok((size, src)) => println!("Got {:?} from {}", &buffer[..size], src),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => Err(e)?
            }
        }
    }

     */
}