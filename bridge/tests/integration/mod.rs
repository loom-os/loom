//! Bridge Integration Test Module

use std::net::SocketAddr;
use std::sync::Arc;

use loom_bridge::BridgeService;
use loom_core::{AgentDirectory, EventBus, ToolRegistry};
use loom_proto::bridge_server::BridgeServer;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

pub use loom_proto::{
    bridge_client::BridgeClient, client_event, server_event, Ack, AgentRegisterRequest,
    ClientEvent, Delivery, Event, HeartbeatRequest, HeartbeatResponse, Publish, ToolCall,
    ToolResult, ToolStatus,
};

/// Start a Bridge gRPC server on an ephemeral localhost port and return the bound address
pub async fn start_test_server(
    event_bus: Arc<EventBus>,
    tool_registry: Arc<ToolRegistry>,
) -> (
    SocketAddr,
    tokio::task::JoinHandle<()>,
    loom_bridge::BridgeService,
) {
    let agent_directory = Arc::new(AgentDirectory::new());
    let svc = loom_bridge::BridgeService::new(loom_bridge::BridgeState::new(
        Arc::clone(&event_bus),
        Arc::clone(&tool_registry),
        agent_directory,
    ));
    let svc_for_return = svc.clone();

    // Bind to 127.0.0.1:0 for an ephemeral port
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(listener);

    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(BridgeServer::new(svc))
            .serve_with_incoming(incoming)
            .await
            .expect("server exited cleanly");
    });

    (addr, handle, svc_for_return)
}

/// Create a new Bridge client connected to the given address
pub async fn new_client(addr: SocketAddr) -> BridgeClient<tonic::transport::Channel> {
    let endpoint = format!("http://{}", addr);
    BridgeClient::connect(endpoint)
        .await
        .expect("connect client")
}

mod e2e_basic;
mod e2e_forward_action;
mod e2e_server_push;
