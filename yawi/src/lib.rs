pub use enums::{KeyState, VirtualKey, ScrollDirection, WindowsScanCode, InputEvent, Input};
pub use hook::InputHook;
pub use send::{send_inputs, send_input};
pub use message::{run, quit, Quitter};

mod enums;
mod send;
mod hook;
mod message;
mod error;