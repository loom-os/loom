# Loom Core Unit Tests

This directory contains unit tests for the core Loom modules.

## Test Structure

Each test file follows the naming convention `<module>_test.rs` and corresponds to a source module in `core/src/`:

| Test File | Source Module | Coverage |
|-----------|---------------|----------|
| `event_test.rs` | `src/event.rs` | EventBus pub/sub, QoS levels, backpressure strategies |
| `action_broker_test.rs` | `src/action_broker.rs` | Capability registration, invocation, timeout, error handling |
| `agent_runtime_test.rs` | `src/agent/runtime.rs` | Agent lifecycle, mailbox distribution, multi-agent scenarios |
| `router_test.rs` | `src/router.rs` | Model routing decisions, privacy levels, confidence thresholds |
| `llm_test.rs` | `src/llm/` | LLM client config, adapter logic, token budget enforcement |

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

# Run specific test case
cargo test --test event_test subscribe_and_receive
```

## Test Coverage Summary

- **EventBus**: 10 tests - pub/sub, QoS, backpressure, filtering, stats
- **ActionBroker**: 9 tests - registration, invocation, timeout, errors, idempotency
- **AgentRuntime**: 8 tests - lifecycle, mailbox, subscriptions, multi-agent
- **ModelRouter**: 14 tests - privacy routing, confidence thresholds, policy decisions
- **LlmClient**: 8 tests - config, adapter, token budgets, tools schema

**Total**: 49 unit tests

## Notes

- All tests use `tokio::test` for async support
- Mock implementations are defined inline for isolation
- Tests focus on observable behavior rather than internal state (MVP approach)
- Token budget truncation logic in `llm/adapter.rs` was fixed during test development
