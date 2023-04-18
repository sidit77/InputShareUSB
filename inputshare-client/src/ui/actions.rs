use std::cell::Cell;
use std::net::{IpAddr, SocketAddr};

use druid::im::Vector;
use druid::{EventCtx, ExtEventSink};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};
use yawi::{HookAction, InputEvent, InputHook, KeyState, VirtualKey};

use crate::connection;
use crate::model::{AppState, ConnectionCommand, ConnectionState, PopupType, SearchResult};
use crate::runtime::ExtEventSinkCallback;

pub fn initiate_connection(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| match data.connection_state {
        ConnectionState::Disconnected => {
            data.connection_state = ConnectionState::Connecting;
            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
            let host = data.config.host_address.clone();
            rt.runtime.spawn(async move {
                connection(&handle, receiver, &host)
                    .await
                    .unwrap_or_else(|err| tracing::warn!("could not establish connection: {}", err));
                handle.add_rt_callback(|rt, data| {
                    rt.hook = None;
                    rt.connection = None;
                    data.connection_state = ConnectionState::Disconnected;
                });
            });
            rt.connection = Some(sender);
        }
        _ => match rt.connection.as_ref() {
            None => tracing::warn!("Connection control channel missing"),
            Some(channel) => channel
                .send(ConnectionCommand::Disconnect)
                .unwrap_or_else(|err| tracing::warn!("Can not send command: {}", err))
        }
    })
}

pub fn start_search(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        let mdns = ServiceDaemon::new()
            .map_err(|err| tracing::warn!("Could not start mdns service: {}", err))
            .ok();
        if let Some(mdns) = mdns {
            match mdns.browse("_inputshare._udp.local.") {
                Ok(receiver) => {
                    rt.mdns = Some(mdns);
                    data.popup = Some(PopupType::Searching(Vector::new()));
                    rt.runtime.spawn(update_popup(receiver, handle));
                }
                Err(err) => {
                    tracing::warn!("Could not browse: {}", err);
                    stop_service(Some(mdns));
                }
            }
        }
    })
}

async fn update_popup(receiver: Receiver<ServiceEvent>, ctx: ExtEventSink) {
    while let Ok(event) = receiver.recv_async().await {
        if let ServiceEvent::ServiceResolved(info) = event {
            ctx.add_idle_callback(move |data: &mut AppState| {
                if let Some(PopupType::Searching(list)) = &mut data.popup {
                    for addrs in info.get_addresses() {
                        let addrs = SocketAddr::new(IpAddr::V4(*addrs), info.get_port());
                        list.push_back(SearchResult { addrs });
                    }
                }
            });
        }
    }
    tracing::trace!("Search popup is no longer updated");
}

pub fn stop_service(service: Option<ServiceDaemon>) {
    if let Some(t) = service {
        t.shutdown()
            .unwrap_or_else(|err| tracing::warn!("Error shutting down mdns service: {}", err))
    }
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
