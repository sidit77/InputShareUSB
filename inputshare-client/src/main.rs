use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{FromRawHandle, IntoRawHandle};
use std::time::{Duration, Instant};
use anyhow::Result;
use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;


fn main() -> Result<()>{

    let addrs = "raspberrypi.local:12345"
        .to_socket_addrs()?
        .filter(|x| x.is_ipv4())
        .next()
        .ok_or(Error::new(ErrorKind::AddrNotAvailable, "Could not find suitable address!"))?;
    println!("rpi addrs: {}", addrs);

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))?;
    println!("my addrs: {}", socket.local_addr()?);

    const SOCKET: Token = Token(0);
    poll.registry().register(&mut socket, SOCKET, Interest::READABLE)?;
    loop {
        poll.poll(&mut events, Some(Duration::from_secs(1)))?;
        events.clear();

        println!("TICK");

        socket.send_to(&[1, 2, 3, 4, 5], addrs)?;
    }



//
    println!("rpi addrs: {}", addrs);
//
    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))?;
    println!("my addrs: {}", socket.local_addr()?);
//


    Ok(())
}