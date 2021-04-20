use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, WH_KEYBOARD_LL, CallNextHookEx, KBDLLHOOKSTRUCT};
use winapi::um::libloaderapi::GetModuleHandleW;
use std::os::raw;
use winapi::shared::minwindef::{WPARAM, LRESULT, LPARAM};
use winapi::shared::windef::HHOOK;
use std::ptr::null;

struct InputHooks{
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

unsafe extern "system" fn low_level_keyboard_proc(code: raw::c_int, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let hooks = HOOKS.as_mut().unwrap();
    if code >= 0 {
        let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);
        println!("{:x?}", key_struct.scanCode);
        //if(!keyCallback(KeyEventArgs(
        //    static_cast<VirtualKey>(keyStruct->vkCode),
        //                getScanCode(keyStruct),
        //                getState(wParam))))
        return 1;
    }
    CallNextHookEx(hooks.keyboard, code, wparam, lparam)
}

pub fn set_up_keyboard_hook() {
    unsafe {
        if HOOKS.is_none() {
            HOOKS = Some(InputHooks::create());
        }
    }
}

pub fn release_hook(){
    unsafe {
        HOOKS = None;
    }
}