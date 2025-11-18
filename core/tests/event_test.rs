use loom_core::event::EventBus;
use loom_core::proto::{Event, QoSLevel};
use loom_core::Result;

// Helper to create a test event
fn make_event(id: &str, event_type: &str) -> Event {
    Event {
        id: id.to_string(),
        r#type: event_type.to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: Default::default(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

#[tokio::test]
async fn subscribe_and_publish_basic() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub_id, mut rx) = bus
        .subscribe("topic.test".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    let evt = make_event("e1", "unit");
    bus.publish("topic.test", evt.clone()).await?;

    let received = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(received.id, "e1");
    Ok(())
}

#[tokio::test]
async fn unsubscribe_stops_receiving_events() -> Result<()> {
    let bus = EventBus::new().await?;
    let (sub_id, mut rx) = bus
        .subscribe("topic.unsub".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Publish before unsubscribe
    let evt1 = make_event("before", "unit");
    bus.publish("topic.unsub", evt1).await?;

    // Unsubscribe
    bus.unsubscribe(&sub_id).await?;

    // Publish after unsubscribe
    let evt2 = make_event("after", "unit");
    bus.publish("topic.unsub", evt2).await?;

    // Should receive the first event
    let first = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
        .await
        .expect("timeout")
        .expect("channel closed");
    assert_eq!(first.id, "before");

    // Should NOT receive the second event (channel should close or timeout)
    let second = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
    assert!(
        second.is_err() || second.unwrap().is_none(),
        "should not receive after unsubscribe"
    );
    Ok(())
}

#[tokio::test]
async fn event_type_filtering_works() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub_id, mut rx) = bus
        .subscribe(
            "topic.filter".to_string(),
            vec!["type_a".to_string()],
            QoSLevel::QosBatched,
        )
        .await?;

    // Publish type_a and type_b
    bus.publish("topic.filter", make_event("a1", "type_a"))
        .await?;
    bus.publish("topic.filter", make_event("b1", "type_b"))
        .await?;
    bus.publish("topic.filter", make_event("a2", "type_a"))
        .await?;

    // Should only receive type_a events
    let r1 = rx.recv().await.expect("channel closed");
    assert_eq!(r1.id, "a1");
    let r2 = rx.recv().await.expect("channel closed");
    assert_eq!(r2.id, "a2");

    // No more events (type_b was filtered)
    let r3 = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
    assert!(r3.is_err(), "type_b should have been filtered");
    Ok(())
}

#[tokio::test]
async fn qos_realtime_drops_under_backpressure() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub_id, mut rx) = bus
        .subscribe("topic.rt".to_string(), vec![], QoSLevel::QosRealtime)
        .await?;

    // Publish many events without consuming to fill the channel
    for i in 0..500 {
        let evt = make_event(&format!("rt{}", i), "unit");
        let _ = bus.publish("topic.rt", evt).await;
    }

    // Drain some events
    let mut received_count = 0;
    while rx.try_recv().is_ok() {
        received_count += 1;
    }

    // Check stats: should see drops
    let stats = bus.get_stats("topic.rt").expect("stats exist");
    assert!(
        stats.dropped_events > 0,
        "expected drops for realtime QoS under backpressure"
    );
    assert!(received_count < 500, "not all events should be received");
    Ok(())
}

#[tokio::test]
async fn qos_batched_queues_without_drop_within_capacity() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub_id, mut rx) = bus
        .subscribe("topic.batch".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Publish fewer than the channel capacity (1024 default for batched)
    for i in 0..100 {
        let evt = make_event(&format!("b{}", i), "unit");
        bus.publish("topic.batch", evt).await?;
    }

    // Drain all
    let mut received_count = 0;
    while let Ok(_) = rx.try_recv() {
        received_count += 1;
    }

    let stats = bus.get_stats("topic.batch").expect("stats exist");
    assert_eq!(
        stats.dropped_events, 0,
        "batched should not drop within capacity"
    );
    assert_eq!(received_count, 100);
    Ok(())
}

#[tokio::test]
async fn publish_to_empty_topic_returns_zero() -> Result<()> {
    let bus = EventBus::new().await?;
    let evt = make_event("orphan", "unit");
    let delivered = bus.publish("topic.empty", evt).await?;
    assert_eq!(delivered, 0, "no subscribers should mean zero delivery");
    Ok(())
}

