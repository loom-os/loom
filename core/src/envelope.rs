use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::proto::{ActionCall, Event};

/// Reserved metadata/header keys for thread & correlation semantics.
///
/// These keys are used in `Event.metadata` and `ActionCall.headers` to carry
/// envelope information for multi-agent coordination.
pub mod keys {
    /// Thread identifier for grouping related messages
    pub const THREAD_ID: &str = "thread_id";
    /// Correlation identifier linking replies to requests
    pub const CORRELATION_ID: &str = "correlation_id";
    /// Logical identity of the message sender (e.g., "agent.foo")
    pub const SENDER: &str = "sender";
    /// Canonical reply topic for responses
    pub const REPLY_TO: &str = "reply_to";
    /// Remaining time-to-live (hops budget)
    pub const TTL: &str = "ttl";
    /// Current hop count (incremented each forwarding)
    pub const HOP_COUNT: &str = "hop";
    /// Timestamp in milliseconds since epoch
    pub const TIMESTAMP_MS: &str = "ts";
}

/// Topic conventions for thread-scoped communication.
///
/// Threads use a standardized naming convention to enable broadcast and
/// reply patterns without explicit subscriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadTopicKind {
    /// Broadcast to all participants interested in this thread.
    /// Topic format: `thread.{thread_id}.broadcast`
    Broadcast,
    /// Replies targeted to requester(s).
    /// Topic format: `thread.{thread_id}.reply`
    Reply,
}

impl ThreadTopicKind {
    /// Builds the canonical topic name for a thread.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - The thread identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::envelope::ThreadTopicKind;
    ///
    /// let broadcast = ThreadTopicKind::Broadcast.topic("task-123");
    /// assert_eq!(broadcast, "thread.task-123.broadcast");
    ///
    /// let reply = ThreadTopicKind::Reply.topic("task-123");
    /// assert_eq!(reply, "thread.task-123.reply");
    /// ```
    pub fn topic(self, thread_id: &str) -> String {
        match self {
            ThreadTopicKind::Broadcast => format!("thread.{thread_id}.broadcast"),
            ThreadTopicKind::Reply => format!("thread.{thread_id}.reply"),
        }
    }
}

/// Unified envelope carrying coordination metadata for Events and ActionCalls.
///
/// `Envelope` standardizes thread-scoped correlation, sender identity, TTL enforcement,
/// and reply routing across the multi-agent system. It rides inside `Event.metadata`
/// and `ActionCall.headers`, enabling patterns like request-reply, fanout-fanin, and
/// contract-net without tight coupling between components.
///
/// # Fields
///
/// * `thread_id` - Groups related messages in a collaboration session
/// * `correlation_id` - Links replies/proposals to originating requests (often same as thread_id)
/// * `sender` - Logical identity of the sender (e.g., "agent.worker-1")
/// * `reply_to` - Canonical topic for replies (typically `thread.{thread_id}.reply`)
/// * `ttl` - Time-to-live: remaining hops before message expires (prevents infinite loops)
/// * `hop` - Current hop count (incremented each forwarding)
/// * `timestamp_ms` - Creation timestamp in milliseconds since epoch
///
/// # Thread Lifecycle
///
/// 1. **Creation**: Initiator creates envelope with `new(thread_id, sender)`
/// 2. **Propagation**: Agent loop calls `next_hop()` to increment hop and decrement TTL
/// 3. **Expiration**: When `next_hop()` returns false (ttl ≤ 0), message is dropped
/// 4. **Reply**: Responders use `reply_topic()` and preserve correlation_id
///
/// # Integration Points
///
/// - **Agent loop**: Extracts envelope via `from_event()`, validates with `next_hop()`, reattaches
/// - **ActionBroker**: Applies envelope to ActionCall headers via `apply_to_action_call()`
/// - **Collaborator**: Uses `broadcast_topic()` and `reply_topic()` for coordination
///
/// # Examples
///
/// ```
/// use loom_core::Envelope;
/// use loom_core::proto::Event;
/// use std::collections::HashMap;
///
/// // Create envelope for a new collaboration thread
/// let mut env = Envelope::new("req-123", "agent.coordinator");
/// assert_eq!(env.thread_id, "req-123");
/// assert_eq!(env.correlation_id, "req-123");
/// assert_eq!(env.ttl, 16);
///
/// // Attach to event
/// let mut evt = Event {
///     id: "evt-1".to_string(),
///     r#type: "request".to_string(),
///     metadata: HashMap::new(),
///     payload: vec![],
///     timestamp_ms: 0,
///     source: "coordinator".to_string(),
///     confidence: 1.0,
///     tags: vec![],
///     priority: 50,
/// };
/// env.attach_to_event(&mut evt);
/// assert!(evt.metadata.contains_key("thread_id"));
///
/// // Extract from event (with TTL check)
/// let mut env2 = Envelope::from_event(&evt);
/// assert!(env2.next_hop()); // Returns true if TTL > 0
/// assert_eq!(env2.hop, 1);
/// assert_eq!(env2.ttl, 15);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    /// Thread identifier grouping related messages in a collaboration session
    pub thread_id: String,
    /// Correlation identifier linking replies/proposals to originating request
    pub correlation_id: String,
    /// Logical identity of the sender (e.g., "agent.worker-1", "broker:translate")
    pub sender: String,
    /// Canonical reply topic for responses (typically `thread.{thread_id}.reply`)
    pub reply_to: String,
    /// Time-to-live: remaining hops before expiration (prevents infinite loops)
    pub ttl: i32,
    /// Current hop count (incremented each forwarding)
    pub hop: u32,
    /// Creation timestamp in milliseconds since epoch
    pub timestamp_ms: i64,
}

