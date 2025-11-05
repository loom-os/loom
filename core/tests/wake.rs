//! Integration tests for transcript-based Wake Word detection

// When the 'wake' feature is enabled, run the real tests
#[cfg(feature = "wake")]
mod wake_tests {
    use loom_core::audio::{WakeWordConfig, WakeWordDetector};
    use loom_core::{Event, EventBus, QoSLevel};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn gen_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{:x}", nanos)
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// Helper to create a transcript.final event with text in metadata
    fn transcript_event(topic_text: &str) -> Event {
        let mut metadata = HashMap::new();
        metadata.insert("text".to_string(), topic_text.to_string());

        Event {
            id: gen_id(),
            r#type: "transcript.final".to_string(),
            timestamp_ms: now_ms(),
            source: "test_stt".to_string(),
            metadata,
            payload: Vec::new(),
            confidence: 0.95,
            tags: vec![],
            priority: 70,
        }
    }

    #[tokio::test]
    async fn test_wake_detects_and_immediate_query() {
        // Setup event bus
        let bus = Arc::new(EventBus::new().await.unwrap());
        bus.start().await.unwrap();

        // Configure unique topics per test run to avoid cross-test interference
        let ns = gen_id();
        let cfg = WakeWordConfig {
            transcript_topic: format!("test.transcript.{}", ns),
            wake_topic: format!("test.wake.{}", ns),
            query_topic: format!("test.query.{}", ns),
            phrases: vec!["hey loom".into(), "loom".into()],
            ..Default::default()
        };

        // Subscribe to outputs
        let (_w_id, mut wake_rx) = bus
            .subscribe(
                cfg.wake_topic.clone(),
                vec!["wake_word_detected".into()],
                QoSLevel::QosRealtime,
            )
            .await
            .unwrap();
        let (_q_id, mut query_rx) = bus
            .subscribe(
                cfg.query_topic.clone(),
                vec!["user.query".into()],
                QoSLevel::QosRealtime,
            )
            .await
            .unwrap();

        // Start detector
        let detector = WakeWordDetector::new(Arc::clone(&bus), cfg.clone());
        let _handle = detector.start().await.unwrap();

        // Allow startup
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

        // Publish a transcript that should trigger immediate query (has remainder)
        bus.publish(
            &cfg.transcript_topic,
            transcript_event("Hey loom what's up"),
        )
        .await
        .unwrap();

        // Collect outputs with timeout
        let mut got_wake = false;
        let mut got_query = false;
        let mut query_text: Option<String> = None;

        let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(400));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(ev) = wake_rx.recv() => {
                    if ev.r#type == "wake_word_detected" { got_wake = true; }
                }
                Some(ev) = query_rx.recv() => {
                    if ev.r#type == "user.query" {
                        got_query = true;
                        query_text = ev.metadata.get("text").cloned();
                    }
                }
                _ = &mut timeout => break,
            }
        }

        assert!(got_wake, "Expected wake_word_detected event");
        assert!(got_query, "Expected immediate user.query event");
        let qt = query_text.unwrap_or_default();
        assert!(
            qt.starts_with("what"),
            "Expected remainder query, got '{}'",
            qt
        );

        bus.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_wake_arms_and_uses_next_utterance() {
        let bus = Arc::new(EventBus::new().await.unwrap());
        bus.start().await.unwrap();

        let ns = gen_id();
        let cfg = WakeWordConfig {
            transcript_topic: format!("test.transcript.{}", ns),
            wake_topic: format!("test.wake.{}", ns),
            query_topic: format!("test.query.{}", ns),
            phrases: vec!["hey loom".into()],
            ..Default::default()
        };

        let (_w_id, mut wake_rx) = bus
            .subscribe(
                cfg.wake_topic.clone(),
                vec!["wake_word_detected".into()],
                QoSLevel::QosRealtime,
            )
            .await
            .unwrap();
        let (_q_id, mut query_rx) = bus
            .subscribe(
                cfg.query_topic.clone(),
                vec!["user.query".into()],
                QoSLevel::QosRealtime,
            )
            .await
            .unwrap();

        let detector = WakeWordDetector::new(Arc::clone(&bus), cfg.clone());
        let _handle = detector.start().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

        // 1) Utterance with only wake (no remainder) -> should arm
        bus.publish(&cfg.transcript_topic, transcript_event("hey loom"))
            .await
            .unwrap();

        // Wait until we observe wake_word_detected to avoid races
        let mut saw_wake = false;
        let mut wake_wait = tokio::time::sleep(tokio::time::Duration::from_millis(300));
        tokio::pin!(wake_wait);
        loop {
            tokio::select! {
                Some(ev) = wake_rx.recv() => {
                    if ev.r#type == "wake_word_detected" { saw_wake = true; break; }
                }
                _ = &mut wake_wait => break,
            }
        }
        assert!(
            saw_wake,
            "Expected wake_word_detected before sending next utterance"
        );
        // Already observed wake; no need to require it again later

        // 2) Next utterance should be treated as query
        bus.publish(&cfg.transcript_topic, transcript_event("what time is it"))
            .await
            .unwrap();

        // give a brief moment before collecting
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

        let mut got_query = false;
        let mut query_text: Option<String> = None;
        let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1200));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(_ev) = wake_rx.recv() => { /* wake already asserted above */ }
                Some(ev) = query_rx.recv() => {
                    if ev.r#type == "user.query" {
                        got_query = true;
                        query_text = ev.metadata.get("text").cloned();
                    }
                }
                _ = &mut timeout => break,
            }
        }

        assert!(got_query, "Expected armed user.query on next utterance");
        assert_eq!(query_text.as_deref(), Some("what time is it"));

        bus.shutdown().await.unwrap();
    }
}

// Placeholder test when the feature is disabled, so `cargo test` still passes
#[cfg(not(feature = "wake"))]
#[test]
fn wake_tests_require_feature() {
    println!("Wake tests require 'wake' feature. Run: cargo test --features wake");
}
