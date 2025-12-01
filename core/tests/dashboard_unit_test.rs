//! Dashboard Unit Tests
//!
//! Comprehensive unit tests for Dashboard components:
//! - EventBroadcaster: SSE event broadcasting
//! - FlowTracker: Flow graph tracking and cleanup
//! - TopologyBuilder: Agent topology snapshot generation
//! - DashboardConfig: Configuration management

use loom_core::agent::directory::AgentDirectory;
use loom_core::dashboard::{
    DashboardConfig, DashboardEvent, DashboardEventType, EventBroadcaster, FlowTracker, NodeType,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

// =============================================================================
// EventBroadcaster Tests
// =============================================================================

#[tokio::test]
async fn broadcaster_creates_with_capacity() {
    let broadcaster = EventBroadcaster::new(100);
    assert_eq!(broadcaster.subscriber_count(), 0);
}

#[tokio::test]
async fn broadcaster_accepts_subscriptions() {
    let broadcaster = EventBroadcaster::new(8);
    let _rx1 = broadcaster.subscribe();
    assert_eq!(broadcaster.subscriber_count(), 1);

    let _rx2 = broadcaster.subscribe();
    assert_eq!(broadcaster.subscriber_count(), 2);

    let _rx3 = broadcaster.subscribe();
    assert_eq!(broadcaster.subscriber_count(), 3);
}

#[tokio::test]
async fn broadcaster_delivers_to_all_subscribers() {
    let broadcaster = EventBroadcaster::new(16);

    let mut rx1 = broadcaster.subscribe();
    let mut rx2 = broadcaster.subscribe();
    let mut rx3 = broadcaster.subscribe();

    let event = DashboardEvent {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type: DashboardEventType::EventPublished,
        event_id: "test-001".to_string(),
        topic: "test.topic".to_string(),
        sender: Some("broadcaster_test".to_string()),
        thread_id: None,
        correlation_id: None,
        payload_preview: "test payload".to_string(),
        trace_id: String::new(),
    };

    broadcaster.broadcast(event.clone());

    // All subscribers should receive the event
    let e1 = rx1.try_recv().expect("rx1 should receive event");
    let e2 = rx2.try_recv().expect("rx2 should receive event");
    let e3 = rx3.try_recv().expect("rx3 should receive event");

    assert_eq!(e1.event_id, "test-001");
    assert_eq!(e2.event_id, "test-001");
    assert_eq!(e3.event_id, "test-001");
}

#[tokio::test]
async fn broadcaster_handles_no_subscribers() {
    let broadcaster = EventBroadcaster::new(8);

    let event = DashboardEvent {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type: DashboardEventType::AgentRegistered,
        event_id: "no-sub-001".to_string(),
        topic: "test".to_string(),
        sender: None,
        thread_id: None,
        correlation_id: None,
        payload_preview: String::new(),
        trace_id: String::new(),
    };

    // Should not panic with no subscribers
    broadcaster.broadcast(event);
}

#[tokio::test]
async fn broadcaster_subscriber_drop_reduces_count() {
    let broadcaster = EventBroadcaster::new(8);

    let rx1 = broadcaster.subscribe();
    let rx2 = broadcaster.subscribe();
    assert_eq!(broadcaster.subscriber_count(), 2);

    drop(rx1);
    // Give tokio time to process the drop
    sleep(Duration::from_millis(10)).await;
    assert_eq!(broadcaster.subscriber_count(), 1);

    drop(rx2);
    sleep(Duration::from_millis(10)).await;
    assert_eq!(broadcaster.subscriber_count(), 0);
}

#[tokio::test]
async fn broadcaster_supports_multiple_event_types() {
    let broadcaster = EventBroadcaster::new(16);
    let mut rx = broadcaster.subscribe();

    let events = vec![
        DashboardEventType::EventPublished,
        DashboardEventType::EventDelivered,
        DashboardEventType::AgentRegistered,
        DashboardEventType::AgentUnregistered,
        DashboardEventType::ToolInvoked,
        DashboardEventType::RoutingDecision,
    ];

    for (i, event_type) in events.iter().enumerate() {
        let event = DashboardEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: event_type.clone(),
            event_id: format!("type-test-{}", i),
            topic: "type.test".to_string(),
            sender: Some("type_tester".to_string()),
            thread_id: None,
            correlation_id: None,
            payload_preview: format!("payload {}", i),
            trace_id: String::new(),
        };
        broadcaster.broadcast(event);
    }

    // Verify all event types received
    for i in 0..events.len() {
        let received = rx.try_recv().expect("should receive event");
        assert_eq!(received.event_id, format!("type-test-{}", i));
    }
}

