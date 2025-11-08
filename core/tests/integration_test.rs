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
//! - `integration::e2e_basic` - Basic end-to-end pipeline tests
//! - `integration::e2e_multi_agent` - Multi-agent interaction tests
//! - `integration::e2e_error_handling` - Error propagation tests
//! - `integration::e2e_routing` - Routing decision tests
//! - `integration::e2e_action_broker` - ActionBroker-specific tests
//!
//! All mock components (MockEchoProvider, MockSlowProvider, MockFailingProvider,
//! MockEchoBehavior) are defined in `integration::mod.rs` for reuse across tests.

mod integration;