impl Envelope {
    /// Creates a new envelope with default TTL and reply topic.
    ///
    /// Sets `correlation_id` equal to `thread_id`, `ttl` to 16, `hop` to 0,
    /// `reply_to` to `thread.{thread_id}.reply`, and `timestamp_ms` to current time.
    ///
    /// # Arguments
    ///
    /// * `thread_id` - Thread identifier for grouping related messages
    /// * `sender` - Logical identity of the sender
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::Envelope;
    ///
    /// let env = Envelope::new("task-42", "agent.coordinator");
    /// assert_eq!(env.thread_id, "task-42");
    /// assert_eq!(env.correlation_id, "task-42");
    /// assert_eq!(env.sender, "agent.coordinator");
    /// assert_eq!(env.ttl, 16);
    /// assert_eq!(env.hop, 0);
    /// assert_eq!(env.reply_to, "thread.task-42.reply");
    /// ```
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

    /// Constructs an envelope from a metadata map with fallbacks for missing keys.
    ///
    /// Uses `fallback_event_id` as `thread_id` if not present in metadata.
    /// Provides sensible defaults for all fields to ensure robustness.
    ///
    /// # Arguments
    ///
    /// * `meta` - Metadata map (e.g., from `Event.metadata` or `ActionCall.headers`)
    /// * `fallback_event_id` - Default thread_id if not in metadata
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::Envelope;
    /// use std::collections::HashMap;
    ///
    /// let mut meta = HashMap::new();
    /// meta.insert("thread_id".to_string(), "req-789".to_string());
    /// meta.insert("sender".to_string(), "agent.worker".to_string());
    ///
    /// let env = Envelope::from_metadata(&meta, "fallback-id");
    /// assert_eq!(env.thread_id, "req-789");
    /// assert_eq!(env.sender, "agent.worker");
    /// ```
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

    /// Writes envelope fields into a metadata map.
    ///
    /// Inserts or overwrites all envelope keys in the provided map.
    /// Used internally by `attach_to_event()` and `apply_to_action_call()`.
    ///
    /// # Arguments
    ///
    /// * `meta` - Mutable reference to metadata map
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::Envelope;
    /// use std::collections::HashMap;
    ///
    /// let env = Envelope::new("thread-1", "agent.sender");
    /// let mut meta = HashMap::new();
    /// env.apply_to_metadata(&mut meta);
    ///
    /// assert_eq!(meta.get("thread_id"), Some(&"thread-1".to_string()));
    /// assert_eq!(meta.get("sender"), Some(&"agent.sender".to_string()));
    /// ```
    pub fn apply_to_metadata(&self, meta: &mut HashMap<String, String>) {
        meta.insert(keys::THREAD_ID.into(), self.thread_id.clone());
        meta.insert(keys::CORRELATION_ID.into(), self.correlation_id.clone());
        meta.insert(keys::SENDER.into(), self.sender.clone());
        meta.insert(keys::REPLY_TO.into(), self.reply_to.clone());
        meta.insert(keys::TTL.into(), self.ttl.to_string());
        meta.insert(keys::HOP_COUNT.into(), self.hop.to_string());
        meta.insert(keys::TIMESTAMP_MS.into(), self.timestamp_ms.to_string());
    }

    /// Extracts envelope from an Event with fallback to event ID.
    ///
    /// Convenience wrapper around `from_metadata(&evt.metadata, &evt.id)`.
    ///
    /// # Arguments
    ///
    /// * `evt` - The event to extract envelope from
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{Envelope, proto::Event};
    /// use std::collections::HashMap;
    ///
    /// let mut evt = Event {
    ///     id: "evt-123".to_string(),
    ///     r#type: "request".to_string(),
    ///     metadata: {
    ///         let mut m = HashMap::new();
    ///         m.insert("thread_id".to_string(), "thread-1".to_string());
    ///         m
    ///     },
    ///     payload: vec![],
    ///     timestamp_ms: 0,
    ///     source: "test".to_string(),
    ///     confidence: 1.0,
    ///     tags: vec![],
    ///     priority: 50,
    /// };
    ///
    /// let env = Envelope::from_event(&evt);
    /// assert_eq!(env.thread_id, "thread-1");
    /// ```
    pub fn from_event(evt: &Event) -> Self {
        Self::from_metadata(&evt.metadata, &evt.id)
    }

