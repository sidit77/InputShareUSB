use laminar::{SocketEvent, Socket, Packet};
use std::thread;
use std::fs::OpenOptions;
use std::io::Write;

//const SERVER: &str = "127.0.0.1:12351";

fn main() {
    println!("Hello server!");

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("/dev/hidg0")
        .expect("can not open device!");

    let addr = "0.0.0.0:12351";
    let mut socket = Socket::bind(addr).unwrap();
    let (sender, receiver) = (socket.get_packet_sender(), socket.get_event_receiver());
    println!("running on {:?}", socket.local_addr().unwrap());
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

                    println!("Received {:?} from {:?}", &msg, ip);

                    file.write(&msg);

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