// =============================================================================
// FlowTracker Tests
// =============================================================================

#[tokio::test]
async fn flow_tracker_initializes_with_eventbus() {
    let tracker = FlowTracker::new();
    let graph = tracker.get_graph().await;

    // Should have EventBus node by default
    assert!(
        graph.nodes.iter().any(|n| n.id == "EventBus"),
        "FlowTracker should initialize with EventBus node"
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| matches!(n.node_type, NodeType::EventBus)),
        "EventBus should have correct type"
    );
}

#[tokio::test]
async fn flow_tracker_records_single_flow() {
    let tracker = FlowTracker::new();

    tracker
        .record_flow("agent_a", "agent_b", "test.topic")
        .await;

    let graph = tracker.get_graph().await;

    // Should have at least 3 nodes: EventBus, agent_a, agent_b
    assert!(graph.nodes.len() >= 3);

    // Should have the flow recorded
    let flow = graph
        .flows
        .iter()
        .find(|f| f.source == "agent_a" && f.target == "agent_b")
        .expect("Flow should be recorded");

    assert_eq!(flow.topic, "test.topic");
    assert_eq!(flow.count, 1);
}

#[tokio::test]
async fn flow_tracker_increments_flow_count() {
    let tracker = FlowTracker::new();

    // Record same flow multiple times
    tracker.record_flow("sender", "receiver", "msg.topic").await;
    tracker.record_flow("sender", "receiver", "msg.topic").await;
    tracker.record_flow("sender", "receiver", "msg.topic").await;

    let graph = tracker.get_graph().await;

    let flow = graph
        .flows
        .iter()
        .find(|f| f.source == "sender" && f.target == "receiver")
        .expect("Flow should exist");

    assert_eq!(flow.count, 3, "Flow count should increment");
}

#[tokio::test]
async fn flow_tracker_records_multiple_topics() {
    let tracker = FlowTracker::new();

    tracker.record_flow("a", "b", "topic.one").await;
    tracker.record_flow("a", "b", "topic.two").await;
    tracker.record_flow("a", "b", "topic.three").await;

    let graph = tracker.get_graph().await;

    // Should have separate flows for different topics
    let flows_a_to_b: Vec<_> = graph
        .flows
        .iter()
        .filter(|f| f.source == "a" && f.target == "b")
        .collect();

    assert_eq!(
        flows_a_to_b.len(),
        3,
        "Should have 3 separate flows for different topics"
    );
}

#[tokio::test]
async fn flow_tracker_updates_node_topics() {
    let tracker = FlowTracker::new();

    tracker
        .record_flow("agent_x", "agent_y", "topic.alpha")
        .await;
    tracker
        .record_flow("agent_x", "agent_y", "topic.beta")
        .await;

    let graph = tracker.get_graph().await;

    let node_x = graph
        .nodes
        .iter()
        .find(|n| n.id == "agent_x")
        .expect("agent_x should exist");

    assert!(node_x.topics.contains(&"topic.alpha".to_string()));
    assert!(node_x.topics.contains(&"topic.beta".to_string()));
    assert_eq!(node_x.event_count, 2);
}

#[tokio::test]
async fn flow_tracker_limits_topics_per_node() {
    let tracker = FlowTracker::new();

    // Record more than MAX_TOPICS_PER_NODE (20) topics
    for i in 0..25 {
        tracker
            .record_flow("agent_many", "target", &format!("topic.{}", i))
            .await;
    }

    let graph = tracker.get_graph().await;
    let node = graph
        .nodes
        .iter()
        .find(|n| n.id == "agent_many")
        .expect("node should exist");

    // Should be limited to 20 topics
    assert_eq!(
        node.topics.len(),
        20,
        "Topics should be limited to 20 per node"
    );

    // Should have the most recent 20 topics (5-24, dropping 0-4)
    assert!(node.topics.contains(&"topic.24".to_string()));
    assert!(!node.topics.contains(&"topic.0".to_string()));
}

