mod receiver;

use std::fmt::{Debug, Formatter};
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::net::{SocketAddr};
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use mio::net::UdpSocket;
use mio_signals::{Signal, Signals, SignalSet};
use udp_connections::{Endpoint, MAX_PACKET_SIZE, Server, ServerEvent, Transport};
use inputshare_common::IDENTIFIER;
use crate::receiver::{InputEvent, InputReceiver};

fn main() -> Result<()>{
    println!("Hello World!");
    //let mut mouse_dev = OpenOptions::new().write(true).append(true).open("/dev/hidg1")?;


    const SERVER: Token = Token(0);
    const SIGNAL: Token = Token(1);
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    let mut signals = Signals::new(SignalSet::all())?;
    poll.registry().register(&mut signals, SIGNAL, Interest::READABLE)?;

    let mut socket = UdpSocket::bind(Endpoint::remote_port(12345))?;
    poll.registry().register(&mut socket, SERVER, Interest::READABLE)?;
    let mut socket = Server::new(MioSocket::from(socket), IDENTIFIER, 1);

    println!("running on {}", socket.local_addr()?);

    let mut receiver = InputReceiver::new();

    let mut buffer = [0u8; MAX_PACKET_SIZE];
    'outer: loop {
        poll.poll(&mut events, Some(Duration::from_secs(1)))?;


        for event in events.iter() {
            match event.token() {
                SIGNAL => loop {
                    match signals.receive()? {
                        Some(Signal::Interrupt) => break 'outer,
                        Some(Signal::Quit) => break 'outer,
                        Some(Signal::Terminate) => break 'outer,
                        Some(_) => continue,
                        None => break
                    }
                }
                _ => {}
            }
        }

        socket.update()?;
        while let Some(event) = socket.next_event(&mut buffer).unwrap() {
            match event {
                ServerEvent::ClientConnected(client_id) => {
                    println!("Client {} connected", client_id);
                },
                ServerEvent::ClientDisconnected(client_id, reason) => {
                    println!("Client {} disconnected: {:?}", client_id, reason);
                },
                ServerEvent::PacketReceived(client_id, latest, payload) => {
                    if latest {
                        socket.send(client_id, receiver.process_packet(payload)?)?;
                        while let Some(event) = receiver.get_event() {
                            match event {
                                InputEvent::MouseMove(dx, dy) => {
                                    //let mut report = [0u8; 7];
                                    //let mut buf = &mut report[..];
                                    //buf.write_u8(0)?;
                                    //buf.write_i16::<LittleEndian>(dx as i16)?;
                                    //buf.write_i16::<LittleEndian>(dy as i16)?;
                                    //buf.write_i8(0)?;
                                    //buf.write_i8(0)?;
                                    //mouse_dev.write_all(&report)?;
                                    println!("{} {}", dx, dy);
                                }
                            }
                            //println!("{:?}", event);

                        }
                    }
                },
                ServerEvent::PacketAcknowledged(_, _) => {
                    //println!("Packet {} acknowledged for {}", seq, client_id)
                }
            }
        }

    }

    socket.disconnect_all()?;
    println!("Shutting down");

    Ok(())

}

struct MioSocket(UdpSocket);

impl Debug for MioSocket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Transport for MioSocket {
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.0.send_to(buf, addr)
    }

    fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl From<UdpSocket> for MioSocket {
    fn from(socket: UdpSocket) -> Self {
        Self(socket)
    }
}