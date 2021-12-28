use winapi::um::winuser::{TranslateMessage, DispatchMessageW, MSG, GetMessageW, PostMessageW, WM_QUIT};
use std::{mem, ptr};

pub fn run() {
    unsafe {
        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

pub fn quit() {
    unsafe { PostMessageW(ptr::null_mut(), WM_QUIT, 0, 0) };
}