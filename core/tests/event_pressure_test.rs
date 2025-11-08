/// EventBus Pressure and Backpressure Tests
///
/// Tests EventBus behavior under high load, different QoS levels,
/// and backpressure strategies (sampling, dropping old events, etc.)
///
/// Test modules are organized by category:
/// - `throughput`: Baseline and concurrent throughput tests
/// - `qos_behavior`: QoS level-specific behavior tests
/// - `backpressure`: Backpressure threshold and dropping tests
/// - `latency`: P50/P99 latency measurement tests
/// - `filtering`: Event type filtering tests
/// - `stats`: Statistics tracking accuracy tests
///
/// Note: Tests use `serial_test` to avoid resource conflicts.
/// Run with: cargo test --test event_pressure_test -- --test-threads=1
mod pressure;
