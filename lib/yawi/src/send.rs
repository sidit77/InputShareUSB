use std::iter::once;
use std::mem::size_of;
use windows::core::Error;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::{KeyState, Input, VirtualKey, ScrollDirection, WinResult};

fn add_to_vec(vec: &mut Vec<INPUT>, input: Input) {
    match input {
        Input::KeyboardKeyInput(key, state) =>  {
            vec.push(create_keyboard_input(KEYBDINPUT{
                wVk: key.into(),
                wScan: 0,
                dwFlags: match state {
                    KeyState::Pressed => Default::default(),
                    KeyState::Released => KEYEVENTF_KEYUP
                },
                time: 0,
                dwExtraInfo: 0
            }));
        }
        Input::StringInput(string) => {
            for c in string.encode_utf16() {
                vec.push(create_keyboard_input(KEYBDINPUT{
                    wVk: Default::default(),
                    wScan: c,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0
                }));
                vec.push(create_keyboard_input(KEYBDINPUT{
                    wVk: Default::default(),
                    wScan: c,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0
                }));
            }
        }
        Input::MouseButtonInput(key, state) => {
            vec.push(create_mouse_input(MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: match key {
                    VirtualKey::XButton1 => XBUTTON1 as i32,
                    VirtualKey::XButton2 => XBUTTON2 as i32,
                    _ => 0
                },
                dwFlags: match (key, state) {
                    (VirtualKey::LButton , KeyState::Pressed ) => MOUSEEVENTF_LEFTDOWN,
                    (VirtualKey::LButton , KeyState::Released) => MOUSEEVENTF_LEFTUP,
                    (VirtualKey::RButton , KeyState::Pressed ) => MOUSEEVENTF_RIGHTDOWN,
                    (VirtualKey::RButton , KeyState::Released) => MOUSEEVENTF_RIGHTUP,
                    (VirtualKey::MButton , KeyState::Pressed ) => MOUSEEVENTF_MIDDLEDOWN,
                    (VirtualKey::MButton , KeyState::Released) => MOUSEEVENTF_MIDDLEUP,
                    (VirtualKey::XButton1, KeyState::Pressed ) => MOUSEEVENTF_XDOWN,
                    (VirtualKey::XButton1, KeyState::Released) => MOUSEEVENTF_XUP,
                    (VirtualKey::XButton2, KeyState::Pressed ) => MOUSEEVENTF_XDOWN,
                    (VirtualKey::XButton2, KeyState::Released) => MOUSEEVENTF_XUP,
                    _ => {tracing::warn!("Unsupported key ({:?}): Skipping!", key); MOUSEEVENTF_WHEEL}
                },
                time: 0,
                dwExtraInfo: 0
            }))
        }
        Input::MouseScrollInput(dir) => {
            vec.push(create_mouse_input(MOUSEINPUT{
                dx: 0,
                dy: 0,
                mouseData: (WHEEL_DELTA as f32 * match dir {
                    ScrollDirection::Horizontal(x) => x,
                    ScrollDirection::Vertical(x) => x
                }) as i32,
                dwFlags: match dir {
                    ScrollDirection::Horizontal(_) => MOUSEEVENTF_HWHEEL,
                    ScrollDirection::Vertical(_) => MOUSEEVENTF_WHEEL
                },
                time: 0,
                dwExtraInfo: 0
            }));
        }
        Input::RelativeMouseMoveInput(dx, dy) => {
            vec.push(create_mouse_input(MOUSEINPUT{
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0
            }))
        }
        Input::AbsoluteMouseMoveInput(x, y) => {
            vec.push(create_mouse_input(MOUSEINPUT{
                dx: x * 65536 / system_metric(SM_CXSCREEN),
                dy: y * 65536 / system_metric(SM_CYSCREEN),
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                time: 0,
                dwExtraInfo: 0
            }))
        }
    }
}

fn system_metric(index: SYSTEM_METRICS_INDEX) -> i32{
    unsafe {
        GetSystemMetrics(index)
    }
}

fn create_mouse_input(ms: MOUSEINPUT) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: ms
        }
    }
}

fn create_keyboard_input(kb: KEYBDINPUT) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: kb
        }
    }
}

/// Send multiple input events to windows
///
/// This function is a wrapper around
/// [SendInput](https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
///
/// Return Ok if the number of send inputs match the number of supplied inputs
pub fn send_inputs<'a>(inputs: impl IntoIterator<Item=Input<'a>>) -> WinResult<()>{
    let mut vec = Vec::new();
    for input in inputs {
        add_to_vec(&mut vec, input);
    }
    let c = unsafe {
        SendInput(&vec, size_of::<INPUT>() as i32)
    };
    match vec.len() == c as usize {
        true => Ok(()),
        false => Err(Error::from_win32())
    }
}

/// Convenience function to send a single input
///
/// See `send_inputs` for more info
pub fn send_input(input: Input) -> WinResult<()> {
    send_inputs(once(input))
}