mod receiver;

use anyhow::Result;
use bytes::Bytes;
use quinn::{Endpoint, ServerConfig};
use tokio::select;
use tokio::signal::ctrl_c;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::receiver::InputReceiver;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(Targets::new()
            .with_default(LevelFilter::DEBUG)
            .with_target("inputshare_server", LevelFilter::TRACE))
        .with(layer()
            .without_time())
        .try_init()?;

    let server_config = {
        let cert = rcgen::generate_simple_self_signed([String::from("inputshare")])?;
        let key = rustls::PrivateKey(cert.serialize_private_key_der());
        let cert = rustls::Certificate(cert.serialize_der()?);
        ServerConfig::with_single_cert(vec![cert], key)?
    };
    let endpoint = Endpoint::server(server_config, "0.0.0.0:12345".parse()?)?;
    tracing::debug!("Running on {}", endpoint.local_addr()?);

    select! {
        res = server(endpoint) => {
            tracing::warn!("Server function returned: {:?}", res);
        }
        _ = ctrl_c() => {
            tracing::debug!("Received quit signal");
        }
    };
    tracing::trace!("End of main function");
    Ok(())
}

async fn server(endpoint: Endpoint) -> Result<()> {
    while let Some(conn) = endpoint.accept().await {
        let connection = conn.await?;
        tracing::debug!("Got connection from {}", connection.remote_address());
        let mut receiver = InputReceiver::new();

        loop {
            let msg = connection.read_datagram().await?;
            if let Some(packet) = receiver.process_packet(&msg)? {
                debug_assert!(packet.len() <= connection.max_datagram_size().unwrap());
                connection.send_datagram(Bytes::copy_from_slice(packet))?;
            }
            while let Some(event) = receiver.get_event() {
                tracing::trace!("Network packet: {:?}", event);
            }
        }

        //while let Ok(recv) = connection.accept_uni().await {
        //    tracing::info!("{}", String::from_utf8_lossy(&recv.read_to_end(300).await?));
        //}
        //tracing::debug!("Connection closed: {}", connection.closed().await)
    }
    Ok(())
}