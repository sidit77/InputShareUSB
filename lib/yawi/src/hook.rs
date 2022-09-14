// partially adapted from https://github.com/timokroeger/kbremap

use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, MapVirtualKeyW, CallNextHookEx, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, VK_SNAPSHOT, VK_SCROLL, VK_PAUSE, VK_NUMLOCK, MAPVK_VK_TO_VSC_EX, LLKHF_EXTENDED, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WH_MOUSE_LL, MSLLHOOKSTRUCT, LLKHF_INJECTED, LLMHF_INJECTED, HC_ACTION};
use winapi::shared::windef::HHOOK;
use winapi::shared::minwindef::{WPARAM, LPARAM, LRESULT, UINT};
use std::cell::Cell;
use std::os::raw;
use crate::{VirtualKey, ScrollDirection, KeyState, WindowsScanCode, InputEvent};
use std::convert::{TryFrom, TryInto};
use std::ptr;

type HookFn = dyn FnMut(InputEvent) -> bool;

thread_local! {
    static HOOK: Cell<Option<Box<HookFn>>> = Cell::default();
}

pub struct InputHook {
    keyboard: HHOOK,
    mouse: HHOOK
}

impl InputHook {
    #[must_use = "The hook will immediately be unregistered and not work."]
    pub fn register(callback: impl FnMut(InputEvent) -> bool + 'static) -> InputHook {
        HOOK.with(|state| {
            assert!(
                state.take().is_none(),
                "Only one keyboard hook can be registered per thread."
            );

            state.set(Some(Box::new(callback)));
            log::debug!("Registering system hooks");
            InputHook {
                keyboard: unsafe {
                    SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), ptr::null_mut(), 0)
                        .as_mut()
                        .expect("Failed to install low-level keyboard hook.")
                },
                mouse: unsafe {
                    SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), ptr::null_mut(), 0)
                        .as_mut()
                        .expect("Failed to install low-level mouse hook.")
                },
            }
        })
    }
}

impl Drop for InputHook {
    fn drop(&mut self) {
        log::debug!("Removing system hooks");
        unsafe { UnhookWindowsHookEx(self.keyboard) };
        unsafe { UnhookWindowsHookEx(self.mouse) };
        HOOK.with(|state| state.take());
    }
}

unsafe extern "system" fn low_level_keyboard_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);
    if code == HC_ACTION && !parse_injected(&key_struct) {
        let event = match parse_virtual_key(&key_struct) {
            Some(key) => match parse_key_state(wparam) {
                Some(state) => Some(InputEvent::KeyboardKeyEvent(key, parse_scancode(&key_struct), state)),
                None => {log::warn!("Unknown event: {}", wparam); None}
            }
            None => {log::warn!("Unknown key: {}", key_struct.vkCode); None}
        };

        if let Some(event) = event {
            let mut handled = true;
            HOOK.with(|state| {
                match state.take() {
                    None => log::warn!("Keyboard hook callback was already taken"),
                    Some(mut callback) => {
                        handled = !callback(event);
                        state.set(Some(callback));
                    }
                }
            });
            if handled {
                return 1;
            }
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn low_level_mouse_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let key_struct = *(lparam as *const MSLLHOOKSTRUCT);
    if code == HC_ACTION &&  !(key_struct.flags & LLMHF_INJECTED != 0) {
        let event = match wparam as u32{
            winapi::um::winuser::WM_LBUTTONDOWN => Some(InputEvent::MouseButtonEvent(VirtualKey::LButton, KeyState::Pressed)),
            winapi::um::winuser::WM_LBUTTONUP => Some(InputEvent::MouseButtonEvent(VirtualKey::LButton, KeyState::Released)),
            winapi::um::winuser::WM_RBUTTONDOWN => Some(InputEvent::MouseButtonEvent(VirtualKey::RButton, KeyState::Pressed)),
            winapi::um::winuser::WM_RBUTTONUP => Some(InputEvent::MouseButtonEvent(VirtualKey::RButton, KeyState::Released)),
            winapi::um::winuser::WM_MBUTTONDOWN => Some(InputEvent::MouseButtonEvent(VirtualKey::MButton, KeyState::Pressed)),
            winapi::um::winuser::WM_MBUTTONUP => Some(InputEvent::MouseButtonEvent(VirtualKey::MButton, KeyState::Released)),
            winapi::um::winuser::WM_XBUTTONDOWN => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Pressed)),
            winapi::um::winuser::WM_XBUTTONUP => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Released)),
            winapi::um::winuser::WM_NCXBUTTONDOWN => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Pressed)),
            winapi::um::winuser::WM_NCXBUTTONUP => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Released)),
            winapi::um::winuser::WM_MOUSEMOVE => Some(InputEvent::MouseMoveEvent(key_struct.pt.x, key_struct.pt.y)),
            winapi::um::winuser::WM_MOUSEWHEEL => Some(InputEvent::MouseWheelEvent(ScrollDirection::Vertical(parse_wheel_delta(&key_struct)))),
            winapi::um::winuser::WM_MOUSEHWHEEL => Some(InputEvent::MouseWheelEvent(ScrollDirection::Horizontal(parse_wheel_delta(&key_struct)))),
            _ => {log::warn!("Unknown: {}", wparam); None}
        };

        if let Some(event) = event {
            let mut handled = true;
            HOOK.with(|state| {
                match state.take() {
                    None => log::warn!("Mouse hook callback was already taken"),
                    Some(mut callback) => {
                        handled = !callback(event);
                        state.set(Some(callback));
                    }
                }
            });
            if handled {
                return 1;
            }
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

fn parse_wheel_delta(key_struct: &MSLLHOOKSTRUCT) -> f32{
    (key_struct.mouseData >> 16) as i16 as f32 / winapi::um::winuser::WHEEL_DELTA as f32
}

fn parse_xbutton(key_struct: &MSLLHOOKSTRUCT) -> Option<VirtualKey>{
    match (key_struct.mouseData >> 16) as u16 {
        winapi::um::winuser::XBUTTON1 => Some(VirtualKey::XButton1),
        winapi::um::winuser::XBUTTON2 => Some(VirtualKey::XButton2),
        _ => None
    }
}

fn parse_injected(key_struct: &KBDLLHOOKSTRUCT) -> bool {
    // Workaround because some keys (e.g play/pause) are flagged as injected even if they arent.
    // However injected keys don't seem to have the extended flag so this is used to remove these false positives
    key_struct.flags & LLKHF_INJECTED != 0 && key_struct.flags & LLKHF_EXTENDED == 0
}

fn parse_scancode(key_struct: &KBDLLHOOKSTRUCT) -> WindowsScanCode {
    let mut scancode = key_struct.scanCode as WindowsScanCode;
    let vk = key_struct.vkCode as i32;
    if scancode == 0x0 || vk == VK_SNAPSHOT || vk == VK_SCROLL || vk == VK_PAUSE || vk == VK_NUMLOCK {
        scancode = unsafe {MapVirtualKeyW(key_struct.vkCode, MAPVK_VK_TO_VSC_EX)} as WindowsScanCode;
    } else if key_struct.flags & LLKHF_EXTENDED == LLKHF_EXTENDED {
        scancode |= 0xe000;
    }
    scancode
}

fn parse_virtual_key(key_struct: &KBDLLHOOKSTRUCT) -> Option<VirtualKey> {
    key_struct.vkCode.try_into().ok().and_then(|vk:u8|VirtualKey::try_from(vk).ok())
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