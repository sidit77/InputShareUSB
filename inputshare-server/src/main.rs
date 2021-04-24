mod config;

use std::fs::{OpenOptions};
use std::io::{Write, Read};
use std::time::Duration;
use std::net::{TcpListener, TcpStream, Shutdown};

enum PacketType<'a> {
    Keyboard(&'a[u8]), Mouse(&'a[u8])
}

impl<'a> PacketType<'a> {
    fn from_packet(packet: &'a[u8]) -> Option<Self> {
        match packet[0] {
            0x1 => Some(PacketType::Keyboard(&packet[1..9])),
            0x2 => Some(PacketType::Mouse(&packet[1..8])),
            _   => None
        }
    }
}

fn main(){

    println!("Starting server!");

    let cfg = config::Config::load();

    let mut kbfile = OpenOptions::new()
        .write(true)
        .append(true)
        .open("/dev/hidg0")
        .ok();

    let mut msfile = OpenOptions::new()
        .write(true)
        .append(true)
        .open("/dev/hidg1")
        .ok();

    if kbfile.is_none(){
        println!("Writing keyboard into console!");
    }

    if msfile.is_none(){
        println!("Writing mouse into console!");
    }

    //run(Transport::FramedTcp, "0.0.0.0:12345".to_socket_addrs().unwrap().next().unwrap())
    let listener = TcpListener::bind(&cfg.local_address).unwrap();
    println!("Server listens on {}", listener.local_addr().unwrap());

    loop {
        println!("Waiting for incoming connections");
        match listener.accept(){
            Ok((mut stream, addr)) => {
                println!("Got connection from {}", addr);

                match do_handshake(&mut stream) {
                    Ok(_) => {
                        stream.set_read_timeout(None).unwrap();
                        let mut data = [0 as u8; 9];
                        loop {
                            match stream.read(&mut data) {
                                Ok(size) => {
                                    if size == 0 {
                                        break;
                                    }

                                    match PacketType::from_packet(&data).expect("Unknown packet type") {
                                        PacketType::Keyboard(msg) => match kbfile.as_mut() {
                                            None => println!("Received Keyboard:{:?} from {:?}", &msg, &addr),
                                            Some(device) => match device.write(&msg) {
                                                Ok(_) => {},
                                                Err(e) => println!("Encountered error while write packet {:?} into file {:?}:\n{}", &msg, &device, e)
                                            }
                                        }
                                        PacketType::Mouse(msg) => match msfile.as_mut() {
                                            None => println!("Received Mouse:{:?} from {:?}", &msg, &addr),
                                            Some(device) => match device.write(&msg) {
                                                Ok(_) => {},
                                                Err(e) => println!("Encountered error while write packet {:?} into file {:?}:\n{}", &msg, &device, e)
                                            }
                                        }
                                    }

                                    // echo everything!
                                    //stream.write(&data[0..size]).unwrap();

                                },
                                Err(_) => {
                                    println!("An error occurred, terminating connection with {}", stream.peer_addr().unwrap());
                                    stream.shutdown(Shutdown::Both).unwrap();
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        println!("An handshake error occurred, terminating connection with {}", stream.peer_addr().unwrap());
                        stream.shutdown(Shutdown::Both).unwrap();
                    }
                }

            }
            Err(e) => println!("An error occurred!\n{}", e)
        }
    }

}

fn do_handshake(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut data = [0 as u8; 50];
    //let mut buffer = String::new();
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"Authenticate\n")?;
    let buffer = read_string(stream, &mut data)?;
    println!("Got: {}", buffer.trim());
    if buffer.trim() != "secretPassword" {
        anyhow::bail!("Wrong password!");
    }
    stream.write_all(b"Ok\n")?;
    Ok(())
}

fn read_string(stream: &mut TcpStream, data: &mut [u8]) -> anyhow::Result<String> {
    let read = stream.read(data)?;
    Ok(String::from_utf8_lossy(&data[0..read]).to_string())
}