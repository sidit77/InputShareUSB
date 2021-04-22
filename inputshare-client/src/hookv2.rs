use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, MapVirtualKeyW, CallNextHookEx, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL, VK_SNAPSHOT, VK_SCROLL, VK_PAUSE, VK_NUMLOCK, MAPVK_VK_TO_VSC_EX, LLKHF_EXTENDED, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP};
use winapi::um::libloaderapi::GetModuleHandleW;
use crate::inputhook::InputEvent;
use winapi::shared::windef::HHOOK;
use crate::keys::{KeyState, VirtualKey, WindowsScanCode};
use winapi::shared::minwindef::{WPARAM, LPARAM, LRESULT};
use std::os::raw;
use std::ptr::{null};
use std::rc::{Rc, Weak};
use std::ops::{Deref, DerefMut};
use std::cell::RefCell;


static mut NATIVE_HOOK: Option<NativeHook> = None;

struct NativeHook {
    keyboard: HHOOK
}

impl NativeHook {
    unsafe fn register() -> Self{
        let handle = GetModuleHandleW(null());
        let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), handle, 0);
        println!("Registered native hooks!");
        Self {
            keyboard
        }
    }
}

impl Drop for NativeHook {
    fn drop(&mut self) {
        unsafe { UnhookWindowsHookEx(self.keyboard); }
        println!("Unregistered native hooks!");
    }
}


static mut CALLBACKS: Vec<Weak<RefCell<dyn FnMut(InputEvent) -> bool>>> = Vec::new();

pub struct InputHook<'a> {
    pub callback: Rc<RefCell<dyn FnMut(InputEvent) -> bool + 'a>>,
}

impl<'a> InputHook<'a>{
    pub fn new<T>(c: T) -> Self  where T: FnMut(InputEvent) -> bool + 'a{
        let callback = Rc::new(RefCell::new(c));
        let result = Self {
            callback
        };

        unsafe {
            let x = Rc::downgrade(&result.callback);
            CALLBACKS.push(std::mem::transmute(x));
            if NATIVE_HOOK.is_none(){
                NATIVE_HOOK = Some(NativeHook::register());
            }
        }
        result
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

unsafe extern "system" fn low_level_keyboard_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    CALLBACKS.retain(|x| !matches!(x.upgrade(), None));

    if code >= 0 {
        let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);
        let event = InputEvent::KeyboardEvent(parse_virtual_key(&key_struct), parse_scancode(&key_struct), parse_key_state(wparam as u32));

        let keep = CALLBACKS
            .iter_mut()
            .map(|x| match x.upgrade() {
                None => true,
                Some(y) => (y.deref().borrow_mut().deref_mut())(event.clone())
            })
            .fold(true, |a, b| a && b);

        if !keep {
            return 1;
        }

    }
    CallNextHookEx(NATIVE_HOOK.as_ref().unwrap().keyboard, code, wparam, lparam)
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

fn parse_virtual_key(key_struct: &KBDLLHOOKSTRUCT) -> VirtualKey {
    unsafe {
        ((&(key_struct.vkCode) as *const u32) as *const VirtualKey).read()
    }
}

fn parse_key_state(wparam: u32) -> KeyState {
    match wparam {
        WM_KEYDOWN=> KeyState::Pressed,
        WM_SYSKEYDOWN=> KeyState::Pressed,
        WM_KEYUP=> KeyState::Released,
        WM_SYSKEYUP=> KeyState::Released,
        _ => KeyState::Released
    }
}