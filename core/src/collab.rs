use std::{collections::HashMap, sync::Arc};

use tokio::time::{timeout, Duration, Instant};

use crate::{envelope::keys, envelope::ThreadTopicKind, Envelope, Event, EventBus, Result};

/// Control event type names used on Event.r#type for collaboration protocols
pub mod types {
    /// Request event in request-reply or fanout-fanin patterns
    pub const REQ: &str = "collab.request";
    /// Reply event in request-reply or fanout-fanin patterns
    pub const REPLY: &str = "collab.reply";
    /// Call for proposals in contract-net protocol
    pub const CFP: &str = "collab.cfp";
    /// Proposal response in contract-net protocol
    pub const PROPOSAL: &str = "collab.proposal";
    /// Award announcement in contract-net protocol
    pub const AWARD: &str = "collab.award";
    /// Optional heartbeat for barrier synchronization
    pub const BARRIER_TICK: &str = "collab.barrier";
    /// Timeout notification when collaboration fails to complete
    pub const TIMEOUT: &str = "collab.timeout";
    /// Summary event with collaboration results and statistics
    pub const SUMMARY: &str = "collab.summary";
}

/// Lightweight collaboration coordinator for multi-agent interactions.
///
/// `Collaborator` provides three core multi-agent collaboration patterns built on
/// top of EventBus and Envelope:
///
/// 1. **Request-Reply**: Single request with timeout, waiting for first reply
/// 2. **Fanout-Fanin**: Broadcast to multiple topics, collect first_k replies
/// 3. **Contract Net Protocol**: CFP → collect proposals → rank by score → award
///
/// All methods use thread-scoped topics (via Envelope) to ensure proper correlation
/// and avoid cross-talk between concurrent collaborations.
///
/// # Thread Safety
///
/// `Collaborator` is safe to share across tasks and can coordinate multiple
/// concurrent collaborations. Each collaboration gets a unique thread_id.
///
/// # Examples
///
/// ```no_run
/// use loom_core::{Collaborator, EventBus};
/// use std::sync::Arc;
///
/// # async fn example() -> loom_core::Result<()> {
/// let bus = Arc::new(EventBus::new().await?);
/// let collab = Collaborator::new(bus, "agent-1");
///
/// // Request-reply with 5s timeout
/// if let Some(reply) = collab.request_reply(
///     "agents.helper",
///     b"help request".to_vec(),
///     5000
/// ).await? {
///     println!("Got reply: {:?}", reply);
/// }
/// # Ok(())
/// # }
/// ```
pub struct Collaborator {
    event_bus: Arc<EventBus>,
    sender_id: String,
}

impl Collaborator {
    /// Creates a new `Collaborator` instance.
    ///
    /// # Arguments
    ///
    /// * `event_bus` - Shared EventBus for pub/sub communication
    /// * `sender_id` - Identifier for this collaborator (typically agent ID)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loom_core::{Collaborator, EventBus};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> loom_core::Result<()> {
    /// let bus = Arc::new(EventBus::new().await?);
    /// let collab = Collaborator::new(bus, "agent-coordinator");
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(event_bus: Arc<EventBus>, sender_id: impl Into<String>) -> Self {
        Self {
            event_bus,
            sender_id: sender_id.into(),
        }
    }

