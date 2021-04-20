use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, MapVirtualKeyW, CallNextHookEx, KBDLLHOOKSTRUCT,
                          WH_KEYBOARD_LL, VK_SNAPSHOT, VK_SCROLL, VK_PAUSE, VK_NUMLOCK, MAPVK_VK_TO_VSC_EX, LLKHF_EXTENDED, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP};
use winapi::um::libloaderapi::GetModuleHandleW;
use std::os::raw;
use winapi::shared::minwindef::{WPARAM, LRESULT, LPARAM};
use winapi::shared::windef::HHOOK;
use std::ptr::null;
use crate::keys::{WindowsScanCode, VirtualKey, KeyState};

pub enum InputEvent {
    KeyboardEvent(VirtualKey, WindowsScanCode, KeyState)
}

struct InputHooks {
    keyboard: HHOOK
}

impl InputHooks {
    fn create() -> Self {
        let module = unsafe { GetModuleHandleW(null()) };
        let keyboard = unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), module, 0)
        };

        println!("Set Input hook");

        Self {
            keyboard
        }
    }
}

impl Drop for InputHooks {
    fn drop(&mut self) {
        unsafe{
            UnhookWindowsHookEx(self.keyboard);
        }
        println!("Unhooked");
    }
}

static mut HOOKS: Option<InputHooks> = None;
static mut CALLBACK: Option<Box<dyn FnMut(InputEvent) -> bool + 'static>> = None;

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

unsafe extern "system" fn low_level_keyboard_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let hooks = HOOKS.as_mut().unwrap();
    if code >= 0 {
        let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);
        let event = InputEvent::KeyboardEvent(parse_virtual_key(&key_struct), parse_scancode(&key_struct), parse_key_state(wparam as u32));

        println!("{:?} {:?} ({:x?})", parse_key_state(wparam as u32), parse_virtual_key(&key_struct) ,parse_scancode(&key_struct));

        //if(!keyCallback(KeyEventArgs(
        //    static_cast<VirtualKey>(keyStruct->vkCode),
        //                getScanCode(keyStruct),
        //                getState(wParam))))
        if !CALLBACK.as_mut().unwrap()(event) {
            return 1;
        }
    }
    CallNextHookEx(hooks.keyboard, code, wparam, lparam)
}

pub fn set_up_keyboard_hook(event_handler: impl FnMut(InputEvent) -> bool + 'static) {
    unsafe {
        if HOOKS.is_none() {
            HOOKS = Some(InputHooks::create());
            CALLBACK = Some(Box::new(event_handler));
        }
    }
}

pub fn release_hook(){
    unsafe {
        HOOKS = None;
        CALLBACK = None;
    }
}