mod enums;
mod hook;
mod message;
mod query;
mod send;

pub type WinResult<T> = windows::core::Result<T>;

pub use enums::{Input, InputEvent, KeyEvent, KeyState, ScrollDirection, VirtualKey, WindowsScanCode};
pub use hook::{HookAction, HookFn, InputHook};
pub use message::{quit, run};
pub use query::get_cursor_pos;
pub use send::{send_input, send_inputs};
