use error_tools::log::LogResultExt;
use tokio::sync::mpsc::UnboundedSender;
use yawi::{Input, InputEvent, KeyEvent, KeyState, send_inputs, VirtualKey};
use crate::Config;
use crate::hook::util::VirtualKeySet;

#[derive(Debug, Copy, Clone)]
pub enum HookEvent {
    Captured(bool),
    Input(InputEvent)
}

pub fn create_callback(config: &Config, sender: UnboundedSender<HookEvent>) -> impl FnMut(InputEvent) -> bool + 'static {
    let mut old_mouse_pos = yawi::get_cursor_pos();

    let blacklist = VirtualKeySet::from(&config.blacklist);
    let modifiers = VirtualKeySet::from(&config.hotkey.modifiers);
    let trigger = config.hotkey.trigger;

    let mut captured = false;
    let mut pressed_keys = VirtualKeySet::new();
    let mut hotkey_pressed = false;

    move |event|{
        let key_event = event.to_key_event();
        if is_blacklisted(blacklist, key_event) {
            return true;
        }

        let should_handle = is_repeated_event(&mut pressed_keys, key_event);

        if should_handle {
            if let Some(KeyEvent{ key, state}) = key_event {
                if pressed_keys.is_superset(modifiers) && key == trigger && state == KeyState::Pressed {
                    hotkey_pressed = true;
                    if !captured {
                        try_release_all(pressed_keys, trigger);
                    }
                    captured = !captured;
                    sender.send(HookEvent::Captured(captured))
                        .log_ok("Can not send event");
                    return false;
                }
                if hotkey_pressed && key == trigger && state == KeyState::Released {
                    hotkey_pressed = false;
                    return false;
                }
            }
            if captured {
                if let InputEvent::MouseMoveEvent(x, y) = event {
                    let (ox, oy) = old_mouse_pos;
                    if x != ox || y != oy {
                        sender.send(HookEvent::Input(InputEvent::MouseMoveEvent(x - ox, y - oy)))
                            .log_ok("Can not send input event");
                    }
                } else {
                    sender.send(HookEvent::Input(event))
                        .log_ok("Can not send input event");
                }

            } else if let InputEvent::MouseMoveEvent(x, y) = event {
                old_mouse_pos = (x,y);
            }
        }

        !captured
    }
}

fn is_blacklisted(blacklist: VirtualKeySet, event: Option<KeyEvent>) -> bool {
    event.map_or(false, |event| blacklist.contains(event.key))
}

fn is_repeated_event(pressed_keys: &mut VirtualKeySet, event: Option<KeyEvent>) -> bool {
    match event {
        Some(event) => match (pressed_keys.contains(event.key), event.state) {
            (false, KeyState::Pressed) => {
                pressed_keys.insert(event.key);
                true
            },
            (true, KeyState::Released) => {
                pressed_keys.remove(event.key);
                true
            },
            _ => false
        }
        None => true
    }
}

fn try_release_all(keys: VirtualKeySet, trigger: VirtualKey) {
    send_inputs(keys
        .iter()
        .filter(|k| *k != trigger)
        .map(|k| match k.is_mouse_button() {
            true => Input::MouseButtonInput(k, KeyState::Released),
            false => Input::KeyboardKeyInput(k, KeyState::Released),
        }))
        .log_ok("Could not send input events");
}

mod util {
    use std::fmt::{Debug, Formatter};
    use druid::im::HashSet;
    use yawi::VirtualKey;

    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct VirtualKeySet {
        keys: [u64; 4]
    }

    impl VirtualKeySet {
        pub fn new() -> Self {
            Self {
                keys: [0; 4],
            }
        }

        #[inline]
        fn index(key: VirtualKey) -> (usize, u64) {
            let id = u8::from(key);
            let index = (id >> 6) as usize;
            let mask = 1u64 << (id & 0b0011_1111) as u64;
            (index, mask)
        }

        pub fn insert(&mut self, key: VirtualKey) {
            let (index, mask) = Self::index(key);
            self.keys[index] |= mask;
        }

        pub fn remove(&mut self, key: VirtualKey) {
            let (index, mask) = Self::index(key);
            self.keys[index] &= !mask;
        }

        pub fn contains(self, key: VirtualKey) -> bool {
            let (index, mask) = Self::index(key);
            self.keys[index] & mask != 0
        }

        pub fn is_superset(self, other: VirtualKeySet) -> bool {
            self.keys
                .iter()
                .zip(other.keys.iter())
                .all(|(set, sub)| {
                    set & sub == *sub
                })
        }

        pub fn iter(self) -> impl Iterator<Item = VirtualKey> {
            (0..4)
                .into_iter()
                .flat_map(move |index|(0..64)
                    .into_iter()
                    .filter(move |i| self.keys[index] & (1 << i) != 0)
                    .filter_map(move |i| VirtualKey::try_from(((index << 6) | i) as u8).ok()))
        }

    }

    impl Debug for VirtualKeySet {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_set().entries(self.iter()).finish()
        }
    }

    impl From<&HashSet<VirtualKey>> for VirtualKeySet {
        fn from(value: &HashSet<VirtualKey>) -> Self {
            let mut result = VirtualKeySet::new();
            for key in value {
                result.insert(*key);
            }
            result
        }
    }

}
