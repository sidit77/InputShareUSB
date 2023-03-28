use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn run() {
    unsafe {
        let mut msg: MSG = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

pub fn quit() {
    unsafe { PostMessageW(HWND::default(), WM_QUIT, WPARAM::default(), LPARAM::default()) };
}
