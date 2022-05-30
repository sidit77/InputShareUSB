use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, MapVirtualKeyW, CallNextHookEx, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, VK_SNAPSHOT, VK_SCROLL, VK_PAUSE, VK_NUMLOCK, MAPVK_VK_TO_VSC_EX, LLKHF_EXTENDED, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WH_MOUSE_LL, MSLLHOOKSTRUCT, LLKHF_INJECTED, LLMHF_INJECTED};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::HHOOK;
use winapi::shared::minwindef::{WPARAM, LPARAM, LRESULT, UINT};
use std::os::raw;
use std::ptr::{null};
use crate::{VirtualKey, ScrollDirection, KeyState, WindowsScanCode, InputEvent};
use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;
use std::io::{Error, ErrorKind, Result};

struct NativeHook {
    keyboard: Option<HHOOK>,
    mouse: Option<HHOOK>,
    callback: Box<dyn FnMut(InputEvent) -> bool>,
    ignore_injected: bool
}

impl NativeHook {
    unsafe fn register(keyboard: bool, mouse: bool, ignore_injected: bool, callback: Box<dyn FnMut(InputEvent) -> bool>) -> Result<Self>{
        let handle = check(GetModuleHandleW(null()))?;

        let keyboard = if keyboard {
            Some(check(SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), handle, 0))?)
        } else {
            None
        };
        let mouse    = if mouse {
            Some(check(SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), handle, 0))?)
        } else {
            None
        };
        log::debug!("Registered native hooks!");
        Ok(Self {
            keyboard,
            mouse,
            callback,
            ignore_injected
        })
    }
}

fn check<T>(ptr: *mut T) -> Result<*mut T>{
    if ptr.is_null() {
        Err(Error::last_os_error())
    } else {
        Ok(ptr)
    }
}

impl Drop for NativeHook {
    fn drop(&mut self) {
        unsafe {
            if let Some(hook) = self.keyboard {
                UnhookWindowsHookEx(hook);
            }
            if let Some(hook) = self.mouse {
                UnhookWindowsHookEx(hook);
            }
        }
        log::debug!("Unregistered native hooks!");
    }
}

static mut NATIVE_HOOK: Option<NativeHook> = None;

#[derive(Debug, Copy, Clone)]
pub enum HookType {
    Keyboard,
    Mouse,
    KeyboardMouse
}

impl HookType {
    fn is_mouse(self) -> bool {
        match self {
            HookType::Keyboard => false,
            HookType::Mouse => true,
            HookType::KeyboardMouse => true
        }
    }
    fn is_keyboard(self) -> bool {
        match self {
            HookType::Keyboard => true,
            HookType::Mouse => false,
            HookType::KeyboardMouse => true
        }
    }
}

#[derive(Default)]
pub struct InputHook<'a>{
    inner: PhantomData<&'a u8>
}

impl<'a> InputHook<'a>{
    pub fn new<T>(callback: T, ignore_injected: bool, hook_type: HookType) -> Result<Self>  where T: FnMut(InputEvent) -> bool + 'a {

        unsafe {

            match NATIVE_HOOK {
                None => {
                    let callback: Box<dyn FnMut(InputEvent) -> bool + 'a> = Box::new(callback);
                    NATIVE_HOOK = Some(NativeHook::register(
                        hook_type.is_keyboard(),
                        hook_type.is_mouse(),
                        ignore_injected,
                        std::mem::transmute(callback))?);
                    Ok(Self::default())
                },
                Some(_) => Err(Error::new(ErrorKind::AlreadyExists, "A hook for this thread is already set"))
            }
        }
    }

    pub fn remove(self) {
        std::mem::drop(self)
    }
}

impl<'a> Drop for InputHook<'a> {
    fn drop(&mut self) {
        unsafe {
            NATIVE_HOOK = None
        }
    }
}

unsafe extern "system" fn low_level_keyboard_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let nhook = NATIVE_HOOK.as_mut().unwrap();

    if code >= 0 {
        let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);

        if !nhook.ignore_injected || !parse_injected(&key_struct) {

            let event = match parse_virtual_key(&key_struct) {
                Some(key) => match parse_key_state(wparam) {
                    Some(state) => Some(InputEvent::KeyboardKeyEvent(key, parse_scancode(&key_struct), state)),
                    None => {log::warn!("Unknown event: {}", wparam); None}
                }
                None => {log::warn!("Unknown key: {}", key_struct.vkCode); None}
            };

            if let Some(event) = event {
                if !nhook.callback.as_mut()(event) {
                    return 1;
                }
            }

        }

    }
    CallNextHookEx(nhook.keyboard.unwrap(), code, wparam, lparam)
}

unsafe extern "system" fn low_level_mouse_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let nhook = NATIVE_HOOK.as_mut().unwrap();

    if code >= 0 {
        let key_struct = *(lparam as *const MSLLHOOKSTRUCT);

        if !nhook.ignore_injected || key_struct.flags & LLMHF_INJECTED == 0 {
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
                if !nhook.callback.as_mut()(event) {
                    return 1;
                }
            }
        }


    }

    CallNextHookEx(nhook.mouse.unwrap(), code, wparam, lparam)
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