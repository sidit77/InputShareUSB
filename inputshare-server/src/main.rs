mod config;

use std::fs::{OpenOptions};
use std::io::{Write, Read};
use std::env;
use std::time::Duration;
use std::net::{TcpListener, TcpStream, Shutdown};


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
                                    let msg = &data[0..size];
                                    // echo everything!
                                    //stream.write(&data[0..size]).unwrap();
                                    match file.as_mut() {
                                        None => println!("Received {:?} from {:?}", &msg, &addr),
                                        Some(device) => match device.write(&msg) {
                                            Ok(_) => {},
                                            Err(e) => println!("Encountered error while write packet {:?} into file {:?}:\n{}", &msg, &device, e)
                                        }
                                    }
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