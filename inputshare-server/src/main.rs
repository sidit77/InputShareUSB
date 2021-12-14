use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use anyhow::Result;
use mio::net::UdpSocket;
use mio_signals::{Signal, Signals, SignalSet};
use mio_timerfd::{ClockId, TimerFd};

fn main() -> Result<()>{
    println!("Hello World!");


    let mut poll = Poll::new()?;

    let mut events = Events::with_capacity(128);

    let mut socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 12345))?;
    println!("running on {}", socket.local_addr()?);

    let mut signals = Signals::new(SignalSet::all())?;

    let mut timer = TimerFd::new(ClockId::Monotonic)?;
    timer.set_timeout(&Duration::from_secs(1))?;

    const SERVER: Token = Token(0);
    const HEARTBEAT: Token = Token(1);
    const SIGNAL: Token = Token(2);
    poll.registry().register(&mut socket, SERVER, Interest::READABLE)?;
    poll.registry().register(&mut timer, HEARTBEAT, Interest::READABLE)?;
    poll.registry().register(&mut signals, SIGNAL, Interest::READABLE)?;
    'outer: loop {
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
                },
                HEARTBEAT => {
                    println!("Tick!");
                    timer.set_timeout(&Duration::from_millis(1000))?;
                },
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
    }

    println!("Shutting down");

    Ok(())

}