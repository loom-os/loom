/// EventBus Performance Benchmarks using Criterion
///
/// Run with: cargo bench --bench event_bus_benchmark
///
/// Benchmarks cover:
/// - Single publisher throughput
/// - Concurrent publishers throughput
/// - Different QoS levels
/// - Latency measurements
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use loom_core::event::EventBus;
use loom_core::proto::{Event, QoSLevel};
use std::sync::Arc;

fn make_event(id: u64, event_type: &str) -> Event {
    Event {
        id: format!("evt_{}", id),
        r#type: event_type.to_string(),
        timestamp_ms: 0,
        source: "benchmark".to_string(),
        metadata: Default::default(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

/// Benchmark: Single publisher, single consumer throughput
fn bench_single_publisher(c: &mut Criterion) {
    let mut group = c.benchmark_group("eventbus_single_publisher");

    for event_count in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*event_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(event_count),
            event_count,
            |b, &count| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let bus = EventBus::new().await.unwrap();
                        let topic = "bench.single";

                        let (_sub_id, mut rx) = bus
                            .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
                            .await
                            .unwrap();

                        // Consumer task
                        let consumer = tokio::spawn(async move {
                            let mut received = 0;
                            while let Some(_) = rx.recv().await {
                                received += 1;
                                if received >= count {
                                    break;
                                }
                            }
                        });

                        // Publish
                        for i in 0..count {
                            let evt = make_event(i as u64, "bench");
                            bus.publish(topic, evt).await.unwrap();
                        }

                        consumer.await.unwrap();
                        black_box(bus);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Concurrent publishers
fn bench_concurrent_publishers(c: &mut Criterion) {
    let mut group = c.benchmark_group("eventbus_concurrent_publishers");

    for publisher_count in [2, 4, 8].iter() {
        let events_per_publisher = 500;
        let total_events = publisher_count * events_per_publisher;

        group.throughput(Throughput::Elements(total_events as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", publisher_count, events_per_publisher)),
            publisher_count,
            |b, &pubs| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let bus = Arc::new(EventBus::new().await.unwrap());
                        let topic = "bench.concurrent";

                        let (_sub_id, mut rx) = bus
                            .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
                            .await
                            .unwrap();

                        // Consumer
                        let total = pubs * events_per_publisher;
                        let consumer = tokio::spawn(async move {
                            let mut received = 0;
                            while let Some(_) = rx.recv().await {
                                received += 1;
                                if received >= total {
                                    break;
                                }
                            }
                        });

                        // Publishers
                        let mut tasks = vec![];
                        for p in 0..pubs {
                            let bus_clone = bus.clone();
                            let topic_str = topic.to_string();
                            tasks.push(tokio::spawn(async move {
                                for i in 0..events_per_publisher {
                                    let evt =
                                        make_event((p * events_per_publisher + i) as u64, "bench");
                                    let _ = bus_clone.publish(&topic_str, evt).await;
                                }
                            }));
                        }

                        for task in tasks {
                            task.await.unwrap();
                        }

                        consumer.await.unwrap();
                        black_box(bus);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Different QoS levels
fn bench_qos_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("eventbus_qos_levels");
    let event_count = 1_000;

    for qos in [
        QoSLevel::QosRealtime,
        QoSLevel::QosBatched,
        QoSLevel::QosBackground,
    ]
    .iter()
    {
        let qos_name = match qos {
            QoSLevel::QosRealtime => "realtime",
            QoSLevel::QosBatched => "batched",
            QoSLevel::QosBackground => "background",
        };

        group.throughput(Throughput::Elements(event_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(qos_name),
            qos,
            |b, &qos_level| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let bus = EventBus::new().await.unwrap();
                        let topic = format!("bench.qos.{}", qos_name);

                        let (_sub_id, mut rx) = bus
                            .subscribe(topic.clone(), vec![], qos_level)
                            .await
                            .unwrap();

                        // Consumer
                        let consumer = tokio::spawn(async move {
                            let mut received = 0;
                            while let Some(_) = rx.recv().await {
                                received += 1;
                                if received >= event_count {
                                    break;
                                }
                            }
                        });

                        // Publish
                        for i in 0..event_count {
                            let evt = make_event(i as u64, "qos_bench");
                            let _ = bus.publish(&topic, evt).await;
                        }

                        let _ =
                            tokio::time::timeout(std::time::Duration::from_secs(5), consumer).await;
                        black_box(bus);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Publish latency (single event)
fn bench_publish_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("eventbus_publish_latency");

    group.bench_function("single_event_latency", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bus = EventBus::new().await.unwrap();
                let topic = "bench.latency";

                let (_sub_id, _rx) = bus
                    .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
                    .await
                    .unwrap();

                let evt = make_event(1, "latency");
                bus.publish(topic, evt).await.unwrap();
                black_box(bus);
            })
        });
    });

    group.finish();
}

/// Benchmark: Multiple subscribers on same topic
fn bench_multiple_subscribers(c: &mut Criterion) {
    let mut group = c.benchmark_group("eventbus_multiple_subscribers");
    let event_count = 500;

    for sub_count in [2, 5, 10].iter() {
        group.throughput(Throughput::Elements(event_count as u64 * *sub_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_subs", sub_count)),
            sub_count,
            |b, &subs| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        let bus = Arc::new(EventBus::new().await.unwrap());
                        let topic = "bench.multi_sub";

                        // Create multiple subscribers
                        let mut consumers = vec![];
                        for _ in 0..subs {
                            let (_sub_id, mut rx) = bus
                                .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
                                .await
                                .unwrap();

                            consumers.push(tokio::spawn(async move {
                                let mut received = 0;
                                while let Some(_) = rx.recv().await {
                                    received += 1;
                                    if received >= event_count {
                                        break;
                                    }
                                }
                            }));
                        }

                        // Publish
                        for i in 0..event_count {
                            let evt = make_event(i as u64, "multi_sub_bench");
                            bus.publish(topic, evt).await.unwrap();
                        }

                        // Wait for all consumers
                        for consumer in consumers {
                            let _ =
                                tokio::time::timeout(std::time::Duration::from_secs(5), consumer)
                                    .await;
                        }

                        black_box(bus);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Event filtering overhead
fn bench_event_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("eventbus_event_filtering");
    let event_count = 1_000;

    group.bench_function("no_filter", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bus = EventBus::new().await.unwrap();
                let topic = "bench.filter";

                let (_sub_id, mut rx) = bus
                    .subscribe(topic.to_string(), vec![], QoSLevel::QosBatched)
                    .await
                    .unwrap();

                let consumer = tokio::spawn(async move {
                    let mut received = 0;
                    while let Some(_) = rx.recv().await {
                        received += 1;
                        if received >= event_count {
                            break;
                        }
                    }
                });

                for i in 0..event_count {
                    let evt = make_event(i as u64, "type_a");
                    bus.publish(topic, evt).await.unwrap();
                }

                consumer.await.unwrap();
                black_box(bus);
            })
        });
    });

    group.bench_function("with_filter", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let bus = EventBus::new().await.unwrap();
                let topic = "bench.filter";

                let (_sub_id, mut rx) = bus
                    .subscribe(
                        topic.to_string(),
                        vec!["type_a".to_string()],
                        QoSLevel::QosBatched,
                    )
                    .await
                    .unwrap();

                let consumer = tokio::spawn(async move {
                    let mut received = 0;
                    while let Some(_) = rx.recv().await {
                        received += 1;
                        if received >= event_count / 2 {
                            break;
                        }
                    }
                });

                // Publish 50% type_a, 50% type_b
                for i in 0..event_count {
                    let event_type = if i % 2 == 0 { "type_a" } else { "type_b" };
                    let evt = make_event(i as u64, event_type);
                    bus.publish(topic, evt).await.unwrap();
                }

                let _ = tokio::time::timeout(std::time::Duration::from_secs(5), consumer).await;
                black_box(bus);
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_publisher,
    bench_concurrent_publishers,
    bench_qos_levels,
    bench_publish_latency,
    bench_multiple_subscribers,
    bench_event_filtering,
);
criterion_main!(benches);
