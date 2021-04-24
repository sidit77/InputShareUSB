pub use enums::{KeyState, VirtualKey, ScrollDirection, WindowsScanCode, InputEvent, Input};
pub use hook::InputHook;
pub use send::{send_keys, send_key};

mod enums;
mod send;
mod hook;