    /// Writes envelope into Event metadata (mutates in place).
    ///
    /// Convenience wrapper around `apply_to_metadata(&mut evt.metadata)`.
    ///
    /// # Arguments
    ///
    /// * `evt` - Mutable reference to the event
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{Envelope, proto::Event};
    /// use std::collections::HashMap;
    ///
    /// let env = Envelope::new("task-99", "agent.coordinator");
    /// let mut evt = Event {
    ///     id: "evt-1".to_string(),
    ///     r#type: "request".to_string(),
    ///     metadata: HashMap::new(),
    ///     payload: vec![],
    ///     timestamp_ms: 0,
    ///     source: "test".to_string(),
    ///     confidence: 1.0,
    ///     tags: vec![],
    ///     priority: 50,
    /// };
    ///
    /// env.attach_to_event(&mut evt);
    /// assert_eq!(evt.metadata.get("thread_id"), Some(&"task-99".to_string()));
    /// ```
    pub fn attach_to_event(&self, evt: &mut Event) {
        self.apply_to_metadata(&mut evt.metadata);
    }

    /// Writes envelope into ActionCall headers and sets correlation_id.
    ///
    /// Ensures `call.headers` is initialized, applies all envelope fields,
    /// and sets `call.correlation_id` for idempotency tracking.
    ///
    /// # Arguments
    ///
    /// * `call` - Mutable reference to the ActionCall
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::{Envelope, proto::ActionCall};
    /// use std::collections::HashMap;
    ///
    /// let env = Envelope::new("act-456", "agent.executor");
    /// let mut call = ActionCall {
    ///     id: "call-1".to_string(),
    ///     capability: "translate".to_string(),
    ///     version: "1.0.0".to_string(),
    ///     payload: vec![],
    ///     headers: HashMap::new(),
    ///     timeout_ms: 5000,
    ///     correlation_id: String::new(),
    ///     qos: 0,
    /// };
    ///
    /// env.apply_to_action_call(&mut call);
    /// assert_eq!(call.correlation_id, "act-456");
    /// assert!(call.headers.contains_key("thread_id"));
    /// ```
    pub fn apply_to_action_call(&self, call: &mut ActionCall) {
        // Ensure headers map carries all envelope fields
        self.apply_to_metadata(&mut call.headers);
        call.correlation_id = self.correlation_id.clone();
        // Default QoS can be inferred elsewhere; not set here
    }

    /// Advances the envelope to the next hop: increments hop count and decrements TTL.
    ///
    /// Returns `true` if the message is still valid (TTL > 0) and should be processed.
    /// Returns `false` if TTL has expired and the message should be dropped.
    ///
    /// The Agent loop should call this method before processing each event to prevent
    /// infinite forwarding loops.
    ///
    /// # Returns
    ///
    /// * `true` - Message is valid, continue processing
    /// * `false` - TTL expired, drop message
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::Envelope;
    ///
    /// let mut env = Envelope::new("thread-1", "agent-1");
    /// assert_eq!(env.ttl, 16);
    /// assert_eq!(env.hop, 0);
    ///
    /// // First hop
    /// assert!(env.next_hop());
    /// assert_eq!(env.hop, 1);
    /// assert_eq!(env.ttl, 15);
    ///
    /// // Exhaust TTL
    /// for _ in 0..15 {
    ///     env.next_hop();
    /// }
    /// assert_eq!(env.ttl, 0);
    /// assert!(!env.next_hop()); // Returns false when expired
    /// ```
    pub fn next_hop(&mut self) -> bool {
        self.hop = self.hop.saturating_add(1);
        if self.ttl > 0 {
            self.ttl -= 1;
        }
        self.ttl > 0
    }

    /// Returns the broadcast topic for this thread.
    ///
    /// Convenience method wrapping `ThreadTopicKind::Broadcast.topic(&self.thread_id)`.
    /// Format: `thread.{thread_id}.broadcast`
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::Envelope;
    ///
    /// let env = Envelope::new("collab-1", "agent-coordinator");
    /// assert_eq!(env.broadcast_topic(), "thread.collab-1.broadcast");
    /// ```
    pub fn broadcast_topic(&self) -> String {
        ThreadTopicKind::Broadcast.topic(&self.thread_id)
    }

    /// Returns the reply topic for this thread.
    ///
    /// Convenience method wrapping `ThreadTopicKind::Reply.topic(&self.thread_id)`.
    /// Format: `thread.{thread_id}.reply`
    ///
    /// # Examples
    ///
    /// ```
    /// use loom_core::Envelope;
    ///
    /// let env = Envelope::new("req-42", "agent-requester");
    /// assert_eq!(env.reply_topic(), "thread.req-42.reply");
    /// ```
    pub fn reply_topic(&self) -> String {
        ThreadTopicKind::Reply.topic(&self.thread_id)
    }
}
