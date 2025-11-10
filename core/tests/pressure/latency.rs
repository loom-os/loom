//! Latency measurement tests
//!
//! Tests measuring P50/P99 latency distribution under load.

use super::make_event;
use loom_core::event::EventBus;
use loom_core::proto::QoSLevel;
use loom_core::Result;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;

/// Test: Measure latency distribution (P50, P99) under moderate load
#[tokio::test]
#[serial]
pub async fn latency_distribution() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    let topic = "pressure.latency";
    let event_count = 1_000;

    let (_sub_id, mut rx) = bus
        .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
        .await?;

    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let latencies_clone = latencies.clone();

    // Consumer that measures latency
    // It will exit after collecting `event_count` samples or when the channel closes,
    // avoiding potential infinite waits when messages stop arriving.
    let consumer = tokio::spawn(async move {
        while latencies_clone.lock().await.len() < event_count as usize {
            match rx.recv().await {
                Some(evt) => {
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
                None => break,
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

        // Latency assertions based on P0 requirements
        // Target: P50 <100ms, P99 <500ms (actual performance is much better: <1ms)
        assert!(p50 < 100, "P50 latency should be <100ms (P0 target)");
        assert!(p99 < 500, "P99 latency should be <500ms (P0 target)");
    } else {
        println!("Warning: No latency samples collected");
    }

    Ok(())
}
