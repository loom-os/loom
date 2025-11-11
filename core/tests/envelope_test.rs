use loom_core::proto::{ActionCall, Event, QoSLevel};
use loom_core::{Envelope, ThreadTopicKind};
use std::collections::HashMap;

fn dummy_event(id: &str) -> Event {
    Event {
        id: id.to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "tester".to_string(),
        metadata: HashMap::new(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

#[test]
fn envelope_new_sets_defaults() {
    let env = Envelope::new("threadA", "agent.alpha");
    assert_eq!(env.thread_id, "threadA");
    assert_eq!(
        env.correlation_id, "threadA",
        "default correlation equals thread id"
    );
    assert_eq!(env.reply_to, ThreadTopicKind::Reply.topic("threadA"));
    assert_eq!(env.ttl, 16);
    assert_eq!(env.hop, 0);
    assert!(env.timestamp_ms > 0);
}

#[test]
fn envelope_from_metadata_fallbacks() {
    let mut meta = HashMap::new();
    meta.insert("sender".into(), "agent.beta".into());
    // No thread_id provided -> fallback to event id
    let env = Envelope::from_metadata(&meta, "evt-123");
    assert_eq!(env.thread_id, "evt-123");
    assert_eq!(env.correlation_id, "evt-123");
    assert_eq!(env.reply_to, ThreadTopicKind::Reply.topic("evt-123"));
    assert_eq!(env.sender, "agent.beta");
}

#[test]
fn envelope_attach_to_event_roundtrip() {
    let mut evt = dummy_event("e1");
    let env = Envelope::new("threadZ", "agent.gamma");
    env.attach_to_event(&mut evt);
    let env2 = Envelope::from_event(&evt);
    assert_eq!(env, env2);
}

#[test]
fn envelope_apply_to_action_call_sets_headers_and_correlation() {
    let env = Envelope::new("threadQ", "agent.delta");
    let mut call = ActionCall {
        id: "call1".into(),
        capability: "demo.action".into(),
        version: String::new(),
        payload: vec![],
        headers: HashMap::new(),
        correlation_id: "".into(),
        qos: QoSLevel::QosRealtime as i32,
        timeout_ms: 0,
    };
    env.apply_to_action_call(&mut call);
    assert_eq!(call.correlation_id, env.correlation_id);
    assert_eq!(call.headers.get("thread_id"), Some(&env.thread_id));
    assert_eq!(call.headers.get("reply_to"), Some(&env.reply_to));
}

#[test]
fn envelope_next_hop_increments_and_decrements() {
    let mut env = Envelope::new("threadH", "agent.eps");
    env.ttl = 3; // make small for test
    assert!(env.next_hop()); // ttl becomes 2
    assert_eq!(env.hop, 1);
    assert_eq!(env.ttl, 2);
    assert!(env.next_hop()); // ttl becomes 1
    assert_eq!(env.hop, 2);
    assert_eq!(env.ttl, 1);
    assert!(
        !env.next_hop(),
        "third hop should exhaust TTL and return false"
    ); // ttl becomes 0
    assert_eq!(env.hop, 3);
    assert_eq!(env.ttl, 0);
    assert!(!env.next_hop(), "after ttl hits 0 further hops invalid");
}

#[test]
fn thread_topic_helpers() {
    let env = Envelope::new("abc", "agent.omega");
    assert_eq!(env.broadcast_topic(), "thread.abc.broadcast");
    assert_eq!(env.reply_topic(), "thread.abc.reply");
}
