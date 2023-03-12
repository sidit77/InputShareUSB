mod enums;
mod send;
mod hook;
mod message;

pub type WinResult<T> = windows::core::Result<T>;

pub use enums::{KeyState, VirtualKey, ScrollDirection, WindowsScanCode, InputEvent, Input, KeyEvent};
pub use hook::{InputHook};
pub use send::{send_inputs, send_input};
pub use message::{run, quit};
