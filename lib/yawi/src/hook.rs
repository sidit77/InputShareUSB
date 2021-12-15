use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, MapVirtualKeyW, CallNextHookEx, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, VK_SNAPSHOT, VK_SCROLL, VK_PAUSE, VK_NUMLOCK, MAPVK_VK_TO_VSC_EX, LLKHF_EXTENDED, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WH_MOUSE_LL, MSLLHOOKSTRUCT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::HHOOK;
use winapi::shared::minwindef::{WPARAM, LPARAM, LRESULT, UINT};
use std::os::raw;
use std::ptr::{null};
use std::rc::{Rc, Weak};
use std::ops::{Deref, DerefMut};
use std::cell::RefCell;
use crate::{VirtualKey, ScrollDirection, KeyState, WindowsScanCode, InputEvent};
use std::convert::TryInto;
use crate::error::Error;

static mut NATIVE_HOOK: Option<NativeHook> = None;

struct NativeHook {
    keyboard: HHOOK,
    mouse: HHOOK
}

impl NativeHook {
    unsafe fn register() -> crate::Result<Self>{
        let handle = check(GetModuleHandleW(null()))?;
        let keyboard = check(SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), handle, 0))?;
        let mouse    = check(SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), handle, 0))?;
        println!("Registered native hooks!");
        Ok(Self {
            keyboard,
            mouse
        })
    }
}

fn check<T>(ptr: *mut T) -> crate::Result<*mut T>{
    if ptr.is_null() {
        Err(Error::last())
    } else {
        Ok(ptr)
    }
}

impl Drop for NativeHook {
    fn drop(&mut self) {
        unsafe {
            UnhookWindowsHookEx(self.keyboard);
            UnhookWindowsHookEx(self.mouse);
        }
        println!("Unregistered native hooks!");
    }
}


static mut CALLBACKS: Vec<Weak<RefCell<dyn FnMut(InputEvent) -> bool>>> = Vec::new();

pub struct InputHook<'a> {
    pub callback: Rc<RefCell<dyn FnMut(InputEvent) -> bool + 'a>>,
}

impl<'a> InputHook<'a>{
    pub fn new<T>(c: T) -> crate::Result<Self>  where T: FnMut(InputEvent) -> bool + 'a{
        let callback = Rc::new(RefCell::new(c));
        let result = Self {
            callback
        };

        unsafe {
            let x = Rc::downgrade(&result.callback);
            CALLBACKS.push(std::mem::transmute(x));
            if NATIVE_HOOK.is_none(){
                NATIVE_HOOK = Some(NativeHook::register()?);
            }
        }
        Ok(result)
    }
}

impl<'a> Drop for InputHook<'a> {
    fn drop(&mut self) {
        unsafe {
            CALLBACKS.retain(|x| !matches!(x.upgrade(), None));
            if CALLBACKS.len() <= 1 {
                NATIVE_HOOK = None;
            }
        }
    }
}

const IGNORE: usize = 0x1234567;

unsafe extern "system" fn low_level_keyboard_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    CALLBACKS.retain(|x| !matches!(x.upgrade(), None));

    if code >= 0 {
        let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);

        if key_struct.dwExtraInfo != IGNORE {

            let event = match parse_virtual_key(&key_struct) {
                Some(key) => match parse_key_state(wparam) {
                    Some(state) => Some(InputEvent::KeyboardKeyEvent(key, parse_scancode(&key_struct), state)),
                    None => {println!("Unknown event: {}", wparam); None}
                }
                None => {println!("Unknown key: {}", key_struct.vkCode); None}
            };

            if let Some(event) = event {
                if !for_all(event) {
                    return 1;
                }
            }

        }

    }
    CallNextHookEx(NATIVE_HOOK.as_ref().unwrap().keyboard, code, wparam, lparam)
}

unsafe extern "system" fn low_level_mouse_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    CALLBACKS.retain(|x| !matches!(x.upgrade(), None));

    if code >= 0 {
        let key_struct = *(lparam as *const MSLLHOOKSTRUCT);

        if key_struct.dwExtraInfo != IGNORE {
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
                _ => {println!("Unknown: {}", wparam); None}
            };

            if let Some(event) = event {
                if !for_all(event) {
                    return 1;
                }
            }
        }


    }

    CallNextHookEx(NATIVE_HOOK.as_ref().unwrap().mouse, code, wparam, lparam)
}

unsafe fn for_all(event: InputEvent) -> bool {
    CALLBACKS
        .iter_mut()
        .map(|x| match x.upgrade() {
            None => true,
            Some(y) => (y.deref().borrow_mut().deref_mut())(event.clone())
        })
        .fold(true, |a, b| a && b)
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

fn parse_scancode(key_struct: &KBDLLHOOKSTRUCT) -> WindowsScanCode {
    let mut scancode = key_struct.scanCode as WindowsScanCode;
    let vk = key_struct.vkCode as i32;
    if scancode == 0x0 || vk == VK_SNAPSHOT || vk == VK_SCROLL || vk == VK_PAUSE || vk == VK_NUMLOCK {
        scancode = unsafe {MapVirtualKeyW(key_struct.vkCode, MAPVK_VK_TO_VSC_EX)} as WindowsScanCode;
    } else {
        if key_struct.flags & LLKHF_EXTENDED == LLKHF_EXTENDED {
            scancode |= 0xe000;
        }
    }
    scancode
}

fn parse_virtual_key(key_struct: &KBDLLHOOKSTRUCT) -> Option<VirtualKey> {
    key_struct.vkCode.try_into().ok()
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