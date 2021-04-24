pub use enums::{KeyState, VirtualKey, ScrollDirection, WindowsScanCode, InputEvent, Input};
pub use hook::InputHook;
pub use send::{send_keys, send_key};
pub use message::{run, quit, Quitter};

mod enums;
mod send;
mod hook;
mod message;