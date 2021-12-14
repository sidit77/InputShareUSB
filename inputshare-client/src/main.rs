use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs, UdpSocket};
use anyhow::Result;

fn main() -> Result<()>{

    let addrs = "raspberrypi.local:12345"
        .to_socket_addrs()?
        .filter(|x| x.is_ipv4())
        .next()
        .ok_or(Error::new(ErrorKind::AddrNotAvailable, "Could not find suitable address!"))?;

    println!("rpi addrs: {}", addrs);

    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))?;
    println!("my addrs: {}", socket.local_addr()?);

    socket.send_to(&[1, 2, 3, 4, 5], &addrs)?;

    Ok(())
}