/// EventBus Pressure and Backpressure Tests
///
/// Tests EventBus behavior under high load, different QoS levels,
/// and backpressure strategies (sampling, dropping old events, etc.)
///
/// Note: Use `serial_test` for tests that may conflict on shared resources
use loom_core::event::EventBus;
use loom_core::proto::{Event, QoSLevel};
use loom_core::Result;
use serial_test::serial;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

// Helper to create a test event with minimal overhead
fn make_event(id: u64, event_type: &str) -> Event {
    Event {
        id: format!("evt_{}", id),
        r#type: event_type.to_string(),
        timestamp_ms: 0,
        source: "pressure_test".to_string(),
        metadata: Default::default(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

/// Test: Single publisher, single consumer - baseline throughput
#[tokio::test]
#[serial]
async fn pressure_single_publisher_single_consumer() -> Result<()> {
    let bus = EventBus::new().await?;
    let topic = "pressure.single";
    let event_count = 10_000;

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Consumer task
    let received = Arc::new(AtomicU64::new(0));
    let received_clone = received.clone();
    let consumer = tokio::spawn(async move {
        while let Some(_evt) = rx.recv().await {
            received_clone.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Publisher task
    let start = Instant::now();
    for i in 0..event_count {
        let evt = make_event(i, "load_test");
        bus.publish(topic, evt).await?;
    }
    let publish_duration = start.elapsed();

    // Wait a bit for all events to be consumed
    tokio::time::sleep(Duration::from_millis(500)).await;
    drop(bus); // This will close channels

    let _ = tokio::time::timeout(Duration::from_secs(2), consumer).await;

    let received_count = received.load(Ordering::Relaxed);
    let throughput = event_count as f64 / publish_duration.as_secs_f64();

    println!(
        "Single pub/sub: {} events in {:?} = {:.0} events/sec, received: {}",
        event_count, publish_duration, throughput, received_count
    );

    assert_eq!(
        received_count, event_count,
        "All events should be received in batched mode"
    );
    assert!(
        throughput > 1000.0,
        "Throughput should be reasonable (>1k events/sec)"
    );

    Ok(())
}

/// Test: Multiple concurrent publishers to test thread safety and throughput
#[tokio::test]
#[serial]
async fn pressure_concurrent_publishers() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let topic = "pressure.concurrent";
    let publishers = 8;
    let events_per_publisher = 1_000;
    let total_events = publishers * events_per_publisher;

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    // Consumer
    let received = Arc::new(AtomicU64::new(0));
    let received_clone = received.clone();
    let consumer = tokio::spawn(async move {
        while let Some(_evt) = rx.recv().await {
            received_clone.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Multiple publishers
    let start = Instant::now();
    let mut tasks = JoinSet::new();
    for p in 0..publishers {
        let bus_clone = bus.clone();
        let topic_str = topic.to_string();
        tasks.spawn(async move {
            for i in 0..events_per_publisher {
                let evt = make_event((p * events_per_publisher + i) as u64, "concurrent");
                let _ = bus_clone.publish(&topic_str, evt).await;
            }
        });
    }

    // Wait for all publishers to finish
    while tasks.join_next().await.is_some() {}
    let publish_duration = start.elapsed();

    // Wait for consumption
    tokio::time::sleep(Duration::from_millis(500)).await;
    drop(bus);
    let _ = tokio::time::timeout(Duration::from_secs(2), consumer).await;

    let received_count = received.load(Ordering::Relaxed);
    let throughput = total_events as f64 / publish_duration.as_secs_f64();

    println!(
        "Concurrent ({} publishers x {} events): {} total in {:?} = {:.0} events/sec, received: {}",
        publishers,
        events_per_publisher,
        total_events,
        publish_duration,
        throughput,
        received_count
    );

    assert_eq!(received_count, total_events as u64);
    assert!(throughput > 1000.0);

    Ok(())
}

/// Test: Realtime QoS drops events under backpressure
#[tokio::test]
#[serial]
async fn pressure_realtime_qos_drops_under_load() -> Result<()> {
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
async fn pressure_batched_qos_buffers_without_drop() -> Result<()> {
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
    while let Ok(_) = rx.try_recv() {
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
async fn pressure_background_qos_large_buffer() -> Result<()> {
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

/// Test: Backpressure threshold triggers dropping for realtime
#[tokio::test]
#[serial]
async fn pressure_backpressure_threshold_enforcement() -> Result<()> {
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

/// Test: Multiple subscribers on the same topic all receive events
#[tokio::test]
#[serial]
async fn pressure_multiple_subscribers_delivery() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let topic = "pressure.multi_sub";
    let subscriber_count = 5;
    let event_count = 1_000;

    let mut receivers = Vec::new();
    let mut received_counters = Vec::new();

    for _ in 0..subscriber_count {
        let (_sub_id, rx) = bus
            .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
            .await?;
        let counter = Arc::new(AtomicU64::new(0));
        received_counters.push(counter.clone());

        let consumer = tokio::spawn(async move {
            let mut rx = rx;
            while let Some(_evt) = rx.recv().await {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        receivers.push(consumer);
    }

    // Publish events
    for i in 0..event_count {
        let evt = make_event(i, "multi_sub");
        bus.publish(topic, evt).await?;
    }

    // Wait for consumption
    tokio::time::sleep(Duration::from_millis(500)).await;
    drop(bus);

    for consumer in receivers {
        let _ = tokio::time::timeout(Duration::from_secs(2), consumer).await;
    }

    // Check all subscribers received all events
    for (idx, counter) in received_counters.iter().enumerate() {
        let count = counter.load(Ordering::Relaxed);
        println!("Subscriber {}: received {}", idx, count);
        assert_eq!(
            count, event_count as u64,
            "Each subscriber should receive all events"
        );
    }

    Ok(())
}

/// Test: Measure latency distribution (P50, P99) under moderate load
#[tokio::test]
#[serial]
async fn pressure_latency_distribution() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let topic = "pressure.latency";
    let event_count = 1_000;

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let latencies_clone = latencies.clone();

    // Consumer that measures latency
    let consumer = tokio::spawn(async move {
        while let Some(evt) = rx.recv().await {
            let recv_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let send_time = evt.timestamp_ms as u128;
            if send_time > 0 {
                let latency = recv_time.saturating_sub(send_time);
                latencies_clone.lock().await.push(latency as u64);
            }
        }
    });

    // Publish with timestamps
    for i in 0..event_count {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let mut evt = make_event(i, "latency_test");
        evt.timestamp_ms = now;
        bus.publish(topic, evt).await?;

        // Small delay to avoid overwhelming
        if i % 100 == 0 {
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    drop(bus);
    let _ = tokio::time::timeout(Duration::from_secs(2), consumer).await;

    let mut lat = latencies.lock().await;
    lat.sort_unstable();

    if !lat.is_empty() {
        let p50 = lat[lat.len() / 2];
        let p99 = lat[lat.len() * 99 / 100];
        let max = lat[lat.len() - 1];
        let min = lat[0];

        println!(
            "Latency distribution (ms): min={}, P50={}, P99={}, max={}",
            min, p50, p99, max
        );

        // Basic sanity checks
        assert!(p50 < 1000, "P50 latency should be reasonable (<1s)");
        assert!(p99 < 5000, "P99 latency should be reasonable (<5s)");
    } else {
        println!("Warning: No latency samples collected");
    }

    Ok(())
}

/// Test: Event type filtering under load
#[tokio::test]
#[serial]
async fn pressure_event_type_filtering() -> Result<()> {
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

/// Test: Sustained load over time (mini stress test)
#[tokio::test]
#[serial]
async fn pressure_sustained_load() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let topic = "pressure.sustained";
    let duration = Duration::from_secs(3);
    let target_rate = 2_000; // events per second

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    let received = Arc::new(AtomicU64::new(0));
    let received_clone = received.clone();

    let consumer = tokio::spawn(async move {
        while let Some(_evt) = rx.recv().await {
            received_clone.fetch_add(1, Ordering::Relaxed);
        }
    });

    let bus_clone = bus.clone();
    let publisher = tokio::spawn(async move {
        let start = Instant::now();
        let mut count = 0u64;
        let interval = Duration::from_micros(1_000_000 / target_rate as u64);

        while start.elapsed() < duration {
            let evt = make_event(count, "sustained");
            let _ = bus_clone.publish(topic, evt).await;
            count += 1;
            tokio::time::sleep(interval).await;
        }
        count
    });

    let published = publisher.await.unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;
    drop(bus);
    let _ = tokio::time::timeout(Duration::from_secs(2), consumer).await;

    let received_count = received.load(Ordering::Relaxed);
    let throughput = received_count as f64 / duration.as_secs_f64();

    println!(
        "Sustained load: published {} over {:?}, received {}, throughput {:.0} events/sec",
        published, duration, received_count, throughput
    );

    assert!(
        received_count >= published * 90 / 100,
        "Should receive at least 90% of published events"
    );

    Ok(())
}

/// Test: Stats accuracy under load
#[tokio::test]
#[serial]
async fn pressure_stats_accuracy() -> Result<()> {
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