#[tokio::test]
async fn flow_tracker_infers_node_types() {
    let tracker = FlowTracker::new();

    tracker.record_flow("EventBus", "agent1", "test").await;
    tracker.record_flow("Router", "agent2", "test").await;
    tracker.record_flow("llm_client", "agent3", "test").await;
    tracker.record_flow("tool_provider", "agent4", "test").await;
    tracker.record_flow("storage_layer", "agent5", "test").await;
    tracker.record_flow("regular_agent", "agent6", "test").await;

    let graph = tracker.get_graph().await;

    // Verify node types
    assert!(graph
        .nodes
        .iter()
        .any(|n| n.id == "EventBus" && matches!(n.node_type, NodeType::EventBus)));
    assert!(graph
        .nodes
        .iter()
        .any(|n| n.id == "Router" && matches!(n.node_type, NodeType::Router)));
    assert!(graph
        .nodes
        .iter()
        .any(|n| n.id == "llm_client" && matches!(n.node_type, NodeType::LLM)));
    assert!(graph
        .nodes
        .iter()
        .any(|n| n.id == "tool_provider" && matches!(n.node_type, NodeType::Tool)));
    assert!(graph
        .nodes
        .iter()
        .any(|n| n.id == "storage_layer" && matches!(n.node_type, NodeType::Storage)));
    assert!(graph
        .nodes
        .iter()
        .any(|n| n.id == "regular_agent" && matches!(n.node_type, NodeType::Agent)));
}

#[tokio::test]
async fn flow_tracker_cleans_up_old_flows() {
    let tracker = FlowTracker::new();

    // Record a flow
    tracker
        .record_flow("old_agent", "target", "old.topic")
        .await;

    let graph = tracker.get_graph().await;
    assert_eq!(graph.flows.len(), 1, "Should have one flow");

    // Cleanup should not affect recent flows
    tracker.cleanup().await;
    let graph = tracker.get_graph().await;
    assert_eq!(graph.flows.len(), 1, "Recent flow should remain");

    // Note: Testing actual expiry would require waiting 60+ seconds or mocking time
    // For now, we test that cleanup runs without panicking
}

#[tokio::test]
async fn flow_tracker_graph_includes_timestamp() {
    let tracker = FlowTracker::new();
    tracker.record_flow("a", "b", "test").await;

    let graph = tracker.get_graph().await;

    // Should have a valid RFC3339 timestamp
    assert!(!graph.timestamp.is_empty());
    assert!(chrono::DateTime::parse_from_rfc3339(&graph.timestamp).is_ok());
}

// =============================================================================
// TopologyBuilder Tests
// =============================================================================

#[tokio::test]
async fn topology_builder_empty_directory() {
    let directory = Arc::new(AgentDirectory::new());
    let builder = loom_core::dashboard::TopologyBuilder::new(directory);

    let snapshot = builder.build_snapshot().await;

    assert_eq!(snapshot.agents.len(), 0);
    assert_eq!(snapshot.edges.len(), 0);
    assert!(!snapshot.timestamp.is_empty());
}

#[tokio::test]
async fn topology_builder_single_agent() {
    let directory = Arc::new(AgentDirectory::new());
    let builder = loom_core::dashboard::TopologyBuilder::new(directory.clone());

    let agent_info = loom_core::agent::directory::AgentInfo {
        agent_id: "test_agent".to_string(),
        subscribed_topics: vec!["topic.one".to_string()],
        capabilities: vec!["cap.one".to_string()],
        metadata: std::collections::HashMap::new(),
        last_heartbeat: None,
        status: loom_core::agent::directory::AgentStatus::Active,
    };

    directory.register_agent(agent_info);

    let snapshot = builder.build_snapshot().await;

    assert_eq!(snapshot.agents.len(), 1);
    assert_eq!(snapshot.agents[0].id, "test_agent");
    assert_eq!(snapshot.agents[0].topics, vec!["topic.one"]);
    assert_eq!(snapshot.agents[0].capabilities, vec!["cap.one"]);
}

