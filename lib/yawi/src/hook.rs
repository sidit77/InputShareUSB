// partially adapted from https://github.com/timokroeger/kbremap

use std::cell::Cell;
use std::convert::{TryFrom, TryInto};

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::{InputEvent, KeyState, ScrollDirection, VirtualKey, WinResult, WindowsScanCode};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum HookAction {
    #[default]
    Continue,
    Block
}

#[repr(transparent)]
pub struct HookFn(Box<dyn FnMut(InputEvent) -> HookAction>);

impl HookFn {
    pub fn new(callback: impl FnMut(InputEvent) -> HookAction + 'static) -> Self {
        Self(Box::new(callback))
    }

    fn handle(&mut self, event: InputEvent) -> HookAction {
        self.0(event)
    }
}

impl<F: FnMut(InputEvent) -> HookAction + 'static> From<F> for HookFn {
    fn from(value: F) -> Self {
        HookFn::new(value)
    }
}

thread_local! {
    static HOOK: Cell<Option<HookFn>> = Cell::default();
}

pub struct InputHook {
    keyboard: HHOOK,
    mouse: HHOOK
}

impl InputHook {
    #[must_use = "The hook will immediately be unregistered and not work."]
    pub fn register(callback: impl Into<HookFn>) -> WinResult<InputHook> {
        HOOK.with(|state| {
            assert!(state.take().is_none(), "Only one keyboard hook can be registered per thread.");

            tracing::trace!("Registering system hooks");
            let keyboard = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), HINSTANCE::default(), 0)? };
            let mouse = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), HINSTANCE::default(), 0)? };
            state.set(Some(callback.into()));
            Ok(InputHook { keyboard, mouse })
        })
    }
}

impl Drop for InputHook {
    fn drop(&mut self) {
        tracing::trace!("Removing system hooks");
        HOOK.with(|state| state.take());
        unsafe { UnhookWindowsHookEx(self.keyboard) };
        unsafe { UnhookWindowsHookEx(self.mouse) };
    }
}

unsafe extern "system" fn low_level_keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let key_struct = (lparam.0 as *const KBDLLHOOKSTRUCT).read();
    if code == HC_ACTION as i32 && !parse_injected(&key_struct) {
        let event = match parse_virtual_key(&key_struct) {
            Some(key) => match parse_key_state(wparam) {
                Some(state) => Some(InputEvent::KeyboardKeyEvent(key, parse_scancode(&key_struct), state)),
                None => {
                    tracing::warn!("Unknown event: {}", wparam.0);
                    None
                }
            },
            None => {
                tracing::warn!("Unknown key: {}", key_struct.vkCode);
                None
            }
        };

        if let Some(event) = event {
            let mut handled = HookAction::Continue;
            HOOK.with(|state| match state.take() {
                None => tracing::warn!("Keyboard hook callback was already taken"),
                Some(mut callback) => {
                    handled = callback.handle(event);
                    state.set(Some(callback));
                }
            });
            if handled == HookAction::Block {
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

unsafe extern "system" fn low_level_mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let key_struct = (lparam.0 as *const MSLLHOOKSTRUCT).read();
    if code == HC_ACTION as i32 && key_struct.flags & LLMHF_INJECTED == 0 {
        let event = match wparam.0 as u32 {
            WM_LBUTTONDOWN => Some(InputEvent::MouseButtonEvent(VirtualKey::LButton, KeyState::Pressed)),
            WM_LBUTTONUP => Some(InputEvent::MouseButtonEvent(VirtualKey::LButton, KeyState::Released)),
            WM_RBUTTONDOWN => Some(InputEvent::MouseButtonEvent(VirtualKey::RButton, KeyState::Pressed)),
            WM_RBUTTONUP => Some(InputEvent::MouseButtonEvent(VirtualKey::RButton, KeyState::Released)),
            WM_MBUTTONDOWN => Some(InputEvent::MouseButtonEvent(VirtualKey::MButton, KeyState::Pressed)),
            WM_MBUTTONUP => Some(InputEvent::MouseButtonEvent(VirtualKey::MButton, KeyState::Released)),
            WM_XBUTTONDOWN => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Pressed)),
            WM_XBUTTONUP => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Released)),
            WM_NCXBUTTONDOWN => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Pressed)),
            WM_NCXBUTTONUP => parse_xbutton(&key_struct).map(|k| InputEvent::MouseButtonEvent(k, KeyState::Released)),
            WM_MOUSEMOVE => Some(InputEvent::MouseMoveEvent(key_struct.pt.x, key_struct.pt.y)),
            WM_MOUSEWHEEL => Some(InputEvent::MouseWheelEvent(ScrollDirection::Vertical(parse_wheel_delta(&key_struct)))),
            WM_MOUSEHWHEEL => Some(InputEvent::MouseWheelEvent(ScrollDirection::Horizontal(parse_wheel_delta(&key_struct)))),
            _ => {
                tracing::warn!("Unknown: {}", wparam.0);
                None
            }
        };

        if let Some(event) = event {
            let mut handled = HookAction::Continue;
            HOOK.with(|state| match state.take() {
                None => tracing::warn!("Mouse hook callback was already taken"),
                Some(mut callback) => {
                    handled = callback.handle(event);
                    state.set(Some(callback));
                }
            });
            if handled == HookAction::Block {
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

fn parse_wheel_delta(key_struct: &MSLLHOOKSTRUCT) -> f32 {
    (key_struct.mouseData >> 16) as i16 as f32 / WHEEL_DELTA as f32
}

fn parse_xbutton(key_struct: &MSLLHOOKSTRUCT) -> Option<VirtualKey> {
    match (key_struct.mouseData >> 16) as u16 {
        XBUTTON1 => Some(VirtualKey::XButton1),
        XBUTTON2 => Some(VirtualKey::XButton2),
        _ => None
    }
}

fn parse_injected(key_struct: &KBDLLHOOKSTRUCT) -> bool {
    // Workaround because some keys (e.g play/pause) are flagged as injected even if they arent.
    // However injected keys don't seem to have the extended flag so this is used to remove these false positives
    key_struct.flags & LLKHF_INJECTED != Default::default() && key_struct.flags & LLKHF_EXTENDED == Default::default()
}

fn parse_scancode(key_struct: &KBDLLHOOKSTRUCT) -> WindowsScanCode {
    let mut scancode = key_struct.scanCode as WindowsScanCode;
    let vk = VIRTUAL_KEY(key_struct.vkCode as u16);
    if scancode == 0x0 || vk == VK_SNAPSHOT || vk == VK_SCROLL || vk == VK_PAUSE || vk == VK_NUMLOCK {
        scancode = unsafe { MapVirtualKeyW(key_struct.vkCode, MAPVK_VK_TO_VSC_EX) } as WindowsScanCode;
    } else if key_struct.flags & LLKHF_EXTENDED == LLKHF_EXTENDED {
        scancode |= 0xe000;
    }
    scancode
}

fn parse_virtual_key(key_struct: &KBDLLHOOKSTRUCT) -> Option<VirtualKey> {
    key_struct
        .vkCode
        .try_into()
        .ok()
        .and_then(|vk: u8| VirtualKey::try_from(vk).ok())
}

fn parse_key_state(wparam: WPARAM) -> Option<KeyState> {
    match wparam.0 as u32 {
        WM_KEYDOWN => Some(KeyState::Pressed),
        WM_SYSKEYDOWN => Some(KeyState::Pressed),
        WM_KEYUP => Some(KeyState::Released),
        WM_SYSKEYUP => Some(KeyState::Released),
        _ => None
    }
}