#[tokio::test]
async fn multiple_subscribers_on_same_topic() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub1, mut rx1) = bus
        .subscribe("topic.multi".to_string(), vec![], QoSLevel::QosBatched)
        .await?;
    let (_sub2, mut rx2) = bus
        .subscribe("topic.multi".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    let evt = make_event("multi", "unit");
    let delivered = bus.publish("topic.multi", evt).await?;
    assert_eq!(delivered, 2, "both subscribers should receive");

    let r1 = rx1.recv().await.expect("rx1 closed");
    let r2 = rx2.recv().await.expect("rx2 closed");
    assert_eq!(r1.id, "multi");
    assert_eq!(r2.id, "multi");
    Ok(())
}

#[tokio::test]
async fn duplicate_subscribe_to_same_topic_creates_separate_channels() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub1, mut rx1) = bus
        .subscribe("topic.dup".to_string(), vec![], QoSLevel::QosBatched)
        .await?;
    let (_sub2, mut rx2) = bus
        .subscribe("topic.dup".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    let evt = make_event("dup", "unit");
    bus.publish("topic.dup", evt).await?;

    // Both channels should receive independently
    let r1 = rx1.recv().await.expect("rx1");
    let r2 = rx2.recv().await.expect("rx2");
    assert_eq!(r1.id, "dup");
    assert_eq!(r2.id, "dup");
    Ok(())
}

#[tokio::test]
async fn stats_track_published_and_delivered() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub_id, mut rx) = bus
        .subscribe("topic.stats".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    for i in 0..10 {
        let evt = make_event(&format!("s{}", i), "unit");
        bus.publish("topic.stats", evt).await?;
    }

    // Drain
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }

    let stats = bus.get_stats("topic.stats").expect("stats");
    assert_eq!(stats.total_published, 10);
    assert_eq!(stats.total_delivered, 10);
    assert_eq!(count, 10);
    Ok(())
}

#[tokio::test]
async fn shutdown_clears_subscriptions() -> Result<()> {
    let bus = EventBus::new().await?;
    let (_sub_id, mut rx) = bus
        .subscribe("topic.shut".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    bus.shutdown().await?;

    // After shutdown, publish should not crash but also no delivery
    let evt = make_event("post_shut", "unit");
    let delivered = bus.publish("topic.shut", evt).await?;
    assert_eq!(delivered, 0, "subscriptions cleared after shutdown");

    // Channel should be closed or empty
    let r = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
    assert!(r.is_err() || r.unwrap().is_none());
    Ok(())
}

#[tokio::test]
async fn wildcard_subscription_matches_subtopics() -> Result<()> {
    let bus = EventBus::new().await?;

    // Subscribe with wildcard pattern
    let (_sub_id, mut rx) = bus
        .subscribe("market.price.*".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Publish to specific subtopics
    let evt_btc = make_event("btc-price", "price.update");
    bus.publish("market.price.BTC", evt_btc).await?;

    let evt_eth = make_event("eth-price", "price.update");
    bus.publish("market.price.ETH", evt_eth).await?;

    // Verify both events received
    let received1 = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
        .await
        .expect("timeout waiting for BTC event")
        .expect("channel closed");
    assert_eq!(received1.id, "btc-price");

    let received2 = tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv())
        .await
        .expect("timeout waiting for ETH event")
        .expect("channel closed");
    assert_eq!(received2.id, "eth-price");

    Ok(())
}

#[tokio::test]
async fn wildcard_does_not_match_wrong_prefix() -> Result<()> {
    let bus = EventBus::new().await?;

    // Subscribe to market.price.*
    let (_sub_id, mut rx) = bus
        .subscribe("market.price.*".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Publish to different prefix
    let evt = make_event("wrong", "other");
    bus.publish("market.volume.BTC", evt).await?;

    // Should timeout - no event received
    let result = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await;
    assert!(
        result.is_err(),
        "Should not receive events from non-matching topic"
    );

    Ok(())
}

#[tokio::test]
async fn exact_and_wildcard_both_receive() -> Result<()> {
    let bus = EventBus::new().await?;

    // Subscribe with exact topic
    let (_sub_id1, mut rx1) = bus
        .subscribe("market.price.BTC".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Subscribe with wildcard
    let (_sub_id2, mut rx2) = bus
        .subscribe("market.price.*".to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Publish to specific topic
    let evt = make_event("dual", "price.update");
    bus.publish("market.price.BTC", evt).await?;

    // Both should receive
    let r1 = tokio::time::timeout(std::time::Duration::from_millis(500), rx1.recv())
        .await
        .expect("timeout rx1")
        .expect("rx1 closed");
    let r2 = tokio::time::timeout(std::time::Duration::from_millis(500), rx2.recv())
        .await
        .expect("timeout rx2")
        .expect("rx2 closed");

    assert_eq!(r1.id, "dual");
    assert_eq!(r2.id, "dual");

    Ok(())
}
