//! Backpressure mechanism tests
//!
//! Tests covering backpressure threshold enforcement and dropping strategies.

use super::make_event;
use loom_core::event::EventBus;
use loom_core::proto::QoSLevel;
use loom_core::Result;
use serial_test::serial;

/// Test: Backpressure threshold triggers dropping for realtime
#[tokio::test]
#[serial]
pub async fn threshold_enforcement() -> Result<()> {
    let bus = EventBus::new().await?;
    let topic = "pressure.threshold";

    // Create subscription but don't consume
    let (_sub_id, _rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosRealtime)
        .await?;

    // Publish enough to exceed backpressure_threshold (10_000 in code)
    // With realtime QoS, events should start dropping aggressively
    let event_count = 15_000;
    for i in 0..event_count {
        let evt = make_event(i, "threshold_test");
        let _ = bus.publish(topic, evt).await;
    }

    let stats = bus.get_stats(topic).expect("stats");

    println!(
        "Backpressure threshold test: published {}, dropped {}, backlog {}",
        stats.total_published, stats.dropped_events, stats.backlog_size
    );

    assert!(
        stats.dropped_events > 0,
        "Should drop events when over backpressure threshold"
    );
    assert_eq!(stats.total_published, event_count as u64);

    Ok(())
}
