#![windows_subsystem = "windows"]

mod model;
mod runtime;
mod sender;
mod ui;
mod utils;

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use druid::im::Vector;
use druid::{AppLauncher, EventCtx, ExtEventSink, WindowDesc};
use quinn::{ClientConfig, Connection, Endpoint, TransportConfig};
use searchlight::discovery::{Discovery, DiscoveryEvent};
use searchlight::net::IpVersion;
use tokio::select;
use tokio::time::Instant;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::InputHook;

use crate::model::{AppState, ConnectionState, PopupType, SearchResult};
use crate::runtime::{ExtEventSinkCallback, RuntimeDelegate};
use crate::sender::InputSender;
use crate::ui::widget::{theme, Theme};
use crate::utils::{hook, process_hook_event, SkipServerVerification};

pub fn main() {
    tracing_subscriber::registry()
        .with(
            Targets::new()
                .with_default(LevelFilter::DEBUG)
                .with_target("yawi", LevelFilter::TRACE)
                .with_target("inputshare_client", LevelFilter::TRACE)
                .with_target("inputshare_client::ui::widget::list", LevelFilter::DEBUG)
        )
        .with(layer().without_time())
        .init();

    #[cfg(not(debug_assertions))]
    error_tools::gui::set_gui_panic_hook();

    let window = WindowDesc::new(ui::ui())
        .window_size((450.0, 300.0))
        .title("InputShare Client");

    AppLauncher::with_window(window)
        .delegate(RuntimeDelegate::new())
        .configure_env(|env, _| theme::setup_theme(Theme::Light, env))
        .launch(AppState::default())
        .expect("launch failed");
}

fn start_search(ctx: &mut EventCtx) {
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

async fn search(ctx: ExtEventSink) -> anyhow::Result<()> {
    Discovery::builder()
        .service("_http._tcp.local.")?
        .build(IpVersion::Both)?
        .run_async(move |event| {
            ctx.add_idle_callback(move |data: &mut AppState| {
                if let Some(PopupType::Searching(results)) = &mut data.popup {
                    match event {
                        DiscoveryEvent::ResponderFound(resp) => {
                            results.push_back(SearchResult { addrs: resp.addr });
                        }
                        DiscoveryEvent::ResponderLost(_) => {}
                        DiscoveryEvent::ResponseUpdate { .. } => {}
                    }
                }
            });
        })
        .await?;
    Ok(())
}

fn initiate_connection(ctx: &mut EventCtx) {
    let handle = ctx.get_external_handle();
    ctx.add_rt_callback(move |rt, data| {
        rt.hook = None;
        if std::mem::take(&mut data.connection_state) == ConnectionState::Disconnected {
            data.connection_state = ConnectionState::Connecting;
            rt.runtime.spawn(async move {
                connection(&handle)
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

async fn connection(sink: &ExtEventSink) -> anyhow::Result<()> {
    let connection = connect().await?;
    tracing::debug!("Connected to {}", connection.remote_address());
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    sink.add_rt_callback(|rt, data| {
        rt.hook = InputHook::register(hook::create_callback(&data.config, sender))
            .map_err(|err| tracing::warn!("Failed to register hook: {}", err))
            .ok();
    });

    let mut sender = InputSender::new(1.0);
    let mut deadline = None;
    loop {
        let timeout = async move {
            match deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending().await
            };
        };
        select! {
            datagram = connection.read_datagram() => {
                let datagram: Bytes = datagram?;
                sender.read_packet(&datagram)?;
            },
            event = receiver.recv() => match event {
                Some(event) => process_hook_event(&mut sender, sink, event),
                None => break
            },
            _ = timeout => {
                let msg = sender.write_packet()?;
                debug_assert!(msg.len() <= connection.max_datagram_size().unwrap());
                connection.send_datagram(Bytes::copy_from_slice(msg))?;
                deadline = Some(Instant::now() + Duration::from_millis(10));
            }
        };
        deadline = match sender.in_sync() {
            true => None,
            false => Some(deadline.unwrap_or_else(Instant::now))
        };
    }

    tracing::trace!("Shutting down key handler");

    Ok(())
}

async fn connect() -> anyhow::Result<Connection> {
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();
    let mut transport = TransportConfig::default();
    transport.keep_alive_interval(Some(Duration::from_secs(1)));

    let mut config = ClientConfig::new(Arc::new(crypto));
    config.transport_config(Arc::new(transport));
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(config);

    let connection = endpoint
        .connect("127.0.0.1:12345".parse()?, "dummy")?
        .await?;
    Ok(connection)
}
