//! WebSocket Integration Tests
//!
//! Tests WebSocket connection, message flow, and EventBus integration

use futures_util::{SinkExt, StreamExt};
use loom_core::agent::directory::AgentDirectory;
use loom_core::messaging::event_bus::EventBus;
use loom_dashboard::{DashboardConfig, DashboardServer, WsMessage};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Start a test server and return the WebSocket URL
async fn start_test_server(port: u16) -> String {
    // Use a different port for each test to avoid conflicts
    let config = DashboardConfig {
        server: loom_dashboard::config::ServerConfig {
            host: "127.0.0.1".to_string(),
            port,
        },
        ..Default::default()
    };

    let event_bus = Arc::new(EventBus::new().await.unwrap());
    let agent_directory = Arc::new(AgentDirectory::new());

    let server = DashboardServer::new(config.clone(), event_bus, agent_directory);

    // Start server in background
    tokio::spawn(async move {
        let _ = server.serve().await;
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    format!("ws://{}:{}/ws", config.server.host, config.server.port)
}

#[tokio::test]
async fn test_websocket_connection() {
    let ws_url = start_test_server(3031).await;

    // Connect to WebSocket
    let result = timeout(
        Duration::from_secs(2),
        connect_async(&ws_url)
    ).await;

    assert!(result.is_ok(), "Failed to connect to WebSocket");
    let (ws_stream, _) = result.unwrap().unwrap();

    // Close connection
    let (mut write, _read) = ws_stream.split();
    write.close().await.unwrap();
}

#[tokio::test]
async fn test_websocket_chat_request() {
    let ws_url = start_test_server(3032).await;

    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send a chat request
    let chat_request = WsMessage::ChatRequest {
        thread_id: "test-thread".to_string(),
        content: "Hello, world!".to_string(),
        settings: None,
    };

    let json = serde_json::to_string(&chat_request).unwrap();
    write.send(Message::Text(json)).await.unwrap();

    // Expect to receive chat chunk response
    let msg_result = timeout(Duration::from_secs(2), read.next()).await;
    assert!(msg_result.is_ok(), "Timeout waiting for response");

    let msg = msg_result.unwrap().unwrap().unwrap();
    if let Message::Text(text) = msg {
        let response: WsMessage = serde_json::from_str(&text).unwrap();
        match response {
            WsMessage::ChatChunk { thread_id, .. } => {
                assert_eq!(thread_id, "test-thread");
            }
            _ => panic!("Expected ChatChunk response"),
        }
    }

    write.close().await.unwrap();
}

#[tokio::test]
async fn test_websocket_multiple_connections() {
    let ws_url = start_test_server(3033).await;

    // Open multiple connections
    let mut connections = Vec::new();
    for _ in 0..3 {
        let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
        connections.push(ws_stream);
    }

    // All connections should be active
    assert_eq!(connections.len(), 3);

    // Close all connections
    for ws_stream in connections {
        let (mut write, _read) = ws_stream.split();
        write.close().await.unwrap();
    }
}

#[tokio::test]
async fn test_websocket_ping_pong() {
    let ws_url = start_test_server(3034).await;

    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut write, _read) = ws_stream.split();

    // Send ping (axum automatically responds with pong)
    // Just verify the connection stays alive
    write.send(Message::Ping(vec![1, 2, 3])).await.unwrap();

    // Send another ping to verify connection is still alive
    tokio::time::sleep(Duration::from_millis(50)).await;
    let result = write.send(Message::Ping(vec![4, 5, 6])).await;
    assert!(result.is_ok(), "Connection should be alive after ping");

    write.close().await.unwrap();
}

#[tokio::test]
async fn test_websocket_invalid_message() {
    let ws_url = start_test_server(3035).await;

    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send invalid JSON
    write.send(Message::Text("not valid json".to_string())).await.unwrap();

    // Connection should remain open despite error
    // Send valid message
    let ping_msg = WsMessage::ChatRequest {
        thread_id: "test".to_string(),
        content: "test".to_string(),
        settings: None,
    };
    let json = serde_json::to_string(&ping_msg).unwrap();
    write.send(Message::Text(json)).await.unwrap();

    // Should still receive response
    let msg_result = timeout(Duration::from_secs(1), read.next()).await;
    assert!(msg_result.is_ok(), "Connection closed after invalid message");

    write.close().await.unwrap();
}
