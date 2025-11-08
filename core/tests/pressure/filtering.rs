//! Event filtering tests under load
//!
//! Tests covering event type filtering performance.

use super::make_event;
use loom_core::event::EventBus;
use loom_core::proto::QoSLevel;
use loom_core::Result;
use serial_test::serial;

/// Test: Event type filtering under load
#[tokio::test]
#[serial]
pub async fn event_type_filtering() -> Result<()> {
    let bus = EventBus::new().await?;
    let topic = "pressure.filter";
    let event_count = 2_000;

    // Subscribe only to "type_a" events
    let (_sub_id, mut rx) = bus
        .subscribe(
            topic.to_string(),
            vec!["type_a".to_string()],
            QoSLevel::QosBatched,
        )
        .await?;

    // Publish 50% type_a, 50% type_b
    for i in 0..event_count {
        let event_type = if i % 2 == 0 { "type_a" } else { "type_b" };
        let evt = make_event(i, event_type);
        bus.publish(topic, evt).await?;
    }

    let mut received = 0;
    while rx.try_recv().is_ok() {
        received += 1;
    }

    println!(
        "Event filtering: published {}, received {} (should be ~50%)",
        event_count, received
    );

    assert_eq!(
        received,
        event_count / 2,
        "Should only receive type_a events"
    );

    Ok(())
}
