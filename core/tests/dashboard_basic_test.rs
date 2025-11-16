use loom_core::dashboard::EventBroadcaster;
use loom_core::dashboard::FlowTracker;

#[tokio::test]
async fn flow_tracker_retains_recent_flows() {
    let tracker = FlowTracker::new();
    // Record a few flows
    tracker.record_flow("agentA", "agentB", "topic.alpha").await;
    tracker.record_flow("agentA", "agentB", "topic.alpha").await;
    tracker.record_flow("agentB", "agentC", "topic.beta").await;

    let graph = tracker.get_graph().await;
    // Should have at least EventBus plus two agents in nodes
    assert!(
        graph.nodes.len() >= 3,
        "expected at least 3 nodes, got {}",
        graph.nodes.len()
    );
    // Should have some flows visible
    assert!(!graph.flows.is_empty(), "expected some flows, got none");
}

#[tokio::test]
async fn event_broadcaster_subscriber_count_and_delivery() {
    let broadcaster = EventBroadcaster::new(8);
    // subscribe two receivers
    let mut r1 = broadcaster.subscribe();
    let mut r2 = broadcaster.subscribe();
    assert_eq!(broadcaster.subscriber_count(), 2);

    // broadcast an event
    broadcaster.broadcast(loom_core::dashboard::DashboardEvent {
        timestamp: chrono::Utc::now().to_rfc3339(),
        event_type: loom_core::dashboard::DashboardEventType::EventPublished,
        event_id: "e1".into(),
        topic: "test.topic".into(),
        sender: Some("tester".into()),
        thread_id: None,
        correlation_id: None,
        payload_preview: "hello".into(),
        trace_id: String::new(),
    });

    // ensure both receivers can receive
    let e1 = r1.try_recv().expect("receiver 1 should get event");
    let e2 = r2.try_recv().expect("receiver 2 should get event");
    assert_eq!(e1.event_id, "e1");
    assert_eq!(e2.event_id, "e1");
}
