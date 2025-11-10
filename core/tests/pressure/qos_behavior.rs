//! QoS level behavior tests
//!
//! Tests covering different QoS levels: Realtime, Batched, and Background.

use super::make_event;
use loom_core::event::EventBus;
use loom_core::proto::QoSLevel;
use loom_core::Result;
use serial_test::serial;

/// Test: Realtime QoS drops events under backpressure
#[tokio::test]
#[serial]
pub async fn realtime_qos_drops_under_load() -> Result<()> {
    let bus = EventBus::new().await?;
    let topic = "pressure.realtime_drop";
    let event_count = 5_000;

    // Realtime subscription with small buffer (64)
    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosRealtime)
        .await?;

    // Publish rapidly without consuming
    for i in 0..event_count {
        let evt = make_event(i, "realtime_load");
        let _ = bus.publish(topic, evt).await;
    }

    // Now try to consume
    let mut received = 0;
    while rx.try_recv().is_ok() {
        received += 1;
    }

    let stats = bus.get_stats(topic).expect("stats should exist");

    println!(
        "Realtime QoS: published {}, received {}, dropped {}",
        event_count, received, stats.dropped_events
    );

    assert!(
        stats.dropped_events > 0,
        "Realtime QoS should drop events under backpressure"
    );
    assert!(
        received < event_count,
        "Not all events should be received due to drops"
    );
    assert_eq!(
        stats.total_published, event_count as u64,
        "All events should be counted as published"
    );

    Ok(())
}

/// Test: Batched QoS should buffer without drops (within capacity)
#[tokio::test]
#[serial]
pub async fn batched_qos_buffers_without_drop() -> Result<()> {
    let bus = EventBus::new().await?;
    let topic = "pressure.batched";
    let event_count = 500; // Well within 1024 capacity

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Publish without consuming first
    for i in 0..event_count {
        let evt = make_event(i, "batched_load");
        bus.publish(topic, evt).await?;
    }

    // Now consume all
    let mut received = 0;
    while rx.try_recv().is_ok() {
      received += 1;
    }

    let stats = bus.get_stats(topic).expect("stats");

    println!(
        "Batched QoS: published {}, received {}, dropped {}",
        event_count, received, stats.dropped_events
    );

    assert_eq!(
        stats.dropped_events, 0,
        "Batched should not drop within capacity"
    );
    assert_eq!(received, event_count);
    assert_eq!(stats.total_delivered, event_count as u64);

    Ok(())
}

/// Test: Background QoS has largest buffer (4096)
#[tokio::test]
#[serial]
pub async fn background_qos_large_buffer() -> Result<()> {
    let bus = EventBus::new().await?;
    let topic = "pressure.background";
    let event_count = 2_000; // Less than 4096 capacity

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBackground)
        .await?;

    for i in 0..event_count {
        let evt = make_event(i, "bg_load");
        bus.publish(topic, evt).await?;
    }

    let mut received = 0;
    while rx.try_recv().is_ok() {
        received += 1;
    }

    let stats = bus.get_stats(topic).expect("stats");

    println!(
        "Background QoS: published {}, received {}, dropped {}",
        event_count, received, stats.dropped_events
    );

    assert_eq!(stats.dropped_events, 0);
    assert_eq!(received, event_count);

    Ok(())
}
