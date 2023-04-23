use std::cell::Cell;
use std::net::{IpAddr, SocketAddr};

use druid::im::Vector;
use druid::{EventCtx, ExtEventSink};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};
use tracing::instrument;
use yawi::{HookAction, InputEvent, InputHook, KeyState, VirtualKey};

use crate::connection;
use crate::model::{AppState, ConnectionCommand, ConnectionState, PopupType, SearchResult};
use crate::runtime::{ExtEventSinkCallback, RuntimeDelegate};
use crate::utils::error::strip_color;

#[instrument(skip(ctx))]
pub fn initiate_connection(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| match data.connection_state {
        ConnectionState::Disconnected => {
            data.connection_state = ConnectionState::Connecting;
            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
            let host = data.config.host_address.clone();
            let info = data.config.show_network_info;
            let rate = data.config.network_send_rate as f32;
            rt.runtime.spawn(async move {
                let result = connection(&handle, receiver, &host, info, rate).await;
                handle.add_rt_callback(|rt, data| {
                    rt.hook = None;
                    rt.connection = None;
                    data.connection_state = ConnectionState::Disconnected;
                    data.enable_shutdown = false;
                    data.network_info = None;
                    if let Err(err) = result {
                        tracing::warn!("could not establish connection: {:?}", err);
                        open_popup(rt, data, PopupType::Error(strip_color(&format!("{:?}", err))));
                    }
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

#[instrument(skip(ctx))]
pub fn shutdown_server(ctx: &mut EventCtx) {
    ctx.add_rt_callback(|rt, data| {
        if !data.enable_shutdown {
            tracing::warn!("Shutdown functions is currently not enabled");
            return;
        }
        rt.connection
            .as_ref()
            .and_then(|sender| sender.send(ConnectionCommand::ShutdownServer).ok())
            .unwrap_or_else(|| tracing::warn!("Failed to send shutdown signal!"));
    });
}

#[instrument(skip(ctx))]
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
                    open_popup(rt, data, PopupType::Searching(Vector::new()));
                    rt.runtime.spawn(update_popup(receiver, handle));
                }
                Err(err) => {
                    tracing::warn!("Could not browse: {}", err);
                    let service = Some(mdns);
                    if let Some(t) = service {
                        t.shutdown()
                            .unwrap_or_else(|err| tracing::warn!("Error shutting down mdns service: {}", err))
                    }
                }
            }
        }
    })
}

#[instrument(skip(ctx, receiver))]
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

#[instrument(skip(ctx, setter))]
pub fn open_key_picker(ctx: &mut EventCtx, setter: impl FnOnce(&mut AppState, VirtualKey) + Send + 'static) {
    let handle = ctx.get_external_handle();
    let span = tracing::Span::current();
    ctx.add_rt_callback(move |rt, data| {
        let _enter = span.enter();
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
                        setter(data, key);
                        close_popup(rt, data);
                    });
                }
                HookAction::Block
            }
            _ => HookAction::Continue
        })
        .map_err(|err| tracing::warn!("Could not register hook: {}", err))
        .ok();
        open_popup(rt, data, PopupType::PressKey);
    })
}

#[instrument(skip(rt, data))]
pub fn close_popup(rt: &mut RuntimeDelegate, data: &mut AppState) {
    match data.popup.take() {
        Some(PopupType::PressKey) => {
            rt.hook = None;
        }
        Some(PopupType::Searching(_)) => {
            let service = rt.mdns.take();
            if let Some(t) = service {
                t.shutdown()
                    .unwrap_or_else(|err| tracing::warn!("Error shutting down mdns service: {}", err))
            }
        }
        _ => {}
    }
}

#[instrument(skip(rt, data))]
pub fn open_popup(rt: &mut RuntimeDelegate, data: &mut AppState, popup: PopupType) {
    if data.popup.is_some() {
        tracing::warn!("Another popup is already open");
        close_popup(rt, data);
    }
    data.popup = Some(popup);
}
