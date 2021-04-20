use laminar::{SocketEvent, Socket};
use std::thread;
use std::fs::{OpenOptions};
use std::io::{Write};
use std::env;

//const SERVER: &str = "127.0.0.1:12351";

fn main() {
    println!("Starting server!");

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

    let addr = "0.0.0.0:12351";
    let mut socket = Socket::bind(addr).unwrap();
    let (_, receiver) = (socket.get_packet_sender(), socket.get_event_receiver());
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
                _ => {}
            }
        }
    }
}
