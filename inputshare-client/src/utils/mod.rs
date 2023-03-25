use std::sync::Arc;
use druid::ExtEventSink;
use yawi::{InputEvent, KeyState, ScrollDirection};
use crate::model::{AppState, ConnectionState, Side};
use crate::sender::InputSender;
use crate::utils::conversions::{f32_to_i8, vk_to_mb, wsc_to_cdc, wsc_to_hkc};
use crate::utils::hook::HookEvent;

pub mod conversions;
pub mod hook;

pub fn process_hook_event(sender: &mut InputSender, sink: &ExtEventSink, event: HookEvent) {
    match event {
        HookEvent::Captured(captured) => {
            sink.add_idle_callback(move |data: &mut AppState| {
                data.connection_state = ConnectionState::Connected(match captured {
                    true => Side::Remote,
                    false => Side::Local
                });
            });
            sender.reset();
        },
        HookEvent::Input(event) => match event {
            InputEvent::MouseMoveEvent(x, y) => {
                sender.move_mouse(x as i64, y as i64);
            }
            InputEvent::KeyboardKeyEvent(vk, sc, ks) => match wsc_to_hkc(sc) {
                Some(kc) => match ks {
                    KeyState::Pressed => sender.press_key(kc),
                    KeyState::Released => sender.release_key(kc)
                },
                None => match wsc_to_cdc(sc){
                    Some(cdc) => match ks {
                        KeyState::Pressed => sender.press_consumer_device(cdc),
                        KeyState::Released => sender.release_consumer_device(cdc)
                    },
                    None => if! matches!(sc, 0x21d) {
                        tracing::warn!("Unknown key: {} ({:x})", vk, sc)
                    }
                }
            }
            InputEvent::MouseButtonEvent(mb, ks) => match vk_to_mb(mb) {
                Some(button) => match ks {
                    KeyState::Pressed => sender.press_mouse_button(button),
                    KeyState::Released => sender.release_mouse_button(button)
                },
                None => tracing::warn!("Unknown mouse button: {}", mb)
            }
            InputEvent::MouseWheelEvent(sd) => match sd {
                ScrollDirection::Horizontal(amount) => sender.scroll_horizontal(f32_to_i8(amount)),
                ScrollDirection::Vertical(amount) => sender.scroll_vertical(f32_to_i8(amount))
            }
        }
    }
}

pub struct SkipServerVerification;

impl SkipServerVerification {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}