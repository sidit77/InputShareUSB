use winapi::um::winuser::{TranslateMessage, DispatchMessageW, MSG, GetMessageW, PostMessageW, WM_QUIT, PostThreadMessageW};
use std::{mem, ptr};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::shared::minwindef::DWORD;

pub struct Quitter {
    thread: DWORD
}

impl Quitter {
    pub fn from_current_thread() -> Self{
        Self {
            thread: unsafe {GetCurrentThreadId()}
        }
    }
    pub fn quit(&self){
        unsafe {PostThreadMessageW(self.thread, WM_QUIT, 0 ,0)};
    }
}

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