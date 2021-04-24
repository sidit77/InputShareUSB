use winapi::um::winuser::{INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEINPUT, INPUT_MOUSE, XBUTTON1, XBUTTON2, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_WHEEL, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, WHEEL_DELTA, MOUSEEVENTF_MOVE};
use crate::keys::{VirtualKey, KeyState, ScrollDirection};
use std::mem;

const IGNORE: usize = 0x1234567;

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum Input<'a> {
    KeyboardKeyInput(VirtualKey, KeyState),
    StringInput(&'a str),
    MouseButtonInput(VirtualKey, KeyState),
    MouseScrollInput(ScrollDirection),
    RelativeMouseMoveInput(i32, i32)
}

trait AddInputs{
    fn add(&mut self, input: &Input);
}

impl AddInputs for Vec<INPUT> {
    fn add(&mut self, input: &Input) {
        match input {
            Input::KeyboardKeyInput(key, state) =>  {
                self.push(create_keyboard_input(KEYBDINPUT{
                    wVk: key_to_u16(key),
                    wScan: 0,
                    dwFlags: match state {
                        KeyState::Pressed => 0,
                        KeyState::Released => KEYEVENTF_KEYUP
                    },
                    time: 0,
                    dwExtraInfo: IGNORE
                }));
            }
            Input::StringInput(string) => {
                for c in string.encode_utf16() {
                    self.push(create_keyboard_input(KEYBDINPUT{
                        wVk: 0,
                        wScan: c,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: IGNORE
                    }));
                    self.push(create_keyboard_input(KEYBDINPUT{
                        wVk: 0,
                        wScan: c,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: IGNORE
                    }));
                }
            }
            Input::MouseButtonInput(key, state) => {
                self.push(create_mouse_input(MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: match key {
                        VirtualKey::XButton1 => XBUTTON1 as u32,
                        VirtualKey::XButton2 => XBUTTON2 as u32,
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
                        _ => {println!("Unsupported key ({:?}): Skipping!", key); MOUSEEVENTF_WHEEL}
                    },
                    time: 0,
                    dwExtraInfo: IGNORE
                }))
            }
            Input::MouseScrollInput(dir) => {
                self.push(create_mouse_input(MOUSEINPUT{
                    dx: 0,
                    dy: 0,
                    mouseData: (WHEEL_DELTA as f32 * match dir {
                        ScrollDirection::Horizontal(x) => x,
                        ScrollDirection::Vertical(x) => x
                    }) as i32 as u32,
                    dwFlags: match dir {
                        ScrollDirection::Horizontal(_) => MOUSEEVENTF_HWHEEL,
                        ScrollDirection::Vertical(_) => MOUSEEVENTF_WHEEL
                    },
                    time: 0,
                    dwExtraInfo: IGNORE
                }));
            }
            Input::RelativeMouseMoveInput(dx, dy) => {
                self.push(create_mouse_input(MOUSEINPUT{
                    dx: *dx,
                    dy: *dy,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: IGNORE
                }))
            }
        }
    }
}

fn create_mouse_input(ms: MOUSEINPUT) -> INPUT {
    unsafe {
        let mut input = INPUT {
            type_: INPUT_MOUSE,
            u: std::mem::zeroed()
        };
        *input.u.mi_mut() = ms;
        input
    }
}

fn create_keyboard_input(kb: KEYBDINPUT) -> INPUT {
    unsafe {
        let mut input = INPUT {
            type_: INPUT_KEYBOARD,
            u: std::mem::zeroed()
        };
        *input.u.ki_mut() = kb;
        input
    }
}

pub fn send_keys<'a>(inputs: impl Iterator<Item=&'a Input<'a>>) -> anyhow::Result<()>{
    let mut ia: Vec<INPUT> = inputs.fold(Vec::new(), |mut v, i|{v.add(i); v});
    let c = unsafe {winapi::um::winuser::SendInput(ia.len() as u32, ia.as_mut_ptr(), mem::size_of::<INPUT>() as i32)};
    match ia.len() == c as usize {
        true => Ok(()),
        false => anyhow::bail!("Count not inject all inputs!")
    }
}

#[allow(dead_code)]
pub fn send_key(input: &Input) -> anyhow::Result<()> {
    send_keys(std::iter::once(input))
}


fn key_to_u16(key: &VirtualKey) -> u16 {
    (unsafe { ((key as *const VirtualKey) as *const u32).read() }) as u16
}