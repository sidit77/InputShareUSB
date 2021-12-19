use std::convert::TryInto;
use std::mem;
use std::ptr::{null, null_mut};
use std::time::Duration;
use winapi::shared::minwindef::{FALSE};
use winapi::shared::winerror::WAIT_TIMEOUT;
use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0, WAIT_FAILED};
use winapi::um::winuser::{MSG, MsgWaitForMultipleObjects, PeekMessageW, PM_REMOVE, QS_ALLINPUT};


pub fn wait_message_timeout(timeout: Option<Duration>) -> std::io::Result<bool> {
    let timeout = match timeout {
        None => INFINITE,
        Some(duration) => duration.as_millis().try_into().expect("timout to large")
    };
    unsafe {
        match MsgWaitForMultipleObjects(0, null(), FALSE, timeout, QS_ALLINPUT) {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED => Err(std::io::Error::last_os_error()),
            _ => panic!("invalid return type")
        }
    }
}

pub fn get_message() -> Option<MSG> {
    unsafe {
        let mut msg: MSG = mem::zeroed();
        match PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) {
            FALSE => None,
            _ => Some(msg)
        }
    }
}
