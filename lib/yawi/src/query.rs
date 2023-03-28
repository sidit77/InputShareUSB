use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

pub fn get_cursor_pos() -> (i32, i32) {
    unsafe {
        let mut pt = POINT::default();
        GetCursorPos(&mut pt).expect("Failed to get current cursor pos");
        (pt.x, pt.y)
    }
}
