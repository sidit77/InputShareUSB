use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;
use yawi::{send_inputs, HookAction, HookFn, Input, InputEvent, KeyEvent, KeyState, VirtualKey};

use crate::model::Config;
use crate::utils::keyset::VirtualKeySet;

#[derive(Debug, Copy, Clone)]
pub enum HookEvent {
    Captured(bool),
    Input(InputEvent)
}

const CAPTURE_TIMEOUT: Duration = Duration::from_millis(500);

pub fn create_callback(config: &Config, sender: UnboundedSender<HookEvent>) -> HookFn {
    let send = move |event| {
        sender
            .send(event)
            .unwrap_or_else(|err| tracing::warn!("Could not send event: {}", err))
    };

    let mut old_mouse_pos = yawi::get_cursor_pos();

    let blacklist = config.blacklist;
    let modifiers = config.hotkey.modifiers;
    let trigger = config.hotkey.trigger;

    let mut captured = false;
    let mut pressed_keys = VirtualKeySet::new();
    let mut hotkey_pressed = false;

    let mut last_swap = Instant::now();

    send(HookEvent::Captured(captured));
    HookFn::new(move |event| {
        let key_event = event.to_key_event();
        if is_blacklisted(blacklist, key_event) {
            return HookAction::Continue;
        }

        let should_handle = is_repeated_event(&mut pressed_keys, key_event);

        if should_handle {
            if let Some(KeyEvent { key, state }) = key_event {
                if pressed_keys.is_superset(modifiers) && key == trigger && state == KeyState::Pressed {
                    hotkey_pressed = true;
                    if last_swap.elapsed() >= CAPTURE_TIMEOUT {
                        if !captured {
                            try_release_all(pressed_keys, trigger);
                        }
                        captured = !captured;
                        last_swap = Instant::now();
                        send(HookEvent::Captured(captured));
                    }
                    return HookAction::Block;
                }
                if hotkey_pressed && key == trigger && state == KeyState::Released {
                    hotkey_pressed = false;
                    return HookAction::Block;
                }
            }
            if captured {
                if let InputEvent::MouseMoveEvent(x, y) = event {
                    let (ox, oy) = old_mouse_pos;
                    if x != ox || y != oy {
                        send(HookEvent::Input(InputEvent::MouseMoveEvent(x - ox, y - oy)));
                    }
                } else {
                    send(HookEvent::Input(event));
                }
            } else if let InputEvent::MouseMoveEvent(x, y) = event {
                old_mouse_pos = (x, y);
            }
        }

        match captured {
            true => HookAction::Block,
            false => HookAction::Continue
        }
    })
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
            }
            (true, KeyState::Released) => {
                pressed_keys.remove(event.key);
                true
            }
            _ => false
        },
        None => true
    }
}

fn try_release_all(keys: VirtualKeySet, trigger: VirtualKey) {
    send_inputs(
        keys.iter()
            .filter(|k| *k != trigger)
            .map(|k| match k.is_mouse_button() {
                true => Input::MouseButtonInput(k, KeyState::Released),
                false => Input::KeyboardKeyInput(k, KeyState::Released)
            })
    )
    .unwrap_or_else(|err| tracing::warn!("Could not send input events: {}", err));
}
