use winapi::um::winuser::{UnhookWindowsHookEx, SetWindowsHookExW, MapVirtualKeyW, CallNextHookEx, KBDLLHOOKSTRUCT,
                          WH_KEYBOARD_LL, VK_SNAPSHOT, VK_SCROLL, VK_PAUSE, VK_NUMLOCK, MAPVK_VK_TO_VSC_EX, LLKHF_EXTENDED, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP};
use winapi::um::libloaderapi::GetModuleHandleW;
use crate::inputhook::InputEvent;
use winapi::shared::windef::HHOOK;
use crate::keys::{KeyState, VirtualKey, WindowsScanCode};
use winapi::shared::minwindef::{WPARAM, LPARAM, LRESULT};
use std::os::raw;
use std::ptr::{null, null_mut};
use std::borrow::BorrowMut;
use std::rc::{Rc, Weak};
use std::ops::{Deref, DerefMut};
use std::cell::{Cell, RefCell};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, RecvError};
use std::thread::{Thread, JoinHandle};

static mut KEYBOARD_HOOK: Option<HHOOK> = None;

/*
static mut TEST: Option<*mut dyn FnMut()> = None;

pub struct Processor<'a> {
    pub callback: Box<dyn FnMut() + 'a>,
}

impl<'a> Processor<'a>{
    pub fn new<T>(c: T) -> Self  where T: FnMut() + 'a{
        let mut callback = Box::new(c);
        let mut result = Self {
            callback
        };
        unsafe {
            let x = &*result.callback;
            TEST = Some(std::mem::transmute(x));
        }
        result
    }
}

pub fn call_static(){
    unsafe {
        (*TEST.unwrap())()
    }
}
*/


static mut TEST: Vec<Weak<RefCell<dyn FnMut()>>> = Vec::new();

pub struct Processor<'a> {
    pub callback: Rc<RefCell<dyn FnMut() + 'a>>,
}

impl<'a> Processor<'a>{
    pub fn new<T>(c: T) -> Self  where T: FnMut() + 'a{
        let mut callback = Rc::new(RefCell::new(c));
         let mut result = Self {
             callback
         };
         unsafe {
             let x = Rc::downgrade(&result.callback);
             TEST.push(std::mem::transmute(x));
             if KEYBOARD_HOOK.is_none(){
                 KEYBOARD_HOOK = Some(SetWindowsHookExW(
                     WH_KEYBOARD_LL, Some(low_level_keyboard_proc), GetModuleHandleW(null()), 0));
                 println!("Created Windows Hook!");
             }
         }
         result
    }
}

//pub fn call_static(){
//    unsafe {
//        //match TEST.as_mut().unwrap().upgrade() {
//        //    None => println!("I dont exist anymore"),
//        //    Some(mut x) => {
//        //        (x.deref().borrow_mut().deref_mut())();
//        //    }
//        //}
//        TEST.retain(|x| !matches!(x.upgrade(), None));
//        for x in TEST.iter_mut() {
//            match x.upgrade() {
//                None => println!("I dont exist anymore"),
//                Some(mut x) => {
//                    (x.deref().borrow_mut().deref_mut())();
//                }
//            }
//        }
//        //(*TEST.unwrap())()
//    }
//}

impl<'a> Drop for Processor<'a> {
    fn drop(&mut self) {
        println!("i was droped");
        unsafe {
            TEST.retain(|x| !matches!(x.upgrade(), None));
            if TEST.len() <= 1 && KEYBOARD_HOOK.is_some() {
                UnhookWindowsHookEx(KEYBOARD_HOOK.unwrap());
                KEYBOARD_HOOK = None;
                println!("Removed Windows Hook!");
            }
        }
    }
}

trait InputEventHandler {
    fn receive(&mut self, event: InputEvent) -> bool;
}

pub struct InputHook<T> where T: FnMut(InputEvent) -> bool{
    callback: T
}

impl<T> InputHook<T>  where T: FnMut(InputEvent) -> bool {
    pub fn create(callback: T) -> Self{

        //let result: Arc<dyn InputEventHandler> = Arc::new(Self{
        //    callback
        //});

        //unsafe {
        //    if KEYBOARD_HOOK.is_none() {
        //        let module = ;
        //        KEYBOARD_HOOK = Some();
        //        //CALLBACK = Some(Arc::downgrade(&result))
        //    }
        //}
        println!("Created Hook!");

        Self{
            callback
        }
    }
}

impl<'a, T> InputEventHandler for InputHook<T>  where T: FnMut(InputEvent) -> bool {
    fn receive(&mut self, event: InputEvent) -> bool {
        (self.callback)(event)
    }
}

impl<'a, T> Drop for InputHook<T> where T: FnMut(InputEvent) -> bool {
    fn drop(&mut self) {
        unsafe {
            if let Some(hook) = KEYBOARD_HOOK {
                UnhookWindowsHookEx(hook);
                KEYBOARD_HOOK = None;
            }
        }
        println!("Droped Hook");
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
    TEST.retain(|x| !matches!(x.upgrade(), None));

    if code >= 0 {
        let key_struct = *(lparam as *const KBDLLHOOKSTRUCT);
        let event = InputEvent::KeyboardEvent(parse_virtual_key(&key_struct), parse_scancode(&key_struct), parse_key_state(wparam as u32));

        println!("{:?} {:?} ({:x?})", parse_key_state(wparam as u32), parse_virtual_key(&key_struct) ,parse_scancode(&key_struct));

        if matches!(parse_virtual_key(&key_struct), VirtualKey::Escape) {
            if matches!(parse_key_state(wparam as u32), KeyState::Pressed) {
                nwg::stop_thread_dispatch();
            }

        }

        //if(!keyCallback(KeyEventArgs(
        //    static_cast<VirtualKey>(keyStruct->vkCode),
        //                getScanCode(keyStruct),
        //                getState(wParam))))

        if matches!(parse_key_state(wparam as u32), KeyState::Pressed) {
            for x in TEST.iter_mut() {
                match x.upgrade() {
                    None => println!("I dont exist anymore"),
                    Some(mut x) => {
                        (x.deref().borrow_mut().deref_mut())();
                    }
                }
            }
        }


    }
    CallNextHookEx(KEYBOARD_HOOK.unwrap(), code, wparam, lparam)
}