# EventBus Pressure Test Modules

This directory contains modularized pressure and backpressure tests for the EventBus.

## Module Structure

```
pressure/
├── mod.rs              # Module root with shared utilities
├── throughput.rs       # Throughput-focused tests
├── qos_behavior.rs     # QoS level behavior tests
├── backpressure.rs     # Backpressure mechanism tests
├── latency.rs          # Latency measurement tests
├── filtering.rs        # Event filtering tests
└── stats.rs            # Statistics tracking tests
```

## Test Organization

### `throughput.rs` - Throughput Tests

Baseline and concurrent throughput scenarios:

| Test                               | Description                          | Key Metrics                  |
| ---------------------------------- | ------------------------------------ | ---------------------------- |
| `single_publisher_single_consumer` | Baseline single-threaded throughput  | 10k events, ~175k events/sec |
| `concurrent_publishers`            | Multi-threaded concurrent publishing | 8 publishers × 1k events     |
| `sustained_load`                   | Continuous load over time            | 3 seconds @ 2k events/sec    |
| `multiple_subscribers_delivery`    | Fanout to multiple subscribers       | 5 subscribers × 1k events    |

### `qos_behavior.rs` - QoS Level Tests

Different QoS level behavior validation:

| Test                               | QoS Level  | Buffer Size | Expected Behavior                    |
| ---------------------------------- | ---------- | ----------- | ------------------------------------ |
| `realtime_qos_drops_under_load`    | Realtime   | 64          | Aggressive dropping under load       |
| `batched_qos_buffers_without_drop` | Batched    | 1024        | Reliable buffering within capacity   |
| `background_qos_large_buffer`      | Background | 4096        | Large buffer for non-critical events |

### `backpressure.rs` - Backpressure Tests

Backpressure threshold and strategy validation:

| Test                    | Description                                 | Validation                     |
| ----------------------- | ------------------------------------------- | ------------------------------ |
| `threshold_enforcement` | Tests 10k event threshold with Realtime QoS | Drops when exceeding threshold |

### `latency.rs` - Latency Tests

Latency distribution measurements:

| Test                   | Description                         | Metrics                           |
| ---------------------- | ----------------------------------- | --------------------------------- |
| `latency_distribution` | P50/P99 latency under moderate load | P50 <1ms, P99 <2ms (0-1ms actual) |

### `filtering.rs` - Event Filtering Tests

Event type filtering performance:

| Test                   | Description                     | Validation             |
| ---------------------- | ------------------------------- | ---------------------- |
| `event_type_filtering` | Type-based filtering under load | 50% filtered correctly |

### `stats.rs` - Statistics Tests

Statistics tracking accuracy:

| Test             | Description                                  | Validation           |
| ---------------- | -------------------------------------------- | -------------------- |
| `stats_accuracy` | Validates published/delivered/dropped counts | All metrics accurate |

## Running Tests

### All Pressure Tests

```bash
cd core
cargo test --test event_pressure_test -- --test-threads=1 --nocapture
```

### Specific Module

```bash
# Throughput tests only
cargo test --test event_pressure_test throughput -- --nocapture

# QoS behavior tests only
cargo test --test event_pressure_test qos_behavior -- --nocapture

# Latency tests only
cargo test --test event_pressure_test latency -- --nocapture
```

### Specific Test

```bash
cargo test --test event_pressure_test single_publisher_single_consumer -- --nocapture
cargo test --test event_pressure_test realtime_qos_drops_under_load -- --nocapture
```

## Shared Utilities

The `mod.rs` file provides shared utilities:

- **`make_event(id, event_type)`** - Helper to create test events with minimal overhead

## Design Principles

1. **Modularity**: Tests grouped by category for easy maintenance
2. **Isolation**: Each test is self-contained and uses `serial_test`
3. **Clear Naming**: Test names clearly indicate what is being tested
4. **Metrics Output**: All tests print key metrics for performance tracking
5. **Reasonable Assertions**: Tests validate behavior, not absolute performance

## Adding New Tests

To add a new pressure test:

1. **Choose appropriate module**: Select the category that best fits your test
2. **Add test function**: Use `#[tokio::test]` and `#[serial]` attributes
3. **Use shared utilities**: Import and use `make_event()` from parent module
4. **Print metrics**: Output key metrics for documentation
5. **Document in README**: Update this file with test description

Example:

```rust
// In pressure/throughput.rs

/// Test: Your new test description
#[tokio::test]
#[serial]
pub async fn your_new_test() -> Result<()> {
    use super::make_event;

    let bus = EventBus::new().await?;
    // ... test implementation ...

    println!("Your metrics: {}", value);
    assert!(condition, "Expected behavior");

    Ok(())
}
```

## Test Execution Order

All tests run serially (not in parallel) to avoid resource conflicts. The `#[serial]` attribute from `serial_test` crate ensures this.

## Performance Targets

Based on P0 requirements:

| Metric                | Target         | Current Status      |
| --------------------- | -------------- | ------------------- |
| Throughput            | 10k events/sec | ✅ ~175k events/sec |
| P50 Latency           | <100ms         | ✅ <1ms             |
| P99 Latency           | <500ms         | ✅ ~1ms             |
| Concurrent Publishers | 8+             | ✅ Tested with 8    |
| Backpressure          | Drop/sample    | ✅ Validated        |

## Maintenance Notes

- All tests use `serial_test::serial` to prevent conflicts
- Tests print metrics for documentation and debugging
- Assertions are designed to catch regressions, not be overly strict
- MVP focus: tests validate correctness over absolute performance
