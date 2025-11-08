# Loom Core Unit Tests

This directory contains unit tests for the core Loom modules.

## Test Structure

Each test file follows the naming convention `<module>_test.rs` and corresponds to a source module in `core/src/`:

| Test File                | Source Module          | Coverage                                                       |
| ------------------------ | ---------------------- | -------------------------------------------------------------- |
| `event_test.rs`          | `src/event.rs`         | EventBus pub/sub, QoS levels, backpressure strategies          |
| `event_pressure_test.rs` | `src/event.rs`         | EventBus pressure testing (modularized in `pressure/`)         |
| `action_broker_test.rs`  | `src/action_broker.rs` | Capability registration, invocation, timeout, error handling   |
| `agent_runtime_test.rs`  | `src/agent/runtime.rs` | Agent lifecycle, mailbox distribution, multi-agent scenarios   |
| `router_test.rs`         | `src/router.rs`        | Model routing decisions, privacy levels, confidence thresholds |
| `llm_test.rs`            | `src/llm/`             | LLM client config, adapter logic, token budget enforcement     |
| `integration_test.rs`    | Core Pipeline          | End-to-end event → agent → action → result flow                |

### Pressure Test Structure (Modularized)

Pressure tests are organized into submodules under `pressure/`:

| Module                   | File                       | Coverage                               |
| ------------------------ | -------------------------- | -------------------------------------- |
| `pressure::mod`          | `pressure/mod.rs`          | Shared utilities (`make_event()`)      |
| `pressure::throughput`   | `pressure/throughput.rs`   | Baseline & concurrent throughput tests |
| `pressure::qos_behavior` | `pressure/qos_behavior.rs` | QoS level-specific behavior tests      |
| `pressure::backpressure` | `pressure/backpressure.rs` | Backpressure threshold enforcement     |
| `pressure::latency`      | `pressure/latency.rs`      | P50/P99 latency measurements           |
| `pressure::filtering`    | `pressure/filtering.rs`    | Event type filtering under load        |
| `pressure::stats`        | `pressure/stats.rs`        | Statistics tracking accuracy           |

### Integration Test Structure

Integration tests are organized into submodules under `integration/`:

| Module                            | File                                | Coverage                                      |
| --------------------------------- | ----------------------------------- | --------------------------------------------- |
| `integration::mod`                | `integration/mod.rs`                | Shared mock components (providers, behaviors) |
| `integration::e2e_basic`          | `integration/e2e_basic.rs`          | Basic pipeline, event filtering               |
| `integration::e2e_multi_agent`    | `integration/e2e_multi_agent.rs`    | Multi-agent topic routing                     |
| `integration::e2e_error_handling` | `integration/e2e_error_handling.rs` | Error propagation                             |
| `integration::e2e_routing`        | `integration/e2e_routing.rs`        | Routing decisions, privacy policies           |
| `integration::e2e_action_broker`  | `integration/e2e_action_broker.rs`  | Timeout handling, idempotency                 |

## Running Tests

```bash
# Run all core unit tests
cargo test --lib --tests

# Run specific test file
cargo test --test event_test
cargo test --test event_pressure_test
cargo test --test action_broker_test
cargo test --test agent_runtime_test
cargo test --test router_test
cargo test --test llm_test
cargo test --test integration_test

# Run specific test case
cargo test --test event_test subscribe_and_receive
cargo test --test integration_test test_e2e_event_to_action_to_result
```

## Pressure Tests & Benchmarks

### Quick Start

Run the comprehensive pressure test suite and benchmarks:

```bash
cd core/tests
./run_pressure_tests.sh
```

This generates a performance report at `tests/PRESSURE_TEST_REPORT.md`.

### Running Pressure Tests

**Important:** Run pressure tests serially to avoid resource conflicts:

```bash
cargo test --test event_pressure_test -- --test-threads=1 --nocapture
```

The `--nocapture` flag shows detailed metrics output.

**Run specific module:**

```bash
# Throughput tests only
cargo test --test event_pressure_test throughput -- --nocapture

# QoS behavior tests only
cargo test --test event_pressure_test qos_behavior -- --nocapture

# Latency tests only
cargo test --test event_pressure_test latency -- --nocapture
```

See `pressure/README.md` for detailed module documentation.

### Running Benchmarks

```bash
# All benchmarks (takes 5-10 minutes)
cd core
cargo bench --bench event_bus_benchmark

# Quick test (faster, less accurate)
cargo bench --bench event_bus_benchmark -- --test

# Specific benchmark
cargo bench --bench event_bus_benchmark single_publisher
```

### Performance Targets

| Metric                | Target         | Status              |
| --------------------- | -------------- | ------------------- |
| Throughput            | 10k events/sec | ✅ ~175k events/sec |
| P50 Latency           | <100ms         | ✅ <1ms             |
| P99 Latency           | <500ms         | ✅ ~1ms             |
| Concurrent Publishers | 8+             | ✅ Tested           |
| Backpressure          | Drop/sample    | ✅ Implemented      |

See `PRESSURE_TEST_REPORT_TEMPLATE.md` for detailed report format.

## Test Coverage Summary

### Unit Tests

- **EventBus**: 10 tests - pub/sub, QoS, backpressure, filtering, stats
- **EventBus Pressure**: 11 tests - throughput, latency, concurrency, backpressure strategies
- **ActionBroker**: 9 tests - registration, invocation, timeout, errors, idempotency
- **AgentRuntime**: 8 tests - lifecycle, mailbox, subscriptions, multi-agent
- **ModelRouter**: 14 tests - privacy routing, confidence thresholds, policy decisions
- **LlmClient**: 8 tests - config, adapter, token budgets, tools schema

**Total Unit Tests**: 60

### Integration Tests

- **End-to-End Pipeline**: 7 tests - complete event flow validation
  1. `test_e2e_event_to_action_to_result` - Minimal pipeline: Event → Agent → ActionBroker → Result → EventBus
  2. `test_multiple_agents_different_topics` - Multiple agents with different topics
  3. `test_action_broker_error_propagation` - Error propagation and handling
  4. `test_routing_decision_with_privacy_policy` - Routing decision events
  5. `test_action_timeout_handling` - Action timeout handling
  6. `test_idempotent_action_invocation` - Idempotent action invocation caching
  7. `test_e2e_event_type_filtering` - Event type filtering in subscriptions

**Total Integration Tests**: 7

### Benchmarks

- Single publisher throughput (100, 1k, 10k events)
- Concurrent publishers (2, 4, 8 publishers)
- QoS levels comparison (Realtime, Batched, Background)
- Single event publish latency
- Multiple subscribers (2, 5, 10)
- Event filtering overhead

**Grand Total**: 67 tests + 6 benchmark suites

## Notes

- All tests use `tokio::test` for async support
- Mock implementations are defined inline for isolation
- Tests focus on observable behavior rather than internal state (MVP approach)
- Pressure tests use `serial_test` to avoid resource conflicts
- Benchmarks use Criterion.rs for statistical analysis
- Token budget truncation logic in `llm/adapter.rs` was fixed during test development
