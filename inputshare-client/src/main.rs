mod sender;

use std::{mem};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::ffi::OsStr;
use std::io::{Error, ErrorKind};
use std::iter::once;
use std::net::{ToSocketAddrs, UdpSocket};
use std::os::windows::prelude::OsStrExt;
use std::ptr::{null, null_mut};
use std::rc::Rc;
use std::time::{Duration, Instant};
use anyhow::Result;
use udp_connections::{Client, ClientEvent, Endpoint, MAX_PACKET_SIZE};
use winapi::shared::minwindef::{FALSE};
use winapi::shared::windef::{HWND};
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0, WAIT_FAILED};
use winapi::um::winuser::{CreateWindowExW, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, DefWindowProcW, DispatchMessageW, HWND_MESSAGE, MSG, MsgWaitForMultipleObjects, PeekMessageW, PM_REMOVE, PostThreadMessageW, QS_ALLINPUT, RegisterClassW, TranslateMessage, WM_QUIT, WM_USER, WNDCLASSW};
use inputshare_common::IDENTIFIER;
use winsock2_extensions::{NetworkEvents, WinSockExt};
use yawi::{HookType, InputEvent, InputHook, KeyState, VirtualKey};
use crate::sender::InputSender;

fn main() -> Result<()>{
    {
        let thread_id = unsafe {GetCurrentThreadId()};
        ctrlc::set_handler(move || {
            unsafe {PostThreadMessageW(thread_id, WM_QUIT, 0 ,0)};
        })?;
    }

    let input_events = Rc::new(RefCell::new(InputSender::new()));

    let hook = {
        let input_events = input_events.clone();
        let mut old_mouse_pos = None;

        InputHook::new(move |event|{
            match event {
                InputEvent::MouseMoveEvent(x, y) => {
                    if let Some((ox, oy)) = old_mouse_pos {
                        (*input_events).borrow_mut().move_mouse(x - ox, y - oy);
                    }
                    old_mouse_pos = Some((x,y))
                },
                _ => {}
            }
            true
        }, true, HookType::KeyboardMouse)?
    };

    let handle = create_window("Dummy Window");//window.handle.hwnd().unwrap();//unsafe{GetConsoleWindow()};
    //let timer = unsafe {SetTimer(handle, 1, 1000, None) };
    //println!("{:?} {:?}", timer, handle);

    const SOCKET: u32 = WM_USER + 1;

    let socket = UdpSocket::bind(Endpoint::remote_any())?;
    socket.notify(handle, SOCKET, NetworkEvents::Read)?;

    let mut socket = Client::new(socket, IDENTIFIER);
    println!("Running on {}", socket.local_addr()?);
    let server = "raspberrypi.local:12345"
        .to_socket_addrs()?
        .filter(|x| x.is_ipv4())
        .next()
        .ok_or(Error::new(ErrorKind::AddrNotAvailable, "Could not find suitable address!"))?;
    println!("Connecting to {}", server);
    socket.connect(server)?;

    let mut last_send = Instant::now();
    let mut buffer = [0u8; MAX_PACKET_SIZE];
    'outer: loop {
        wait_message_timeout(Some(Duration::from_millis(100)))?;
        while let Some(msg) = get_message() {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            match msg.message {
                WM_QUIT => {
                    if socket.is_connected() {
                        socket.disconnect()?;
                    } else {
                        break 'outer
                    }

                },
                //SOCKET => {
                //    println!("FINALLY");
                //    let mut buf = [0u8; 1000];
                //    loop {
                //        match socket.recv_from(&mut buf) {
                //            Ok((size, src)) => println!("Got {:?} from {}", &buf[..size], src),
                //            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                //            Err(e) => Err(e)?
                //        }
//
                //    }
                //}
                _ => {}
            }
        }

        socket.update().unwrap();
        while let Some(event) = socket.next_event(&mut buffer).unwrap() {
            match event {
                ClientEvent::Connected(id) => {
                    println!("Connected as {}", id);
                    //input_events.borrow_mut().get_mut().insert(InputSender::new());
                },
                ClientEvent::Disconnected(reason) => {
                    println!("Disconnected: {:?}", reason);
                    //input_events.borrow_mut().get_mut().take();
                    break 'outer
                },
                ClientEvent::PacketReceived(latest, payload) => {
                    if latest {
                        (*input_events).borrow_mut().read_packet(payload)?;
                    }
                    //println!("Packet {:?}", payload);
                },
                ClientEvent::PacketAcknowledged(_) => {
                    //println!("{} got acknowledged", seq);
                }
            }
        }

        if socket.is_connected() && last_send.elapsed() >= Duration::from_millis(10) && !(*input_events).borrow_mut().in_sync(){
            socket.send((*input_events).borrow_mut().write_packet()?)?;
            last_send = Instant::now();
        }

    }

    println!("Shutdown");

    hook.remove();

    Ok(())
}

fn wait_message_timeout(timeout: Option<Duration>) -> std::io::Result<bool> {
    let timeout = match timeout {
        None => INFINITE,
        Some(duration) => duration.as_millis().try_into().expect("timout to large")
    };
    unsafe {
        match MsgWaitForMultipleObjects(0, null(), FALSE, timeout, QS_ALLINPUT) {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED => Err(std::io::Error::last_os_error()),
            _ => panic!("invalid return type")
        }
    }
}

fn get_message() -> Option<MSG> {
    unsafe {
        let mut msg: MSG = mem::zeroed();
        match PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) {
            FALSE => None,
            _ => Some(msg)
        }
    }
}

fn win32_string( value: &str ) -> Vec<u16> {
    OsStr::new( value ).encode_wide().chain( once( 0 ) ).collect()
}

fn create_window(name: &str) -> HWND {
    let name = win32_string(name);

    unsafe {
        let hinstance = GetModuleHandleW(null_mut());
        let wnd_class = WNDCLASSW {
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(DefWindowProcW),
            hInstance: hinstance,
            lpszClassName: name.as_ptr(),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null_mut(),
        };
        RegisterClassW(&wnd_class );

        let handle = CreateWindowExW(
            0,
            name.as_ptr(),
            null_mut(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            null_mut(),
            null_mut());

        handle
    }
}
