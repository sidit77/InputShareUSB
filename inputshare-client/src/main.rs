#![windows_subsystem = "windows"]

mod model;
mod runtime;
mod sender;
mod ui;
mod utils;

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use druid::{AppLauncher, ExtEventSink, WindowDesc};
use eyre::{eyre, WrapErr};
use quinn::{ClientConfig, Connection, Endpoint, TransportConfig};
use tokio::{select, spawn};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::Instant;
use tracing::instrument;
use tracing_error::ErrorLayer;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use yawi::InputHook;

use crate::model::{AppState, ConnectionCommand, NetworkInfo};
use crate::runtime::{ExtEventSinkCallback, RuntimeDelegate};
use crate::sender::InputSender;
use crate::ui::widget::{theme, Theme};
use crate::utils::error::set_eyre_hook;
use crate::utils::{hook, process_hook_event, SkipServerVerification};

#[instrument]
pub fn main() {
    set_eyre_hook();
    tracing_subscriber::registry()
        .with(
            Targets::new()
                .with_default(LevelFilter::DEBUG)
                .with_target("yawi", LevelFilter::TRACE)
                .with_target("inputshare_client", LevelFilter::TRACE)
                .with_target("inputshare_client::ui::widget::list", LevelFilter::DEBUG)
                .with_target("mdns_sd", LevelFilter::INFO)
        )
        .with(layer().without_time())
        .with(ErrorLayer::default())
        .init();

    #[cfg(not(debug_assertions))]
    error_tools::gui::set_gui_panic_hook();

    let window = WindowDesc::new(ui::ui())
        .window_size((400.0, 400.0))
        .title("InputShare Client");

    AppLauncher::with_window(window)
        .delegate(RuntimeDelegate::new())
        .configure_env(|env, _| theme::setup_theme(Theme::Light, env))
        .launch(AppState::default())
        .expect("launch failed");
}

#[instrument(skip(sink, controller))]
async fn connection(sink: &ExtEventSink, mut controller: UnboundedReceiver<ConnectionCommand>, host: &str, info: bool) -> eyre::Result<()> {
    let wait = async {
        loop {
            match controller.recv().await {
                None => return Err(eyre!("control channel closed")),
                Some(ConnectionCommand::ShutdownServer) => tracing::warn!("Can not send a shutdown signal until connected"),
                Some(ConnectionCommand::Disconnect) => {
                    tracing::debug!("Canceling connection");
                    return Ok(());
                }
            }
        }
    };
    let connection: Connection = select! {
        conn = connect(host) => conn?,
        res = wait => return res
    };
    tracing::debug!("Connected to {}", connection.remote_address());

    if info {
        spawn(collect_network_info(connection.clone(), sink.clone()));
    }

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    sink.add_rt_callback(|rt, data| {
        tracing::info_span!("register input hook").in_scope(move || {
            if rt.hook.is_some() {
                tracing::warn!("Hook already exists");
            }
            rt.hook = InputHook::register(hook::create_callback(&data.config, sender))
                .map_err(|err| tracing::warn!("Failed to register hook: {}", err))
                .ok();
            data.enable_shutdown = true;
        })
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
                None => return Err(eyre!("Input hook got removed"))
            },
            cmd = controller.recv() => match cmd {
                None => return Err(eyre!("control channel got removed")),
                Some(ConnectionCommand::Disconnect) => break,
                Some(ConnectionCommand::ShutdownServer) => sender.shutdown_remote()
            },
            _ = timeout => {
                let msg = sender.write_packet()?;
                debug_assert!(msg.len() <= connection.max_datagram_size().unwrap());
                connection.send_datagram(Bytes::copy_from_slice(msg))?;
                deadline = Some(Instant::now() + Duration::from_millis(10));
                //tracing::debug!("stats: {:#?}", connection.stats().path);
            }
        };
        deadline = match sender.in_sync() {
            true => None,
            false => Some(deadline.unwrap_or_else(Instant::now))
        };
    }

    connection.close(0u8.into(), b"Disconnected");
    tracing::trace!("Shutting down key handler");

    Ok(())
}

#[instrument]
async fn connect(host: &str) -> eyre::Result<Connection> {
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

    let addrs = host
        .to_socket_addrs()
        .wrap_err("Is the used host name correct?")?
        .find(|a| a.is_ipv4())
        .ok_or_else(|| eyre!("Can not find suitable address"))?;
    tracing::debug!("Resolved {} to {}", host, addrs);
    let connection = endpoint.connect(addrs, "dummy")?.await?;
    Ok(connection)
}

#[instrument(skip(connection, sink))]
async fn collect_network_info(connection: Connection, sink: ExtEventSink) {
    while connection.close_reason().is_none() {
        let path = connection.stats().path;
        let info = NetworkInfo {
            rtt: path.rtt,
            cwnd: path.cwnd,
            congestion_events: path.congestion_events,
            lost_packets: path.lost_packets,
            lost_bytes: path.lost_bytes,
            sent_packets: path.sent_packets,
        };
        sink.add_idle_callback(move |data: &mut AppState| {
            data.network_info = Some(info);
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    sink.add_idle_callback(move |data: &mut AppState| {
        data.network_info = None;
    });
}