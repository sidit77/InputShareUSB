mod error;
mod enums;

use std::{mem, ptr};
use std::convert::TryInto;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::iter::once;
use std::net::UdpSocket;
use std::os::raw;
use std::os::windows::prelude::OsStrExt;
use std::ptr::{null, null_mut};
use std::time::Duration;
use anyhow::Result;
use winapi::shared::minwindef::{FALSE, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HHOOK, HWND};
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0, WAIT_FAILED};
use winapi::um::winuser::{CallNextHookEx, CreateWindowExW, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, DefWindowProcW, DispatchMessageW, HWND_MESSAGE, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, MapVirtualKeyW, MAPVK_VK_TO_VSC_EX, MSG, MsgWaitForMultipleObjects, PeekMessageW, PM_REMOVE, PostMessageW, PostThreadMessageW, QS_ALLINPUT, RegisterClassW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, VK_NUMLOCK, VK_PAUSE, VK_SCROLL, VK_SNAPSHOT, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_USER, WNDCLASSW};
use winsock2_extensions::{NetworkEvents, WinSockExt};
use crate::enums::{InputEvent, KeyState, VirtualKey, WindowsScanCode};


fn main() -> Result<()>{
    {
        let thread_id = unsafe {GetCurrentThreadId()};
        ctrlc::set_handler(move || {
            unsafe {PostThreadMessageW(thread_id, WM_QUIT, 0 ,0)};
        })?;
    }

    unsafe {
        let handle = check(GetModuleHandleW(null()))?;
        let keyboard = check(SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), handle, 0))?;
        HOOK = Some(keyboard)
    }

    let handle = create_window("Dummy Window");//window.handle.hwnd().unwrap();//unsafe{GetConsoleWindow()};
    //let timer = unsafe {SetTimer(handle, 1, 1000, None) };
    //println!("{:?} {:?}", timer, handle);

    const SOCKET: u32 = WM_USER + 1;

    let socket = UdpSocket::bind("127.0.0.1:12345")?;
    socket.notify(handle, SOCKET, NetworkEvents::Read)?;


    'outer: loop {
        wait_message_timeout(Some(Duration::from_secs(1)))?;
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
        println!("Tick");
    }

//
    println!("Shutdown");
    unsafe {
        if let Some(hook) = HOOK{
            UnhookWindowsHookEx(hook);
        }
    }

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

fn check<T>(ptr: *mut T) -> error::Result<*mut T>{
    if ptr.is_null() {
        Err(error::Error::last())
    } else {
        Ok(ptr)
    }
}

const IGNORE: usize = 0x1234567;

static mut HOOK: Option<HHOOK> = None;

unsafe extern "system" fn low_level_keyboard_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);

        if key_struct.dwExtraInfo != IGNORE {

            let event = match parse_virtual_key(&key_struct) {
                Some(key) => match parse_key_state(wparam) {
                    Some(state) => Some(InputEvent::KeyboardKeyEvent(key, parse_scancode(&key_struct), state)),
                    None => {println!("Unknown event: {}", wparam); None}
                }
                None => {println!("Unknown key: {}", key_struct.vkCode); None}
            };

            if let Some(event) = event {
                println!("{:?}", event);
                PostMessageW(ptr::null_mut(), WM_USER, 0, 0);
            }

        }

    }
    CallNextHookEx(HOOK.unwrap(), code, wparam, lparam)
}

fn parse_scancode(key_struct: &KBDLLHOOKSTRUCT) -> WindowsScanCode {
    let mut scancode = key_struct.scanCode as WindowsScanCode;
    let vk = key_struct.vkCode as i32;
    if scancode == 0x0 || vk == VK_SNAPSHOT || vk == VK_SCROLL || vk == VK_PAUSE || vk == VK_NUMLOCK {
        scancode = unsafe {MapVirtualKeyW(key_struct.vkCode, MAPVK_VK_TO_VSC_EX)} as WindowsScanCode;
    } else {
        if key_struct.flags & LLKHF_EXTENDED == LLKHF_EXTENDED {
            scancode |= 0xe000;
        }
    }
    scancode
}

fn parse_virtual_key(key_struct: &KBDLLHOOKSTRUCT) -> Option<VirtualKey> {
    key_struct.vkCode.try_into().ok()
}

fn parse_key_state(wparam: WPARAM) -> Option<KeyState> {
    match wparam as UINT {
        WM_KEYDOWN    => Some(KeyState::Pressed),
        WM_SYSKEYDOWN => Some(KeyState::Pressed),
        WM_KEYUP      => Some(KeyState::Released),
        WM_SYSKEYUP   => Some(KeyState::Released),
        _ => None
    }
}
