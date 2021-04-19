use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, WH_KEYBOARD_LL, CallNextHookEx};
use winapi::um::libloaderapi::GetModuleHandleW;
use std::os::raw;
use winapi::shared::minwindef::{WPARAM, LRESULT, LPARAM};
use winapi::shared::windef::HHOOK;
use std::ptr::null;

static mut _hook: Option<HHOOK> = None;

unsafe extern "system" fn lpfn(code: raw::c_int, wParam: WPARAM, lParam: LPARAM) -> LRESULT {
    println!("{}", code);
    println!("{}", wParam);
    println!("{}", lParam);
    CallNextHookEx(_hook.unwrap(), code, wParam, lParam)
}

pub fn set_up_keyboard_hook() {
    unsafe {
        _hook = Some(SetWindowsHookExW(WH_KEYBOARD_LL, Some(lpfn), GetModuleHandleW(null()), 0));
    }
}

pub fn remove_keyboard_hook() {
    unsafe {
        UnhookWindowsHookEx(_hook.unwrap());
    }
}