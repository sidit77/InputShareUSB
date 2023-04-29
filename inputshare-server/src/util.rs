use std::collections::HashMap;
use std::net::SocketAddr;

use anyhow::Result;
use mdns_sd::{DaemonEvent, Receiver, ServiceDaemon, ServiceInfo};
use quinn::ServerConfig;
use tracing::instrument;

#[instrument]
pub fn self_signed_config(concurrent_connections: u32) -> Result<ServerConfig> {
    let cert = rcgen::generate_simple_self_signed([String::from("inputshare")])?;
    let key = rustls::PrivateKey(cert.serialize_private_key_der());
    let cert = rustls::Certificate(cert.serialize_der()?);
    let mut config = ServerConfig::with_single_cert(vec![cert], key)?;
    config.concurrent_connections(concurrent_connections);
    Ok(config)
}

#[instrument]
pub fn start_mdns(interface: SocketAddr) -> Result<ServiceDaemon> {
    let mdns = ServiceDaemon::new()?;
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
    tokio::spawn(monitor_daemon(monitor));
    Ok(mdns)
}

#[instrument(skip(monitor))]
async fn monitor_daemon(monitor: Receiver<DaemonEvent>) {
    while let Ok(event) = monitor.recv_async().await {
        tracing::trace!("{:?}", &event);
    }
}

#[cfg(unix)]
pub async fn quit() {
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
pub async fn quit() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not register signals")
}
