use std::{collections::HashMap, sync::Arc};

use tokio::time::{timeout, Duration, Instant};

use crate::{envelope::keys, envelope::ThreadTopicKind, Envelope, Event, EventBus, Result};

/// Control event type names used on Event.r#type
pub mod types {
    pub const REQ: &str = "collab.request";
    pub const REPLY: &str = "collab.reply";
    pub const CFP: &str = "collab.cfp"; // contract-net: call for proposals
    pub const PROPOSAL: &str = "collab.proposal";
    pub const AWARD: &str = "collab.award";
    pub const BARRIER_TICK: &str = "collab.barrier"; // optional heartbeat
    pub const TIMEOUT: &str = "collab.timeout";
    pub const SUMMARY: &str = "collab.summary";
}

/// Lightweight collaboration coordinator built on EventBus + Envelope
pub struct Collaborator {
    event_bus: Arc<EventBus>,
    sender_id: String,
}

impl Collaborator {
    pub fn new(event_bus: Arc<EventBus>, sender_id: impl Into<String>) -> Self {
        Self {
            event_bus,
            sender_id: sender_id.into(),
        }
    }

    /// Send a request and wait for the first reply on the thread reply topic.
    /// Returns None on timeout.
    pub async fn request_reply(
        &self,
        topic: &str,
        payload: Vec<u8>,
        timeout_ms: u64,
    ) -> Result<Option<Event>> {
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
        let deadline = Duration::from_millis(timeout_ms.max(1));
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

    /// Fanout to multiple topics, collect up to first_k replies or until timeout.
    pub async fn fanout_fanin(
        &self,
        topics: &[String],
        payload: Vec<u8>,
        first_k: usize,
        timeout_ms: u64,
    ) -> Result<Vec<Event>> {
        if topics.is_empty() || first_k == 0 {
            return Ok(Vec::new());
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
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1));
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

    /// Contract Net Protocol: send CFP to a broadcast thread topic, collect proposals for window_ms,
    /// pick top `max_awards` by numeric `score` metadata, publish awards to the thread broadcast topic, return selected proposals.
    pub async fn contract_net(
        &self,
        broadcast_thread_id: &str,
        cfp_payload: Vec<u8>,
        window_ms: u64,
        max_awards: usize,
    ) -> Result<Vec<Event>> {
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
        let end = Instant::now() + Duration::from_millis(window_ms.max(1));
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
        let winners = proposals
            .into_iter()
            .take(max_awards.max(1))
            .collect::<Vec<_>>();

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