    /// Performs a request-reply interaction with timeout.
    ///
    /// Publishes a request event to the specified topic, then waits for the first
    /// reply on a dedicated thread-scoped reply topic. Uses Envelope correlation
    /// to match the reply to this specific request.
    ///
    /// # Arguments
    ///
    /// * `topic` - Topic to publish the request (e.g., "agents.helper")
    /// * `payload` - Request payload bytes
    /// * `timeout_ms` - Maximum wait time in milliseconds. Must be > 0.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Event))` - Received a matching reply within timeout
    /// * `Ok(None)` - Timeout expired with no reply
    /// * `Err(_)` - EventBus error or invalid parameters (timeout_ms == 0)
    ///
    /// # Timeout Behavior
    ///
    /// On timeout, publishes a `collab.timeout` event to the reply topic for
    /// observability before returning `Ok(None)`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loom_core::{Collaborator, EventBus};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> loom_core::Result<()> {
    /// let bus = Arc::new(EventBus::new().await?);
    /// let collab = Collaborator::new(bus, "agent-1");
    ///
    /// match collab.request_reply(
    ///     "agents.calculator",
    ///     b"compute 2+2".to_vec(),
    ///     3000
    /// ).await? {
    ///     Some(reply) => {
    ///         let result = String::from_utf8_lossy(&reply.payload);
    ///         println!("Result: {}", result);
    ///     }
    ///     None => println!("No reply within 3s"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request_reply(
        &self,
        topic: &str,
        payload: Vec<u8>,
        timeout_ms: u64,
    ) -> Result<Option<Event>> {
        if timeout_ms == 0 {
            return Err(crate::LoomError::EventBusError(
                "timeout_ms must be greater than 0".into(),
            ));
        }

        // Prepare envelope and subscribe to reply topic first to avoid races
        let thread_id = format!(
            "req_{}",
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000)
        );
        let env = Envelope::new(thread_id, self.sender_id.clone());
        let reply_topic = env.reply_topic();

        let (_sub_id, mut rx) = self
            .event_bus
            .subscribe(
                reply_topic.clone(),
                vec![types::REPLY.into()],
                crate::proto::QoSLevel::QosBatched,
            )
            .await?;

        let mut md = HashMap::new();
        env.apply_to_metadata(&mut md);
        let mut evt = Event {
            id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
            r#type: types::REQ.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            source: self.sender_id.clone(),
            metadata: md,
            payload,
            confidence: 1.0,
            tags: vec!["collab".into()],
            priority: 50,
        };
        env.attach_to_event(&mut evt);
        let _ = self.event_bus.publish(topic, evt).await?;

        // Await first reply that matches correlation
        let deadline = Duration::from_millis(timeout_ms);
        let corr_id = env.correlation_id.clone();
        let res = timeout(deadline, async move {
            while let Some(ev) = rx.recv().await {
                // basic correlation match
                if ev.metadata.get(keys::CORRELATION_ID) == Some(&corr_id) {
                    return Some(ev);
                }
            }
            None
        })
        .await
        .ok()
        .flatten();

        if res.is_none() {
            // emit timeout summary (best-effort) on reply topic
            let mut md = HashMap::new();
            env.apply_to_metadata(&mut md);
            md.insert("reason".into(), "request_reply_timeout".into());
            let mut evt = Event {
                id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                r#type: types::TIMEOUT.into(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                source: self.sender_id.clone(),
                metadata: md,
                payload: Vec::new(),
                confidence: 1.0,
                tags: vec!["collab".into()],
                priority: 40,
            };
            env.attach_to_event(&mut evt);
            let _ = self.event_bus.publish(&reply_topic, evt).await?;
        }
        Ok(res)
    }

    /// Performs a fanout-fanin interaction: broadcast to multiple topics and
    /// collect up to `first_k` replies within timeout.
    ///
    /// Publishes the same request to all specified topics concurrently, then
    /// collects replies on a dedicated thread-scoped reply topic until either
    /// `first_k` replies are received or timeout expires.
    ///
    /// # Arguments
    ///
    /// * `topics` - List of topics to broadcast to. Returns empty vec if empty.
    /// * `payload` - Request payload bytes (cloned for each topic)
    /// * `first_k` - Maximum number of replies to collect. Must be > 0.
    /// * `timeout_ms` - Maximum wait time in milliseconds. Must be > 0.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Event>)` - Collected replies (may be fewer than `first_k` if timeout)
    /// * `Err(_)` - EventBus error or invalid parameters (first_k == 0, timeout_ms == 0)
    ///
    /// # Completion Behavior
    ///
    /// Always publishes a `collab.summary` event with `received` and `target_first_k`
    /// metadata for observability, regardless of whether `first_k` was reached.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loom_core::{Collaborator, EventBus};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> loom_core::Result<()> {
    /// let bus = Arc::new(EventBus::new().await?);
    /// let collab = Collaborator::new(bus, "agent-orchestrator");
    ///
    /// let topics = vec![
    ///     "agents.worker-1".to_string(),
    ///     "agents.worker-2".to_string(),
    ///     "agents.worker-3".to_string(),
    /// ];
    ///
    /// // Get first 2 replies within 5s
    /// let replies = collab.fanout_fanin(
    ///     &topics,
    ///     b"task: analyze".to_vec(),
    ///     2,
    ///     5000
    /// ).await?;
    ///
    /// println!("Received {} replies", replies.len());
    /// for reply in replies {
    ///     println!("From: {}", reply.source);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fanout_fanin(
        &self,
        topics: &[String],
        payload: Vec<u8>,
        first_k: usize,
        timeout_ms: u64,
    ) -> Result<Vec<Event>> {
        if topics.is_empty() {
            return Ok(Vec::new());
        }
        if first_k == 0 {
            return Err(crate::LoomError::EventBusError(
                "first_k must be greater than 0".into(),
            ));
        }
        if timeout_ms == 0 {
            return Err(crate::LoomError::EventBusError(
                "timeout_ms must be greater than 0".into(),
            ));
        }

        let thread_id = format!(
            "fanout_{}",
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000)
        );
        let env = Envelope::new(thread_id, self.sender_id.clone());
        let reply_topic = env.reply_topic();
        let (_sub_id, mut rx) = self
            .event_bus
            .subscribe(
                reply_topic.clone(),
                vec![types::REPLY.into(), types::PROPOSAL.into()],
                crate::proto::QoSLevel::QosBatched,
            )
            .await?;

        // Broadcast a request to each topic
        for t in topics {
            let mut md = HashMap::new();
            env.apply_to_metadata(&mut md);
            let mut evt = Event {
                id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                r#type: types::REQ.into(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                source: self.sender_id.clone(),
                metadata: md.clone(),
                payload: payload.clone(),
                confidence: 1.0,
                tags: vec!["collab".into()],
                priority: 50,
            };
            env.attach_to_event(&mut evt);
            let _ = self.event_bus.publish(t, evt).await?;
        }

        // Gather first_k
        let mut out = Vec::with_capacity(first_k);
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let corr_id = env.correlation_id.clone();
        while out.len() < first_k && Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if let Ok(Some(ev)) = timeout(remaining, rx.recv()).await.map(|o| o) {
                if ev.metadata.get(keys::CORRELATION_ID) == Some(&corr_id) {
                    out.push(ev);
                }
            } else {
                break;
            }
        }
        // Emit barrier summary
        let mut md = HashMap::new();
        env.apply_to_metadata(&mut md);
        md.insert("received".into(), out.len().to_string());
        md.insert("target_first_k".into(), first_k.to_string());
        let mut evt = Event {
            id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
            r#type: types::SUMMARY.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            source: self.sender_id.clone(),
            metadata: md,
            payload: Vec::new(),
            confidence: 1.0,
            tags: vec!["collab".into()],
            priority: 40,
        };
        env.attach_to_event(&mut evt);
        let _ = self.event_bus.publish(&reply_topic, evt).await?;
        Ok(out)
    }

    /// Performs Contract Net Protocol: announce CFP, collect proposals, rank by
    /// score, award to top bidders, and return winning proposals.
    ///
    /// This implements a task allocation protocol where:
    /// 1. Coordinator publishes CFP (call for proposals) to broadcast topic
    /// 2. Agents publish proposals to reply topic with `score` metadata
    /// 3. Coordinator ranks proposals by score (descending)
    /// 4. Coordinator publishes awards to broadcast topic for top `max_awards`
    /// 5. Coordinator returns winning proposals for task assignment
    ///
    /// # Arguments
    ///
    /// * `broadcast_thread_id` - Thread ID for the collaboration session
    /// * `cfp_payload` - Call for proposals payload (task description)
    /// * `window_ms` - Proposal collection window in milliseconds. Must be > 0.
    /// * `max_awards` - Maximum number of proposals to award. Must be > 0.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Event>)` - Top `max_awards` proposals sorted by score (descending)
    /// * `Err(_)` - EventBus error or invalid parameters (window_ms == 0, max_awards == 0)
    ///
    /// # Proposal Ranking
    ///
    /// Proposals are sorted by the `score` field in their metadata. Missing or
    /// invalid scores are treated as 0.0. Higher scores rank first.
    ///
    /// # Completion Behavior
    ///
    /// Always publishes:
    /// - `collab.award` events to broadcast topic (one per winner)
    /// - `collab.summary` event with `winners` and `max_awards` metadata
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use loom_core::{Collaborator, EventBus};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> loom_core::Result<()> {
    /// let bus = Arc::new(EventBus::new().await?);
    /// let collab = Collaborator::new(bus, "task-coordinator");
    ///
    /// // Announce task and collect proposals for 3s
    /// let winners = collab.contract_net(
    ///     "task-123",
    ///     b"Translate document from EN to FR".to_vec(),
    ///     3000,
    ///     2  // Award to top 2 bidders
    /// ).await?;
    ///
    /// println!("Awarded to {} agents:", winners.len());
    /// for (i, proposal) in winners.iter().enumerate() {
    ///     let score = proposal.metadata.get("score")
    ///         .map(|s| s.as_str())
    ///         .unwrap_or("N/A");
    ///     println!("  {}. {} (score: {})", i+1, proposal.source, score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn contract_net(
        &self,
        broadcast_thread_id: &str,
        cfp_payload: Vec<u8>,
        window_ms: u64,
        max_awards: usize,
    ) -> Result<Vec<Event>> {
        if window_ms == 0 {
            return Err(crate::LoomError::EventBusError(
                "window_ms must be greater than 0".into(),
            ));
        }
        if max_awards == 0 {
            return Err(crate::LoomError::EventBusError(
                "max_awards must be greater than 0".into(),
            ));
        }

        let thread_id = broadcast_thread_id.to_string();
        let env = Envelope::new(thread_id.clone(), self.sender_id.clone());
        // Subscribe to proposals on reply topic (agents reply on thread reply by convention)
        let reply_topic = ThreadTopicKind::Reply.topic(&thread_id);
        let (_sub_id, mut rx) = self
            .event_bus
            .subscribe(
                reply_topic.clone(),
                vec![types::PROPOSAL.into()],
                crate::proto::QoSLevel::QosBatched,
            )
            .await?;

        // Publish CFP to broadcast topic
        let broadcast_topic = ThreadTopicKind::Broadcast.topic(&thread_id);
        let mut md = HashMap::new();
        env.apply_to_metadata(&mut md);
        let mut cfp_evt = Event {
            id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
            r#type: types::CFP.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            source: self.sender_id.clone(),
            metadata: md,
            payload: cfp_payload,
            confidence: 1.0,
            tags: vec!["collab".into()],
            priority: 50,
        };
        env.attach_to_event(&mut cfp_evt);
        let _ = self.event_bus.publish(&broadcast_topic, cfp_evt).await?;

        // Collect proposals during window
        let end = Instant::now() + Duration::from_millis(window_ms);
        let mut proposals: Vec<Event> = Vec::new();
        while Instant::now() < end {
            let remaining = end.saturating_duration_since(Instant::now());
            match timeout(remaining, rx.recv()).await {
                Ok(Some(ev)) => {
                    if ev.metadata.get(keys::CORRELATION_ID) == Some(&env.correlation_id) {
                        proposals.push(ev);
                    }
                }
                _ => break,
            }
        }

        // Rank by metadata.score (descending)
        proposals.sort_by(|a, b| {
            let sa = a
                .metadata
                .get("score")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let sb = b
                .metadata
                .get("score")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        let winners = proposals.into_iter().take(max_awards).collect::<Vec<_>>();

        // Publish awards to broadcast topic
        for w in &winners {
            let mut award_meta = w.metadata.clone();
            award_meta.insert(
                "award_to".into(),
                w.metadata.get(keys::SENDER).cloned().unwrap_or_default(),
            );
            let mut award_evt = Event {
                id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                r#type: types::AWARD.into(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                source: self.sender_id.clone(),
                metadata: award_meta,
                payload: Vec::new(),
                confidence: 1.0,
                tags: vec!["collab".into()],
                priority: 60,
            };
            env.attach_to_event(&mut award_evt);
            let _ = self.event_bus.publish(&broadcast_topic, award_evt).await?;
        }

        // Publish summary with total winners
        let mut md = HashMap::new();
        env.apply_to_metadata(&mut md);
        md.insert("winners".into(), winners.len().to_string());
        md.insert("max_awards".into(), max_awards.to_string());
        let mut summary_evt = Event {
            id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
            r#type: types::SUMMARY.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            source: self.sender_id.clone(),
            metadata: md,
            payload: Vec::new(),
            confidence: 1.0,
            tags: vec!["collab".into()],
            priority: 40,
        };
        env.attach_to_event(&mut summary_evt);
        let _ = self
            .event_bus
            .publish(&broadcast_topic, summary_evt)
            .await?;

        Ok(winners)
    }
}
