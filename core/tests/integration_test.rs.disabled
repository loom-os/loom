//! Core Integration Tests
//!
//! End-to-end flow: Event → Agent → ActionBroker → Result → EventBus
//!
//! This test suite validates the minimal pipeline:
//! 1. Publish event to EventBus
//! 2. Agent behavior processes event
//! 3. ActionBroker executes capability
//! 4. Result event published back to EventBus
//! 5. Routing decision events are observed
//!
//! ## Test Organization
//!
//! Tests are organized into submodules by functionality:
//! - `test_e2e_event_to_action_to_result`, `test_e2e_event_type_filtering` - Basic end-to-end pipeline tests
//! - `test_multiple_agents_different_topics` - Multi-agent interaction tests
//! - `test_action_broker_error_propagation` - Error propagation tests
//! - `test_routing_decision_with_privacy_policy` - Routing decision tests
//! - `test_action_timeout_handling`, `test_idempotent_action_invocation` - ActionBroker-specific tests
//!
//! All mock components (MockEchoProvider, MockSlowProvider, MockFailingProvider,
//! MockEchoBehavior) are defined in `integration::mod.rs` for reuse across tests.

mod integration;
