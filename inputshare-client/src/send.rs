use winapi::um::winuser::{INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE};
use crate::keys::{VirtualKey, KeyState};
use std::mem;
use winapi::shared::minwindef::DWORD;

const IGNORE: usize = 0x1234567;

pub enum Input {
    KeyboardInput(VirtualKey, KeyState),
    StringInput(String)
}

trait AddInputs{
    fn add(&mut self, input: &Input);
}

impl AddInputs for Vec<INPUT> {
    fn add(&mut self, input: &Input) {
        match input {
            Input::KeyboardInput(key, state) =>  {
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
        }
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

pub fn send_keys<'a>(inputs: impl Iterator<Item=&'a Input>) {
    let mut ia: Vec<INPUT> = inputs.fold(Vec::new(), |mut v, i|{v.add(i); v});
    unsafe {
        winapi::um::winuser::SendInput(ia.len() as u32, ia.as_mut_ptr(), mem::size_of::<INPUT>() as i32);
    }
}

pub fn send_key(input: &Input) {
    send_keys(std::iter::once(input));
}


fn key_to_u16(key: &VirtualKey) -> u16 {
    (unsafe { ((key as *const VirtualKey) as *const u32).read() }) as u16
}