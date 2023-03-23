mod receiver;
mod configfs;

use anyhow::{Context, Result};
use bytes::Bytes;
use quinn::{Connecting, ConnectionError, Endpoint, ServerConfig};
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::sync::mpsc::UnboundedSender;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::receiver::{InputEvent, InputReceiver};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(Targets::new()
            .with_default(LevelFilter::DEBUG)
            .with_target("inputshare_server::configfs", LevelFilter::DEBUG)
            .with_target("inputshare_server", LevelFilter::TRACE))
        .with(layer()
            .without_time())
        .try_init()?;

    let mut server_config = {
        let cert = rcgen::generate_simple_self_signed([String::from("inputshare")])?;
        let key = rustls::PrivateKey(cert.serialize_private_key_der());
        let cert = rustls::Certificate(cert.serialize_der()?);
        ServerConfig::with_single_cert(vec![cert], key)?
    };
    server_config.concurrent_connections(1);
    let endpoint = Endpoint::server(server_config, "0.0.0.0:12345".parse()?)?;
    tracing::debug!("Running on {}", endpoint.local_addr()?);

    select! {
        res = server(endpoint) => {
            if let Err(err) = res {
                tracing::error!("server crashed: {}", err);
            }
        }
        _ = ctrl_c() => {
            tracing::debug!("Received quit signal");
        }
    };
    Ok(())
}

async fn server(endpoint: Endpoint) -> Result<()> {
    let processor = configfs_input_processor().await?;
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
            processor.send(event)
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

async fn configfs_input_processor() -> Result<UnboundedSender<InputEvent>> {
    use configfs::*;
    let mut keyboard = Keyboard::new().await?;
    let mut mouse = Mouse::new(5.try_into()?).await?;
    let mut consumer_device = ConsumerDevice::new().await?;
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    tracing::debug!("Starting configfs processor");
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
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
                InputEvent::Reset => async {
                    keyboard.reset().await?;
                    mouse.reset().await?;
                    consumer_device.reset().await?;
                    Ok(())
                }.await,
                InputEvent::Shutdown => Ok(tracing::warn!("Shutdown is currently not supported!"))
            };
            if let Err(err) = result {
                tracing::error!("Could not write hid command: {}", err);
                break;
            }
        }
        tracing::debug!("Stopping configfs processor");
    });
    Ok(sender)
}