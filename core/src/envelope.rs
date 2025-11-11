use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::proto::{ActionCall, Event};

/// Reserved metadata/header keys for thread & correlation semantics
pub mod keys {
    pub const THREAD_ID: &str = "thread_id";
    pub const CORRELATION_ID: &str = "correlation_id";
    pub const SENDER: &str = "sender";
    pub const REPLY_TO: &str = "reply_to";
    pub const TTL: &str = "ttl"; // remaining hops/time budget units (opaque)
    pub const HOP_COUNT: &str = "hop";
    pub const TIMESTAMP_MS: &str = "ts";
}

/// Topic conventions for thread-scoped communication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadTopicKind {
    /// Broadcast to all participants interested in this thread
    Broadcast,
    /// Replies targeted to requester(s)
    Reply,
}

impl ThreadTopicKind {
    /// Build the canonical topic name for a thread
    pub fn topic(self, thread_id: &str) -> String {
        match self {
            ThreadTopicKind::Broadcast => format!("thread.{thread_id}.broadcast"),
            ThreadTopicKind::Reply => format!("thread.{thread_id}.reply"),
        }
    }
}

/// Unified envelope that rides inside Event.metadata and ActionCall.headers
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub thread_id: String,
    pub correlation_id: String,
    pub sender: String,
    pub reply_to: String,
    pub ttl: i32,
    pub hop: u32,
    pub timestamp_ms: i64,
}

impl Envelope {
    /// Construct an envelope with reasonable defaults
    pub fn new(thread_id: impl Into<String>, sender: impl Into<String>) -> Self {
        let thread_id = thread_id.into();
        let sender = sender.into();
        Self {
            reply_to: ThreadTopicKind::Reply.topic(&thread_id),
            thread_id: thread_id.clone(),
            correlation_id: thread_id.clone(),
            sender,
            ttl: 16,
            hop: 0,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Populate from a metadata map; uses fallbacks when keys are missing
    pub fn from_metadata(meta: &HashMap<String, String>, fallback_event_id: &str) -> Self {
        let thread_id = meta
            .get(keys::THREAD_ID)
            .cloned()
            .unwrap_or_else(|| fallback_event_id.to_string());
        let correlation_id = meta
            .get(keys::CORRELATION_ID)
            .cloned()
            .unwrap_or_else(|| thread_id.clone());
        let sender = meta.get(keys::SENDER).cloned().unwrap_or_default();
        let reply_to = meta
            .get(keys::REPLY_TO)
            .cloned()
            .unwrap_or_else(|| ThreadTopicKind::Reply.topic(&thread_id));
        let ttl = meta
            .get(keys::TTL)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(16);
        let hop = meta
            .get(keys::HOP_COUNT)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let timestamp_ms = meta
            .get(keys::TIMESTAMP_MS)
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        Self {
            thread_id,
            correlation_id,
            sender,
            reply_to,
            ttl,
            hop,
            timestamp_ms,
        }
    }

    /// Merge into a metadata map
    pub fn apply_to_metadata(&self, meta: &mut HashMap<String, String>) {
        meta.insert(keys::THREAD_ID.into(), self.thread_id.clone());
        meta.insert(keys::CORRELATION_ID.into(), self.correlation_id.clone());
        meta.insert(keys::SENDER.into(), self.sender.clone());
        meta.insert(keys::REPLY_TO.into(), self.reply_to.clone());
        meta.insert(keys::TTL.into(), self.ttl.to_string());
        meta.insert(keys::HOP_COUNT.into(), self.hop.to_string());
        meta.insert(keys::TIMESTAMP_MS.into(), self.timestamp_ms.to_string());
    }

    /// Extract from Event (reads metadata) with sensible defaults
    pub fn from_event(evt: &Event) -> Self {
        Self::from_metadata(&evt.metadata, &evt.id)
    }

    /// Write into Event.metadata (in-place)
    pub fn attach_to_event(&self, evt: &mut Event) {
        self.apply_to_metadata(&mut evt.metadata);
    }

    /// Apply to ActionCall.headers and correlation field
    pub fn apply_to_action_call(&self, call: &mut ActionCall) {
        // Ensure headers map carries all envelope fields
        if call.headers.is_empty() {
            call.headers = HashMap::new();
        }
        self.apply_to_metadata(&mut call.headers);
        call.correlation_id = self.correlation_id.clone();
        // Default QoS can be inferred elsewhere; not set here
    }

    /// Increment hop count and decrement ttl. Returns whether the message is still valid (ttl>0)
    pub fn next_hop(&mut self) -> bool {
        self.hop = self.hop.saturating_add(1);
        if self.ttl > 0 {
            self.ttl -= 1;
        }
        self.ttl > 0
    }

    /// Convenience: compute the broadcast topic for this thread
    pub fn broadcast_topic(&self) -> String {
        ThreadTopicKind::Broadcast.topic(&self.thread_id)
    }

    /// Convenience: compute the reply topic for this thread
    pub fn reply_topic(&self) -> String {
        ThreadTopicKind::Reply.topic(&self.thread_id)
    }
}
