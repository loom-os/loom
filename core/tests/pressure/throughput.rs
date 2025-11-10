//! Throughput-focused pressure tests
//!
//! Tests covering baseline throughput, concurrent publishers, and sustained load scenarios.

use super::make_event;
use loom_core::event::EventBus;
use loom_core::proto::QoSLevel;
use loom_core::Result;
use serial_test::serial;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

/// Test: Single publisher, single consumer - baseline throughput
#[tokio::test]
#[serial]
pub async fn single_publisher_single_consumer() -> Result<()> {
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
        while received_clone.load(Ordering::Relaxed) < event_count as u64 {
            if rx.recv().await.is_none() {
                break;
            }
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
pub async fn concurrent_publishers() -> Result<()> {
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
        while received_clone.load(Ordering::Relaxed) < total_events as u64 {
            if rx.recv().await.is_none() {
                break;
            }
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

/// Test: Sustained load over time (mini stress test)
#[tokio::test]
#[serial]
pub async fn sustained_load() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let topic = "pressure.sustained";
    let duration = Duration::from_secs(3);
    let target_rate = 2_000; // events per second

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    let received = Arc::new(AtomicU64::new(0));
    let received_clone = received.clone();
    // the `published` variable is inside the publisher task
    let consumer = tokio::spawn(async move {
        // we can't know the published count in advance here; we must rely on the main flow dropping the bus to close the channels
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

/// Test: Multiple subscribers on the same topic all receive events
#[tokio::test]
#[serial]
pub async fn multiple_subscribers_delivery() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let topic = "pressure.multi_sub";
    let subscriber_count = 5;
    let event_count = 1_000;

    let mut receivers = Vec::new();
    let mut received_counters = Vec::new();

    for _ in 0..subscriber_count {
        let (_sub_id, mut rx) = bus
            .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
            .await?;
        let counter = Arc::new(AtomicU64::new(0));
        received_counters.push(counter.clone());

        let consumer = tokio::spawn(async move {
            while counter.load(Ordering::Relaxed) < event_count as u64 {
                if rx.recv().await.is_none() {
                    break;
                }
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
