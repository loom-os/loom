use loom_core::{event::EventBus, proto::Event, Result};

#[tokio::test]
async fn realtime_drops_under_pressure_and_counters_update() -> Result<()> {
    let bus = EventBus::new().await?;
    // Set up a realtime subscriber with small capacity (64 by default)
    let (_sub_id, mut rx) = bus
        .subscribe("topic.rt".to_string(), vec![], loom_core::proto::QoSLevel::QosRealtime)
        .await?;

    // Do not consume from rx to force the sender-side queue to fill, causing drops
    // Publish far more than the channel buffer
    for i in 0..1000u32 {
        let ev = Event {
            id: format!("e{:_>04}", i),
            r#type: "unit".into(),
            timestamp_ms: 0,
            source: "test".into(),
            metadata: Default::default(),
            payload: vec![],
            confidence: 1.0,
            tags: vec![],
            priority: 0,
        };
        let _ = bus.publish("topic.rt", ev).await?;
    }

    // Drain a few to avoid unused var warning
    let _ = rx.try_recv().ok();

    let stats = bus.get_stats("topic.rt").expect("stats exist");
    assert!(stats.total_published >= 1000);
    assert!(stats.dropped_events > 0, "expected some drops for realtime under backpressure");
    Ok(())
}

#[tokio::test]
async fn batched_queues_without_unbounded_growth() -> Result<()> {
    let bus = EventBus::new().await?;
    // Batched subscriber uses capacity 1024 by default; we will publish less than that to avoid drops
    let (_sub_id, mut rx) = bus
        .subscribe("topic.batch".to_string(), vec![], loom_core::proto::QoSLevel::QosBatched)
        .await?;

    for i in 0..200u32 {
        let ev = Event {
            id: format!("b{:_>04}", i),
            r#type: "unit".into(),
            timestamp_ms: 0,
            source: "test".into(),
            metadata: Default::default(),
            payload: vec![],
            confidence: 1.0,
            tags: vec![],
            priority: 0,
        };
        let _ = bus.publish("topic.batch", ev).await?;
    }

    // Receive some to progress the channel
    let mut received = 0u32;
    while let Ok(_e) = rx.try_recv() {
        received += 1;
    }

    let stats = bus.get_stats("topic.batch").expect("stats exist");
    assert_eq!(stats.dropped_events, 0, "batched should queue without drop within capacity");
    assert!(stats.total_delivered >= received as u64);
    Ok(())
}
