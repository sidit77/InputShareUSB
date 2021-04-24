mod config;

use std::fs::{OpenOptions};
use std::io::{Write, Read};
use std::env;
use std::time::Duration;
use std::net::{TcpListener, TcpStream, Shutdown};

enum PacketType {
    Keyboard, Mouse
}

impl PacketType {
    fn from_id(id: u8) -> Option<Self> {
        match id {
            0x1 => Some(PacketType::Keyboard),
            0x2 => Some(PacketType::Mouse),
            _   => None
        }
    }
}

fn main(){

    println!("Starting server!");

    let cfg = config::Config::load();
    let mut file = match env::args().nth(1) {
        None => {
            println!("Using console as backend!");
            None
        },
        Some(path) => {
            println!("Writing into {}", &path);
            Some(OpenOptions::new()
                .write(true)
                .append(true)
                .open(path)
                .expect("can not open device!"))
        }
    };

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
                        let mut data = [0 as u8; 50];
                        loop {
                            match stream.read(&mut data) {
                                Ok(size) => {
                                    if size == 0 {
                                        break;
                                    }
                                    let msg = &data[1..size];

                                    match PacketType::from_id(data[0]).expect("Unknown packet type") {
                                        PacketType::Keyboard => match file.as_mut() {
                                            None => println!("Received Keyboard:{:?} from {:?}", &msg, &addr),
                                            Some(device) => match device.write(&msg) {
                                                Ok(_) => {},
                                                Err(e) => println!("Encountered error while write packet {:?} into file {:?}:\n{}", &msg, &device, e)
                                            }
                                        }
                                        PacketType::Mouse => match file.as_mut() {
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

/*
fn main() {
    println!("Starting server!");

    let cfg = config::Config::load();
    let mut file = match env::args().nth(1) {
        None => {
            println!("Using console as backend!");
            None
        },
        Some(path) => {
            println!("Writing into {}", &path);
            Some(OpenOptions::new()
                .write(true)
                .append(true)
                .open(path)
                .expect("can not open device!"))
        }
    };

    let mut socket = Socket::bind_with_config(&cfg.local_address, Config{
        heartbeat_interval: Some(Duration::from_secs(1)),
        ..Default::default()
    }).unwrap();
    let (sender, receiver) = (socket.get_packet_sender(), socket.get_event_receiver());
    println!("Running on {:?}", socket.local_addr().unwrap());
    let _thread = thread::spawn(move || socket.start_polling());

    loop {
        if let Ok(event) = receiver.recv() {
            match event {
                SocketEvent::Packet(packet) => {
                    let msg = packet.payload();

                    //if msg == b"Bye!" {
                    //    break;
                    //}

                    //let msg = String::from_utf8_lossy(msg);
                    let ip = packet.addr().ip();

                    match file.as_mut() {
                        None => println!("Received {:?} from {:?}", msg, ip),
                        Some(device) => match device.write(msg) {
                            Ok(_) => {},
                            Err(e) => println!("Encountered error while write packet {:?} into file {:?}:\n{}", msg, device, e)
                        }
                    }

                    sender.send(Packet::unreliable(packet.addr(),  "OK".as_bytes().to_vec())).expect("This should send");

                    //sender
                    //    .send(Packet::reliable_unordered(
                    //        packet.addr(),
                    //        "Copy that!".as_bytes().to_vec(),
                    //    ))
                    //    .expect("This should send");
                }
                SocketEvent::Timeout(address) => {
                    println!("Client timed out: {}", address);
                }
                SocketEvent::Connect(address) => {
                    println!("Client connected from: {}", address);
                }
                SocketEvent::Disconnect(address) => {
                    println!("Client disconnected from: {}", address);
                }
            }
        }
    }
}
*/