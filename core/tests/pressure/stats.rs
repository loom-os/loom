//! Statistics tracking tests
//!
//! Tests verifying the accuracy of EventBus statistics under load.

use super::make_event;
use loom_core::event::EventBus;
use loom_core::proto::QoSLevel;
use loom_core::Result;
use serial_test::serial;

/// Test: Stats accuracy under load
#[tokio::test]
#[serial]
pub async fn stats_accuracy() -> Result<()> {
    let bus = EventBus::new().await?;
    let topic = "pressure.stats";
    let event_count = 1_000;

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Publish
    for i in 0..event_count {
        let evt = make_event(i, "stats_test");
        bus.publish(topic, evt).await?;
    }

    // Consume
    let mut received = 0;
    while rx.try_recv().is_ok() {
        received += 1;
    }

    let stats = bus.get_stats(topic).expect("stats");

    println!(
        "Stats: published={}, delivered={}, received={}, dropped={}",
        stats.total_published, stats.total_delivered, received, stats.dropped_events
    );

    assert_eq!(stats.total_published, event_count as u64);
    assert_eq!(stats.total_delivered, received as u64);
    assert_eq!(stats.dropped_events, 0);
    assert_eq!(received, event_count);

    Ok(())
}
