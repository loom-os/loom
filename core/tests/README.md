# Loom Core Unit Tests

This directory contains unit tests for the core Loom modules.

## Test Structure

Each test file follows the naming convention `<module>_test.rs` and corresponds to a source module in `core/src/`:

| Test File               | Source Module          | Coverage                                                       |
| ----------------------- | ---------------------- | -------------------------------------------------------------- |
| `event_test.rs`         | `src/event.rs`         | EventBus pub/sub, QoS levels, backpressure strategies          |
| `action_broker_test.rs` | `src/action_broker.rs` | Capability registration, invocation, timeout, error handling   |
| `agent_runtime_test.rs` | `src/agent/runtime.rs` | Agent lifecycle, mailbox distribution, multi-agent scenarios   |
| `router_test.rs`        | `src/router.rs`        | Model routing decisions, privacy levels, confidence thresholds |
| `llm_test.rs`           | `src/llm/`             | LLM client config, adapter logic, token budget enforcement     |
| `integration_test.rs`   | Core Pipeline          | End-to-end event → agent → action → result flow                |

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
cargo test --test action_broker_test
cargo test --test agent_runtime_test
cargo test --test router_test
cargo test --test llm_test
cargo test --test integration_test

# Run specific test case
cargo test --test event_test subscribe_and_receive
cargo test --test integration_test test_e2e_event_to_action_to_result
```

## Test Coverage Summary

### Unit Tests

- **EventBus**: 10 tests - pub/sub, QoS, backpressure, filtering, stats
- **ActionBroker**: 9 tests - registration, invocation, timeout, errors, idempotency
- **AgentRuntime**: 8 tests - lifecycle, mailbox, subscriptions, multi-agent
- **ModelRouter**: 14 tests - privacy routing, confidence thresholds, policy decisions
- **LlmClient**: 8 tests - config, adapter, token budgets, tools schema

**Total Unit Tests**: 49

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

**Grand Total**: 56 tests

## Notes

- All tests use `tokio::test` for async support
- Mock implementations are defined inline for isolation
- Tests focus on observable behavior rather than internal state (MVP approach)
- Token budget truncation logic in `llm/adapter.rs` was fixed during test development
