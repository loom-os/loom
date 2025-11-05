#[cfg(feature = "wake")]
use loom_core::audio::{WakeWordConfig, WakeWordDetector};
#[cfg(feature = "wake")]
use loom_core::proto::Event;
#[cfg(feature = "wake")]
use loom_core::{EventBus, QoSLevel};
#[cfg(feature = "wake")]
use std::sync::Arc;

#[cfg(feature = "wake")]
fn now_ms() -> i64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()) as i64
}
#[cfg(feature = "wake")]
fn gen_id() -> String {
    format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

#[cfg(not(feature = "wake"))]
fn main() {}

#[cfg(feature = "wake")]
#[tokio::main]
async fn main() -> loom_core::Result<()> {
    tracing_subscriber::fmt::init();

    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;

    // Start wake word detector
    let wake = WakeWordDetector::new(Arc::clone(&bus), WakeWordConfig::default());
    let _handle = wake.start().await?;

    // Subscribe to wake + query topics
    let (_w_id, mut w_rx) = bus
        .subscribe(
            "wake".into(),
            vec!["wake_word_detected".into()],
            QoSLevel::QosRealtime,
        )
        .await?;
    let (_q_id, mut q_rx) = bus
        .subscribe(
            "query".into(),
            vec!["user.query".into()],
            QoSLevel::QosRealtime,
        )
        .await?;

    // Helper to publish a transcript.final
    let publish_transcript = |text: &str| {
        let mut md = std::collections::HashMap::new();
        md.insert("text".into(), text.to_string());
        Event {
            id: gen_id(),
            r#type: "transcript.final".into(),
            timestamp_ms: now_ms(),
            source: "stt".into(),
            metadata: md,
            payload: text.as_bytes().to_vec(),
            confidence: 1.0,
            tags: vec![],
            priority: 70,
        }
    };

    // Simulate utterances
    bus.publish("transcript", publish_transcript("hey loom"))
        .await?;
    bus.publish("transcript", publish_transcript("what time is it"))
        .await?;

    // Read two messages
    if let Some(ev) = w_rx.recv().await {
        println!("[wake] {}", serde_json::to_string(&ev.metadata).unwrap());
    }
    if let Some(ev) = q_rx.recv().await {
        println!("[query] {}", String::from_utf8_lossy(&ev.payload));
    }

    Ok(())
}
