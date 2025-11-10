use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::fmt;

use loom_bridge::start_server;
use loom_core::Loom;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fmt().compact().init();

    let mut loom = Loom::new().await?;
    loom.start().await?;

    let addr: SocketAddr = std::env::var("LOOM_BRIDGE_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;
    start_server(addr, loom.event_bus.clone(), loom.action_broker.clone())
        .await
        .map_err(|e| e.into())
}
