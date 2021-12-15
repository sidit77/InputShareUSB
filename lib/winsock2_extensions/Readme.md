# WinSock2 Extensions

This crate extends the normals Rust sockets with some windows-only functionality

**Example**
```rust
const SOCKET_READY: u32 = WM_USER + 1;
let handle = window.handle();
let socket = UdpSocket::bind("127.0.0.1:12345")?;
socket.notify(handle, SOCKET_READY, NetworkEvents::Read)?;


unsafe {
    let mut msg: MSG = mem::zeroed();
    while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
        if msg.message == SOCKET_READY {
            let mut buf = [0u8; 1000];
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((size, src)) => println!("Got {:?} from {}", &buf[..size], src),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) => Err(e)?
                }

            }
        }
    }
}
```