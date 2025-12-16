//! WebSocket Connection Manager
//!
//! Manages active WebSocket connections, broadcasts messages,
//! and handles connection lifecycle.

use super::message::WsMessage;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// WebSocket connection handle
pub struct Connection {
    /// Unique connection ID
    pub id: String,
    /// Message sender channel
    pub tx: mpsc::UnboundedSender<WsMessage>,
}

/// WebSocket connection manager
#[derive(Clone)]
pub struct ConnectionManager {
    /// Active connections
    connections: Arc<DashMap<String, Connection>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    /// Register a new connection
    pub fn add(&self, id: String, tx: mpsc::UnboundedSender<WsMessage>) {
        debug!(connection_id = %id, "Registering WebSocket connection");
        self.connections.insert(id.clone(), Connection { id, tx });
    }

    /// Remove a connection
    pub fn remove(&self, id: &str) {
        debug!(connection_id = %id, "Removing WebSocket connection");
        self.connections.remove(id);
    }

    /// Get connection count
    pub fn count(&self) -> usize {
        self.connections.len()
    }

    /// Send a message to a specific connection
    pub fn send_to(&self, id: &str, message: WsMessage) -> Result<(), SendError> {
        if let Some(conn) = self.connections.get(id) {
            conn.tx
                .send(message)
                .map_err(|_| SendError::ConnectionClosed)?;
            Ok(())
        } else {
            Err(SendError::NotFound)
        }
    }

    /// Broadcast a message to all connections
    pub fn broadcast(&self, message: WsMessage) {
        let count = self.connections.len();
        debug!(connection_count = count, "Broadcasting message to all connections");

        let mut failed = 0;
        for conn in self.connections.iter() {
            if conn.tx.send(message.clone()).is_err() {
                failed += 1;
            }
        }

        if failed > 0 {
            warn!(failed_count = failed, "Some broadcasts failed");
        }
    }

    /// Broadcast to connections matching a predicate
    pub fn broadcast_filtered<F>(&self, message: WsMessage, predicate: F)
    where
        F: Fn(&str) -> bool,
    {
        for conn in self.connections.iter() {
            if predicate(&conn.id) {
                let _ = conn.tx.send(message.clone());
            }
        }
    }

    /// Get all connection IDs
    pub fn connection_ids(&self) -> Vec<String> {
        self.connections
            .iter()
            .map(|entry| entry.id.clone())
            .collect()
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Send error types
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// Connection not found
    #[error("Connection not found")]
    NotFound,

    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_manager_add_remove() {
        let manager = ConnectionManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        manager.add("conn-1".to_string(), tx);
        assert_eq!(manager.count(), 1);

        manager.remove("conn-1");
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_send_to_existing_connection() {
        let manager = ConnectionManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        manager.add("conn-1".to_string(), tx);

        let msg = WsMessage::error("TEST", "Test message");
        manager.send_to("conn-1", msg.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            WsMessage::Error { code, .. } => assert_eq!(code, "TEST"),
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_connection() {
        let manager = ConnectionManager::new();
        let msg = WsMessage::error("TEST", "Test message");

        let result = manager.send_to("nonexistent", msg);
        assert!(matches!(result, Err(SendError::NotFound)));
    }

    #[tokio::test]
    async fn test_broadcast() {
        let manager = ConnectionManager::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        manager.add("conn-1".to_string(), tx1);
        manager.add("conn-2".to_string(), tx2);

        let msg = WsMessage::error("BROADCAST", "Test broadcast");
        manager.broadcast(msg);

        // Both should receive
        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();

        match (msg1, msg2) {
            (WsMessage::Error { code: c1, .. }, WsMessage::Error { code: c2, .. }) => {
                assert_eq!(c1, "BROADCAST");
                assert_eq!(c2, "BROADCAST");
            }
            _ => panic!("Wrong message types"),
        }
    }

    #[tokio::test]
    async fn test_connection_ids() {
        let manager = ConnectionManager::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();

        manager.add("conn-1".to_string(), tx1);
        manager.add("conn-2".to_string(), tx2);

        let ids = manager.connection_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"conn-1".to_string()));
        assert!(ids.contains(&"conn-2".to_string()));
    }
}
