use anyhow::Result;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(Targets::new()
            .with_default(LevelFilter::TRACE))
        .with(layer()
            .without_time())
        .try_init()?;
    tracing::trace!("Test");
    tokio::signal::ctrl_c().await?;
    tracing::trace!("end");
    Ok(())
}