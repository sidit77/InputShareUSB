use std::cell::Cell;

use druid::EventCtx;
use druid::im::Vector;
use yawi::{HookAction, InputEvent, InputHook, KeyState, VirtualKey};

use crate::model::{AppState, ConnectionState, PopupType};
use crate::runtime::ExtEventSinkCallback;
use crate::{connection, search};

pub fn initiate_connection(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        rt.hook = None;
        if std::mem::take(&mut data.connection_state) == ConnectionState::Disconnected {
            data.connection_state = ConnectionState::Connecting;
            let host = data.config.host_address.clone();
            rt.runtime.spawn(async move {
                connection(&handle, &host)
                    .await
                    .unwrap_or_else(|err| tracing::warn!("could not establish connection: {}", err));
                handle.add_rt_callback(|rt, data| {
                    rt.hook = None;
                    data.connection_state = ConnectionState::Disconnected;
                });
            });
        }
    })
}

pub fn start_search(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        let task = rt.runtime.spawn(async move {
            if let Err(err) = search(handle.clone()).await {
                tracing::error!("mdns search failed: {}", err);
                handle.add_rt_callback(|rt, data| {
                    rt.mdns = None;
                    data.popup = None;
                });
            }
        });
        rt.mdns = Some(task);
        data.popup = Some(PopupType::Searching(Vector::new()))
    })
}

pub fn open_key_picker(ctx: &mut EventCtx, setter: impl FnOnce(&mut AppState, VirtualKey) + Send + 'static) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        if data.connection_state != ConnectionState::Disconnected {
            return;
        }
        assert!(rt.hook.is_none());
        let mut found = None;
        let setter = Cell::new(Some(setter));
        rt.hook = InputHook::register(move |event: InputEvent| match event.to_key_event() {
            Some(yawi::KeyEvent {
                key,
                state: KeyState::Pressed
            }) if found.is_none() || found == Some(key) => {
                found = Some(key);
                HookAction::Block
            }
            Some(yawi::KeyEvent {
                key,
                state: KeyState::Released
            }) if found == Some(key) => {
                if let Some(setter) = setter.take() {
                    handle.add_rt_callback(move |rt, data| {
                        rt.hook = None;
                        setter(data, key);
                        data.popup = None;
                    });
                }
                HookAction::Block
            }
            _ => HookAction::Continue
        })
        .map_err(|err| tracing::warn!("Could not register hook: {}", err))
        .ok();
        data.popup = Some(PopupType::PressKey);
    })
}