#[tokio::test]
async fn topology_builder_multiple_agents_creates_edges() {
    let directory = Arc::new(AgentDirectory::new());
    let builder = loom_core::dashboard::TopologyBuilder::new(directory.clone());

    // Agent A subscribes to topic.shared
    directory.register_agent(loom_core::agent::directory::AgentInfo {
        agent_id: "agent_a".to_string(),
        subscribed_topics: vec!["topic.shared".to_string()],
        capabilities: vec![],
        metadata: std::collections::HashMap::new(),
        last_heartbeat: None,
        status: loom_core::agent::directory::AgentStatus::Active,
    });

    // Agent B subscribes to topic.shared
    directory.register_agent(loom_core::agent::directory::AgentInfo {
        agent_id: "agent_b".to_string(),
        subscribed_topics: vec!["topic.shared".to_string()],
        capabilities: vec![],
        metadata: std::collections::HashMap::new(),
        last_heartbeat: None,
        status: loom_core::agent::directory::AgentStatus::Active,
    });

    let snapshot = builder.build_snapshot().await;

    assert_eq!(snapshot.agents.len(), 2);
    assert_eq!(snapshot.edges.len(), 2); // topic.shared -> agent_a, topic.shared -> agent_b

    let edge_targets: Vec<_> = snapshot.edges.iter().map(|e| e.to_agent.as_str()).collect();
    assert!(edge_targets.contains(&"agent_a"));
    assert!(edge_targets.contains(&"agent_b"));
}

#[tokio::test]
async fn topology_builder_handles_multiple_topics() {
    let directory = Arc::new(AgentDirectory::new());
    let builder = loom_core::dashboard::TopologyBuilder::new(directory.clone());

    directory.register_agent(loom_core::agent::directory::AgentInfo {
        agent_id: "multi_topic_agent".to_string(),
        subscribed_topics: vec![
            "topic.one".to_string(),
            "topic.two".to_string(),
            "topic.three".to_string(),
        ],
        capabilities: vec!["cap.a".to_string(), "cap.b".to_string()],
        metadata: std::collections::HashMap::new(),
        last_heartbeat: None,
        status: loom_core::agent::directory::AgentStatus::Active,
    });

    let snapshot = builder.build_snapshot().await;

    assert_eq!(snapshot.agents.len(), 1);
    assert_eq!(snapshot.agents[0].topics.len(), 3);
    assert_eq!(snapshot.agents[0].capabilities.len(), 2);

    // Should have 3 edges (one per topic)
    assert_eq!(snapshot.edges.len(), 3);
}

#[tokio::test]
async fn topology_builder_snapshot_has_timestamp() {
    let directory = Arc::new(AgentDirectory::new());
    let builder = loom_core::dashboard::TopologyBuilder::new(directory);

    let snapshot = builder.build_snapshot().await;

    assert!(!snapshot.timestamp.is_empty());
    assert!(chrono::DateTime::parse_from_rfc3339(&snapshot.timestamp).is_ok());
}

// =============================================================================
// DashboardConfig Tests
// =============================================================================

#[test]
fn dashboard_config_default_values() {
    let config = DashboardConfig::default();

    assert_eq!(config.port, 3030);
    assert_eq!(config.host, "127.0.0.1");
}

#[test]
fn dashboard_config_from_env_uses_defaults() {
    // Clear env vars if set
    std::env::remove_var("LOOM_DASHBOARD_PORT");
    std::env::remove_var("LOOM_DASHBOARD_HOST");

    let config = DashboardConfig::from_env();

    assert_eq!(config.port, 3030);
    assert_eq!(config.host, "127.0.0.1");
}

#[test]
fn dashboard_config_from_env_custom_port() {
    std::env::set_var("LOOM_DASHBOARD_PORT", "8080");

    let config = DashboardConfig::from_env();

    assert_eq!(config.port, 8080);

    std::env::remove_var("LOOM_DASHBOARD_PORT");
}

#[test]
fn dashboard_config_from_env_custom_host() {
    std::env::set_var("LOOM_DASHBOARD_HOST", "0.0.0.0");

    let config = DashboardConfig::from_env();

    assert_eq!(config.host, "0.0.0.0");

    std::env::remove_var("LOOM_DASHBOARD_HOST");
}

#[test]
fn dashboard_config_enabled_false_by_default() {
    std::env::remove_var("LOOM_DASHBOARD");

    assert!(!DashboardConfig::enabled());
}

#[test]
fn dashboard_config_enabled_true_when_set() {
    std::env::set_var("LOOM_DASHBOARD", "true");

    assert!(DashboardConfig::enabled());

    std::env::remove_var("LOOM_DASHBOARD");
}

#[test]
fn dashboard_config_enabled_handles_boolean_string() {
    std::env::set_var("LOOM_DASHBOARD", "1");
    assert!(!DashboardConfig::enabled()); // "1" doesn't parse as bool

    std::env::set_var("LOOM_DASHBOARD", "false");
    assert!(!DashboardConfig::enabled());

    std::env::set_var("LOOM_DASHBOARD", "true");
    assert!(DashboardConfig::enabled());

    std::env::remove_var("LOOM_DASHBOARD");
}
