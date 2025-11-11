use std::sync::Arc;

use loom_core::{collab_types, Collaborator, Envelope, Event, EventBus, QoSLevel};

#[tokio::test]
async fn request_reply_basic() {
    let bus = Arc::new(EventBus::new().await.unwrap());
    bus.start().await.unwrap();

    // Responder subscribes to request topic and replies to reply_to
    let bus_clone = Arc::clone(&bus);
    let (_sid, mut rx) = bus_clone
        .subscribe(
            "service.echo".into(),
            vec![collab_types::REQ.into()],
            QoSLevel::QosBatched,
        )
        .await
        .unwrap();
    tokio::spawn(async move {
        while let Some(mut req) = rx.recv().await {
            // Build reply
            let env = Envelope::from_event(&req);
            let mut md = std::collections::HashMap::new();
            env.apply_to_metadata(&mut md);
            let mut reply = Event {
                id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                r#type: collab_types::REPLY.into(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                source: "agent.echo".into(),
                metadata: md,
                payload: req.payload.clone(),
                confidence: 1.0,
                tags: vec!["test".into()],
                priority: 50,
            };
            env.attach_to_event(&mut reply);
            let _ = bus_clone.publish(&env.reply_topic(), reply).await;
        }
    });

    let collab = Collaborator::new(Arc::clone(&bus), "agent.client");
    let res = collab
        .request_reply("service.echo", b"hello".to_vec(), 1000)
        .await
        .unwrap();
    assert!(res.is_some());
    let evt = res.unwrap();
    assert_eq!(evt.payload, b"hello");
}

#[tokio::test]
async fn fanout_first_k() {
    let bus = Arc::new(EventBus::new().await.unwrap());
    bus.start().await.unwrap();

    // Two responders with different delays
    for i in 0..3u8 {
        let bus_i = Arc::clone(&bus);
        let topic = format!("service.race.{}", i);
        let (_sid, mut rx) = bus_i
            .subscribe(
                topic.clone(),
                vec![collab_types::REQ.into()],
                QoSLevel::QosBatched,
            )
            .await
            .unwrap();
        tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                let delay = (i as u64) * 50;
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                let env = Envelope::from_event(&req);
                let mut md = std::collections::HashMap::new();
                md.insert("score".into(), (100 - delay as i32).to_string());
                env.apply_to_metadata(&mut md);
                let mut reply = Event {
                    id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                    r#type: collab_types::REPLY.into(),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    source: format!("agent.{}", i),
                    metadata: md,
                    payload: vec![i],
                    confidence: 1.0,
                    tags: vec!["test".into()],
                    priority: 50,
                };
                env.attach_to_event(&mut reply);
                let _ = bus_i.publish(&env.reply_topic(), reply).await;
            }
        });
    }

    let collab = Collaborator::new(Arc::clone(&bus), "agent.client");
    let topics = vec![
        "service.race.0".to_string(),
        "service.race.1".to_string(),
        "service.race.2".to_string(),
    ];
    let replies = collab
        .fanout_fanin(&topics, b"go".to_vec(), 2, 1000)
        .await
        .unwrap();
    assert_eq!(replies.len(), 2);
}

#[tokio::test]
async fn contract_net_selects_top_score() {
    let bus = Arc::new(EventBus::new().await.unwrap());
    bus.start().await.unwrap();

    // Participants listen on broadcast thread and post proposals to reply
    let thread_id = "cnp.test";
    let broadcast_topic = format!("thread.{}.broadcast", thread_id);
    for score in [10, 80, 50] {
        let bus_i = Arc::clone(&bus);
        let (_sid, mut rx) = bus_i
            .subscribe(
                broadcast_topic.clone(),
                vec![collab_types::CFP.into()],
                QoSLevel::QosBatched,
            )
            .await
            .unwrap();
        tokio::spawn(async move {
            while let Some(req) = rx.recv().await {
                let env = Envelope::from_event(&req);
                let mut md = std::collections::HashMap::new();
                md.insert("score".into(), score.to_string());
                md.insert("sender".into(), format!("agent.score{}", score));
                env.apply_to_metadata(&mut md);
                let mut proposal = Event {
                    id: format!("evt_{}", chrono::Utc::now().timestamp_millis()),
                    r#type: collab_types::PROPOSAL.into(),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    source: format!("agent.score{}", score),
                    metadata: md,
                    payload: vec![],
                    confidence: 1.0,
                    tags: vec!["test".into()],
                    priority: 50,
                };
                env.attach_to_event(&mut proposal);
                let _ = bus_i.publish(&env.reply_topic(), proposal).await;
            }
        });
    }

    let collab = Collaborator::new(Arc::clone(&bus), "agent.client");
    let winners = collab
        .contract_net(thread_id, b"task".to_vec(), 300, 1)
        .await
        .unwrap();
    assert_eq!(winners.len(), 1);
    let top = &winners[0];
    assert_eq!(top.metadata.get("score").unwrap(), "80");
}
