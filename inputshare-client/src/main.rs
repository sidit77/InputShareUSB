use std::{mem};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::iter::once;
use std::net::UdpSocket;
use std::os::windows::prelude::OsStrExt;
use std::ptr::{null, null_mut};
use std::time::Duration;
use anyhow::Result;
use winapi::shared::minwindef::{FALSE};
use winapi::shared::windef::{HWND};
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0, WAIT_FAILED};
use winapi::um::winuser::{CreateWindowExW, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, DefWindowProcW, DispatchMessageW, HWND_MESSAGE, MSG, MsgWaitForMultipleObjects, PeekMessageW, PM_REMOVE, PostThreadMessageW, QS_ALLINPUT, RegisterClassW, TranslateMessage, WM_QUIT, WM_USER, WNDCLASSW};
use winsock2_extensions::{NetworkEvents, WinSockExt};
use yawi::{HookType, InputHook};

fn main() -> Result<()>{
    {
        let thread_id = unsafe {GetCurrentThreadId()};
        ctrlc::set_handler(move || {
            unsafe {PostThreadMessageW(thread_id, WM_QUIT, 0 ,0)};
        })?;
    }

    let mut input_events = RefCell::new(VecDeque::new());

    let hook = InputHook::new(|event|{
            input_events.borrow_mut().push_back(event);
            true
        }, true, HookType::Keyboard)?;

    let handle = create_window("Dummy Window");//window.handle.hwnd().unwrap();//unsafe{GetConsoleWindow()};
    //let timer = unsafe {SetTimer(handle, 1, 1000, None) };
    //println!("{:?} {:?}", timer, handle);

    const SOCKET: u32 = WM_USER + 1;

    let socket = UdpSocket::bind("127.0.0.1:12345")?;
    socket.notify(handle, SOCKET, NetworkEvents::Read)?;


    'outer: loop {
        wait_message_timeout(None)?;
        while let Some(msg) = get_message() {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            match msg.message {
                WM_QUIT => break 'outer,
                SOCKET => {
                    println!("FINALLY");
                    let mut buf = [0u8; 1000];
                    loop {
                        match socket.recv_from(&mut buf) {
                            Ok((size, src)) => println!("Got {:?} from {}", &buf[..size], src),
                            Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                            Err(e) => Err(e)?
                        }

                    }
                }
                _ => {}
            }
        }
        while let Some(event) = input_events.borrow_mut().pop_front() {
            println!("{:?}", event);
        }
        println!("Tick");
    }

//
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
