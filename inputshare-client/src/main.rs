mod gui;

use std::ptr::null;
use std::os::raw;
use winapi::um::winuser::{SetWindowsHookExW, CallNextHookEx, WH_KEYBOARD_LL, MSG, TranslateMessage, DispatchMessageW, GetMessageW, UnhookWindowsHookEx};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::{HHOOK, POINT, HWND};
use winapi::shared::minwindef::{WPARAM, LPARAM, LRESULT, UINT, DWORD};
use std::rc::Rc;
use crate::gui::SystemTray;


const SERVER: &str = "127.0.0.1:12351";

static mut _hook: Option<HHOOK> = None;

unsafe extern "system" fn lpfn(code: raw::c_int, wParam: WPARAM, lParam: LPARAM) -> LRESULT {
    println!("{}", code);
    println!("{}", wParam);
    println!("{}", lParam);
    CallNextHookEx(_hook.unwrap(), code, wParam, lParam)
}

fn set_up_keyboard_hook() {
    unsafe {
        _hook = Some(SetWindowsHookExW(WH_KEYBOARD_LL, Some(lpfn), GetModuleHandleW(null()), 0));
    }
}


fn main() {
    println!("Hello client!");

    unsafe {
        set_up_keyboard_hook();
    }

    nwg::init().expect("Failed to init Native Windows GUI");
    let _ui = SystemTray::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();

    unsafe {
        UnhookWindowsHookEx(_hook.unwrap());
    }

/*
    let addr = "127.0.0.1:12352";
    let mut socket = Socket::bind(addr).unwrap();
    println!("Connected on {}", addr);

    let server = SERVER.parse().unwrap();

    println!("Type a message and press Enter to send. Send `Bye!` to quit.");

    let stdin = stdin();
    let mut s_buffer = String::new();

    loop {
        s_buffer.clear();
        stdin.read_line(&mut s_buffer).unwrap();
        let line = s_buffer.replace(|x| x == '\n' || x == '\r', "");

        socket.send(Packet::reliable_unordered(
            server,
            line.clone().into_bytes(),
        )).unwrap();

        socket.manual_poll(Instant::now());

        if line == "Bye!" {
            break;
        }

        match socket.recv() {
            Some(SocketEvent::Packet(packet)) => {
                if packet.addr() == server {
                    println!("Server sent: {}", String::from_utf8_lossy(packet.payload()));
                } else {
                    println!("Unknown sender.");
                }
            }
            Some(SocketEvent::Timeout(_)) => {}
            _ => println!("Silence.."),
        }
    }

    */
}
