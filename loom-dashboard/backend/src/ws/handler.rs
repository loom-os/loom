//! WebSocket Handler
//!
//! Handles WebSocket connections, processes messages, and manages
//! the bidirectional communication.

use super::message::WsMessage;
use crate::server::DashboardState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<DashboardState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<DashboardState>) {
    let connection_id = Uuid::new_v4().to_string();
    info!(connection_id = %connection_id, "New WebSocket connection");

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // Register connection
    state.connection_manager.add(connection_id.clone(), tx);

    // Task: Send messages from channel to WebSocket
    let conn_id_clone = connection_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match serde_json::to_string(&message) {
                Ok(json) => {
                    if ws_sender.send(Message::Text(json)).await.is_err() {
                        debug!(connection_id = %conn_id_clone, "Failed to send message, connection closed");
                        break;
                    }
                }
                Err(e) => {
                    error!(connection_id = %conn_id_clone, error = %e, "Failed to serialize message");
                }
            }
        }
    });

    // Task: Receive messages from WebSocket
    let conn_id_clone = connection_id.clone();
    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    debug!(connection_id = %conn_id_clone, "Received text message");
                    if let Err(e) = handle_client_message(&conn_id_clone, &text, &state_clone).await {
                        warn!(connection_id = %conn_id_clone, error = %e, "Failed to handle message");
                    }
                }
                Ok(Message::Binary(data)) => {
                    debug!(connection_id = %conn_id_clone, size = data.len(), "Received binary message");
                    // Could handle binary messages if needed
                }
                Ok(Message::Ping(_)) => {
                    debug!(connection_id = %conn_id_clone, "Received ping");
                    // Axum automatically handles pong responses
                }
                Ok(Message::Pong(_)) => {
                    debug!(connection_id = %conn_id_clone, "Received pong");
                }
                Ok(Message::Close(_)) => {
                    info!(connection_id = %conn_id_clone, "Client closed connection");
                    break;
                }
                Err(e) => {
                    warn!(connection_id = %conn_id_clone, error = %e, "WebSocket error");
                    break;
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {
            debug!(connection_id = %connection_id, "Send task completed");
        }
        _ = recv_task => {
            debug!(connection_id = %connection_id, "Receive task completed");
        }
    }

    // Cleanup
    state.connection_manager.remove(&connection_id);
    info!(connection_id = %connection_id, "WebSocket connection closed");
}

/// Handle a message from the client
async fn handle_client_message(
    connection_id: &str,
    text: &str,
    state: &DashboardState,
) -> Result<(), Box<dyn std::error::Error>> {
    let message: WsMessage = serde_json::from_str(text)?;

    match message {
        WsMessage::ChatRequest {
            thread_id,
            content,
            settings,
        } => {
            debug!(
                connection_id = %connection_id,
                thread_id = %thread_id,
                "Received chat request"
            );
            handle_chat_request(connection_id, thread_id, content, settings, state).await?;
        }

        WsMessage::PermissionResponse {
            request_id,
            approved,
            reason,
        } => {
            debug!(
                connection_id = %connection_id,
                request_id = %request_id,
                approved = approved,
                "Received permission response"
            );
            handle_permission_response(request_id, approved, reason, state).await?;
        }

        WsMessage::Subscribe { topics } => {
            debug!(
                connection_id = %connection_id,
                topics = ?topics,
                "Received subscribe request"
            );
            handle_subscribe(connection_id, topics, state).await?;
        }

        _ => {
            warn!(connection_id = %connection_id, "Unexpected message type from client");
            return Err("Unexpected message type".into());
        }
    }

    Ok(())
}

/// Handle chat request
async fn handle_chat_request(
    _connection_id: &str,
    thread_id: String,
    content: String,
    _settings: Option<super::message::ChatSettings>,
    state: &DashboardState,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Forward to backend agent via EventBus
    // For now, send a mock response
    let response = WsMessage::chat_chunk(
        thread_id.clone(),
        format!("Echo: {}", content),
        "text".to_string(),
        0,
    );

    state.connection_manager.broadcast(response);

    // Send completion
    let complete = WsMessage::chat_complete(
        thread_id,
        super::message::StreamStats {
            duration_ms: 100,
            total_tokens: 10,
            tool_calls: 0,
        },
    );

    state.connection_manager.broadcast(complete);

    Ok(())
}

/// Handle permission response
async fn handle_permission_response(
    _request_id: String,
    _approved: bool,
    _reason: Option<String>,
    _state: &DashboardState,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Forward to backend agent
    Ok(())
}

/// Handle subscribe request
async fn handle_subscribe(
    _connection_id: &str,
    _topics: Vec<String>,
    _state: &DashboardState,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement topic subscription
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_parsing() {
        let json = r#"{"type":"chat_request","thread_id":"thread-123","content":"Hello"}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();

        match msg {
            WsMessage::ChatRequest { thread_id, content, .. } => {
                assert_eq!(thread_id, "thread-123");
                assert_eq!(content, "Hello");
            }
            _ => panic!("Wrong message type"),
        }
    }
}
