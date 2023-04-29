mod configfs;
mod receiver;
mod util;

use std::future::pending;
use std::num::NonZeroU8;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use bytes::Bytes;
use clap::{arg, command, Parser};
use mdns_sd::Error;
use quinn::{Connecting, ConnectionError, Endpoint};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio::{select, spawn};
use tracing::{instrument, Instrument, Span};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::receiver::{InputEvent, InputReceiver};
use crate::util::{quit, self_signed_config, start_mdns};

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
    console: bool,

    /// Disabled the mDNS service that is use for service discovery
    #[arg(short, long)]
    no_mdns: bool
}

#[instrument]
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
    let interface = args.interface.parse()?;
    tracing::debug!("Attempting to bind {}", interface);
    let endpoint = Endpoint::server(self_signed_config(1)?, interface)?;

    let mdns = match args.no_mdns {
        true => None,
        false => start_mdns(interface)
            .map_err(|err| tracing::error!("Failed to start mdns service: {}\n{}", err, err.backtrace()))
            .ok()
    };

    spawn({
        let endpoint = endpoint.clone();
        async move {
            quit().await;
            tracing::debug!("Received quit signal");
            endpoint.close(0u8.into(), b"Server shutting down");
        }
    });

    let processor = match args.console {
        true => log_input_processor().await?,
        false => configfs_input_processor(args.mouse_tesselation_factor.try_into()?, args.auto_movement_timeout).await?
    };

    while let Some(conn) = endpoint.accept().await {
        let processor = processor.clone();
        spawn(async move {
            handle_connection(processor, conn)
                .await
                .unwrap_or_else(|err| tracing::error!("Connection crashed!\n{:?}", err))
        });
    }
    drop(processor);
    tracing::debug!("Stopping server");

    if let Some(mdns) = mdns {
        tracing::debug!("Stopping mDNS service");
        while let Err(Error::Again) = mdns.shutdown() {
            sleep(Duration::from_millis(50)).await;
        }
    }
    Ok(())
}

#[instrument(skip_all, fields(addrs = tracing::field::Empty))]
async fn handle_connection(processor: UnboundedSender<InputEvent>, connecting: Connecting) -> Result<()> {
    let connection = connecting.await?;
    let span = Span::current();
    span.record("addrs", connection.remote_address().to_string());
    tracing::debug!("Established connection");
    let mut receiver = InputReceiver::new();

    loop {
        let msg = match connection.read_datagram().await {
            Ok(msg) => msg,
            Err(ConnectionError::ApplicationClosed(close)) => {
                tracing::debug!("Connection closed: {}", close);
                return Ok(());
            }
            Err(ConnectionError::LocallyClosed) => {
                tracing::debug!("Closing Connection");
                return Ok(());
            }
            Err(err) => return Err(err.into())
        };
        if let Some(packet) = receiver.process_packet(&msg)? {
            ensure!(packet.len() <= connection.max_datagram_size().unwrap());
            connection.send_datagram(Bytes::copy_from_slice(packet))?;
        }
        while let Some(event) = receiver.get_event() {
            processor
                .send(event)
                .context("The input processor seems to be gone")?;
        }
    }
}

#[instrument]
async fn log_input_processor() -> Result<UnboundedSender<InputEvent>> {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    tracing::debug!("Starting print processor");
    spawn(
        async move {
            while let Some(event) = receiver.recv().await {
                tracing::info!("New input event: {:?}", event);
            }
            tracing::debug!("Stopping print processor");
        }
        .instrument(Span::current())
    );
    Ok(sender)
}

#[instrument]
async fn configfs_input_processor(tess_factor: NonZeroU8, timeout: Option<u64>) -> Result<UnboundedSender<InputEvent>> {
    use configfs::*;
    let mut keyboard = Keyboard::new().await?;
    let mut mouse = Mouse::new(tess_factor).await?;
    let mut consumer_device = ConsumerDevice::new().await?;
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    tracing::debug!("Starting configfs processor");
    spawn(
        async move {
            let mut idle_move_x = -10;
            loop {
                let timeout = async move {
                    match timeout {
                        Some(timeout) => sleep(Duration::from_secs(timeout)).await,
                        None => pending().await
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
                                },
                                InputEvent::Shutdown => run_command("shutdown", &["-r", "now"]).await,
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
        }
        .instrument(Span::current())
    );
    Ok(sender)
}

#[instrument]
async fn run_command(command: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(command).args(args).output().await?;
    if output.status.success() && output.stderr.is_empty() {
        return Ok(());
    }
    bail!("{}", String::from_utf8_lossy(&output.stderr));
}
