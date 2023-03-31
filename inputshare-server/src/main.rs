mod configfs;
mod receiver;

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use bytes::Bytes;
use clap::{arg, command, Parser};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use quinn::{Connecting, ConnectionError, Endpoint, ServerConfig};
use tokio::select;
use tokio::sync::mpsc::UnboundedSender;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::receiver::{InputEvent, InputReceiver};

/// The server for inputshare
#[derive(Parser, Debug)]
#[command(about, version, author)]
struct Args {
    /// When set automatically moves the mouse every x seconds without input
    #[arg(short, long)]
    auto_movement_timeout: Option<u64>,

    /// Split each mouse movement command in up to x usb packets
    /// Higher values mean smoother movement but carry a higher risk of saturating the usb connection
    #[arg(short, long, default_value_t = 5)]
    mouse_tesselation_factor: u8,

    /// The interface that should be bound
    #[arg(short, long, default_value = "0.0.0.0:60067")]
    interface: String,

    /// Replace the actual HID emulation with a simple console logger
    /// Useful for debugging or testing on windows
    #[arg(short, long)]
    console: bool
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            Targets::new()
                .with_default(LevelFilter::DEBUG)
                .with_target("inputshare_server::configfs", LevelFilter::DEBUG)
                .with_target("inputshare_server", LevelFilter::TRACE)
                .with_target("mdns_sd", LevelFilter::INFO)
        )
        .with(layer().without_time())
        .try_init()?;

    let args = Args::parse();

    let mut server_config = {
        let cert = rcgen::generate_simple_self_signed([String::from("inputshare")])?;
        let key = rustls::PrivateKey(cert.serialize_private_key_der());
        let cert = rustls::Certificate(cert.serialize_der()?);
        ServerConfig::with_single_cert(vec![cert], key)?
    };
    server_config.concurrent_connections(1);
    let interface = args.interface.parse()?;
    tracing::debug!("Attempting to bind {}", interface);
    let endpoint = Endpoint::server(server_config, interface)?;

    let mdns = ServiceDaemon::new()?;
    {
        let service_info = ServiceInfo::new(
            "_inputshare._udp.local.",
            "InputShare Server",
            "inputshare.local.",
            "",
            interface.port(),
            HashMap::new()
        )?
        .enable_addr_auto();
        mdns.register(service_info)?;
        let monitor = mdns.monitor()?;
        tokio::spawn(async move {
            while let Ok(event) = monitor.recv_async().await {
                tracing::trace!("ndms daemon event: {:?}", &event);
            }
        });
    }

    select! {
        res = server(endpoint, args) => {
            if let Err(err) = res {
                tracing::error!("server crashed: {}", err);
            }
        }
        _ = quit() => {
            tracing::debug!("Received quit signal");
        }
    };
    mdns.shutdown()?;
    Ok(())
}

#[cfg(unix)]
async fn quit() {
    use tokio::signal::unix::*;
    let mut quit = signal(SignalKind::quit()).expect("Could not register signal");
    let mut interrupt = signal(SignalKind::interrupt()).expect("Could not register signal");
    let mut terminate = signal(SignalKind::terminate()).expect("Could not register signal");
    select! {
        _ = quit.recv() => { }
        _ = interrupt.recv() => { }
        _ = terminate.recv() => { }
    }
}

#[cfg(not(unix))]
async fn quit() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not register signals")
}

async fn server(endpoint: Endpoint, args: Args) -> Result<()> {
    let processor = match args.console {
        true => log_input_processor().await?,
        false => configfs_input_processor(args).await?
    };
    while let Some(conn) = endpoint.accept().await {
        let processor = processor.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(processor, conn).await {
                tracing::error!("connection crashed with error:\n{:?}", e);
            }
        });
    }
    Ok(())
}

async fn handle_connection(processor: UnboundedSender<InputEvent>, connection: Connecting) -> Result<()> {
    let connection = connection.await?;
    tracing::debug!("Got connection from {}", connection.remote_address());
    let mut receiver = InputReceiver::new();

    loop {
        let msg = match connection.read_datagram().await {
            Ok(msg) => msg,
            Err(ConnectionError::ApplicationClosed(close)) => {
                tracing::debug!("Connection closed: {}", close);
                return Ok(());
            }
            Err(err) => return Err(err.into())
        };
        if let Some(packet) = receiver.process_packet(&msg)? {
            debug_assert!(packet.len() <= connection.max_datagram_size().unwrap());
            connection.send_datagram(Bytes::copy_from_slice(packet))?;
        }
        while let Some(event) = receiver.get_event() {
            processor
                .send(event)
                .context("The input processor seems to be gone")?;
        }
    }
}

async fn log_input_processor() -> Result<UnboundedSender<InputEvent>> {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    tracing::debug!("Starting print processor");
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            tracing::info!("New input event: {:?}", event);
        }
        tracing::debug!("Stopping print processor");
    });
    Ok(sender)
}

async fn configfs_input_processor(args: Args) -> Result<UnboundedSender<InputEvent>> {
    use configfs::*;
    let mut keyboard = Keyboard::new().await?;
    let mut mouse = Mouse::new(args.mouse_tesselation_factor.try_into()?).await?;
    let mut consumer_device = ConsumerDevice::new().await?;
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    tracing::debug!("Starting configfs processor");
    tokio::spawn(async move {
        let mut idle_move_x = -10;
        loop {
            let timeout = args.auto_movement_timeout;
            let timeout = async move {
                match timeout {
                    Some(timeout) => tokio::time::sleep(Duration::from_secs(timeout)).await,
                    None => std::future::pending().await
                };
            };
            select! {
                event = receiver.recv() => match event {
                    Some(event) => {
                        let result = match event {
                            InputEvent::MouseMove(x, y) => mouse.move_by(x as i16, y as i16).await,
                            InputEvent::KeyPress(key) => keyboard.press_key(key).await,
                            InputEvent::KeyRelease(key) => keyboard.release_key(key).await,
                            InputEvent::MouseButtonPress(button) => mouse.press_button(button).await,
                            InputEvent::MouseButtonRelease(button) => mouse.release_button(button).await,
                            InputEvent::ConsumerDevicePress(button) => consumer_device.press_key(button).await,
                            InputEvent::ConsumerDeviceRelease(button) => consumer_device.release_key(button).await,
                            InputEvent::HorizontalScrolling(amount) => mouse.scroll_horizontal(amount).await,
                            InputEvent::VerticalScrolling(amount) => mouse.scroll_vertical(amount).await,
                            InputEvent::Reset => {
                                async {
                                    keyboard.reset().await?;
                                    mouse.reset().await?;
                                    consumer_device.reset().await?;
                                    Ok(())
                                }
                                .await
                            }
                            InputEvent::Shutdown => Ok(tracing::warn!("Shutdown is currently not supported!"))
                        };
                        if let Err(err) = result {
                            tracing::error!("Could not write hid command: {}", err);
                            break;
                        }
                    },
                    None => break
                },
                _ = timeout => {
                    if let Err(err) = mouse.move_by(idle_move_x, 0).await {
                         tracing::error!("Could not write hid command: {}", err);
                         break;
                    }
                    idle_move_x *= -1;
                }
            };
        }
        tracing::debug!("Stopping configfs processor");
    });
    Ok(sender)
}
