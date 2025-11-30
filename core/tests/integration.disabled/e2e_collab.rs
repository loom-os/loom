use async_trait::async_trait;
use loom_core::agent::AgentBehavior;
use loom_core::envelope::ThreadTopicKind;
use loom_core::proto::{Action, AgentConfig, AgentState, Event};
use loom_core::Result;
use loom_core::{
    collab_types, ActionBroker, AgentRuntime, Collaborator, Envelope, EventBus, ModelRouter,
};
use std::sync::Arc;

struct CollabResponder {
    id: String,
    bus: Arc<EventBus>,
}

#[async_trait]
impl AgentBehavior for CollabResponder {
    async fn on_event(&mut self, event: Event, _state: &mut AgentState) -> Result<Vec<Action>> {
        let mut env = Envelope::from_event(&event);
        // reply to request
        if event.r#type == collab_types::REQ {
            env.sender = format!("agent.{}", self.id);
            let mut reply = Event {
                id: format!("evt_reply_{}", chrono::Utc::now().timestamp_millis()),
                r#type: collab_types::REPLY.into(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                source: format!("agent.{}", self.id),
                metadata: Default::default(),
                payload: event.payload.clone(),
                confidence: 1.0,
                tags: vec!["collab".into()],
                priority: 50,
            };
            // propagate hop/ttl and attach
            let _alive = env.next_hop();
            env.attach_to_event(&mut reply);
            let _ = self.bus.publish(&env.reply_topic(), reply).await?;
        } else if event.r#type == collab_types::CFP {
            // respond with a proposal that carries a score
            env.sender = format!("agent.{}", self.id);
            let mut proposal = Event {
                id: format!("evt_prop_{}", chrono::Utc::now().timestamp_millis()),
                r#type: collab_types::PROPOSAL.into(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                source: format!("agent.{}", self.id),
                metadata: Default::default(),
                payload: Vec::new(),
                confidence: 1.0,
                tags: vec!["collab".into()],
                priority: 50,
            };
            // embed score in metadata for ranking
            proposal.metadata.insert(
                "score".into(),
                if self.id == "b" { "0.9" } else { "0.7" }.into(),
            );
            let _alive = env.next_hop();
            env.attach_to_event(&mut proposal);
            let _ = self.bus.publish(&env.reply_topic(), proposal).await?;
        }
        Ok(vec![])
    }

    async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> {
        Ok(())
    }
    async fn on_shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn collab_request_reply_through_agents() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;
    let broker = Arc::new(ActionBroker::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(bus.clone(), broker.clone(), router).await?;

    // thread topic
    let thread_broadcast = ThreadTopicKind::Broadcast.topic("t_reqrep");

    // two responders on broadcast
    for id in ["a", "b"] {
        let cfg = AgentConfig {
            agent_id: format!("agent_{}", id),
            agent_type: "responder".into(),
            subscribed_topics: vec![thread_broadcast.clone()],
            capabilities: vec![],
            parameters: Default::default(),
        };
        runtime
            .create_agent(
                cfg,
                Box::new(CollabResponder {
                    id: id.into(),
                    bus: bus.clone(),
                }),
            )
            .await?;
    }

    let collab = Collaborator::new(bus.clone(), "test.client");
    // send request and expect a reply
    let res = collab
        .request_reply(&thread_broadcast, b"ping".to_vec(), 1000)
        .await?;
    assert!(res.is_some(), "should receive a reply");
    let ev = res.unwrap();
    assert_eq!(ev.r#type, collab_types::REPLY);
    Ok(())
}

#[tokio::test]
async fn collab_contract_net_selects_top_score() -> Result<()> {
    let bus = Arc::new(EventBus::new().await?);
    bus.start().await?;
    let broker = Arc::new(ActionBroker::new());
    let router = ModelRouter::new().await?;
    let runtime = AgentRuntime::new(bus.clone(), broker.clone(), router).await?;

    let thread_id = "t_cnp";
    let broadcast_topic = ThreadTopicKind::Broadcast.topic(thread_id);

    for id in ["a", "b"] {
        let cfg = AgentConfig {
            agent_id: format!("agent_{}", id),
            agent_type: "responder".into(),
            subscribed_topics: vec![broadcast_topic.clone()],
            capabilities: vec![],
            parameters: Default::default(),
        };
        runtime
            .create_agent(
                cfg,
                Box::new(CollabResponder {
                    id: id.into(),
                    bus: bus.clone(),
                }),
            )
            .await?;
    }

    let collab = Collaborator::new(bus.clone(), "test.client");
    let winners = collab
        .contract_net(thread_id, b"cfp".to_vec(), 800, 1)
        .await?;
    assert_eq!(winners.len(), 1);
    // Winner should be agent.b (score 0.9) based on our behavior
    let sender = winners[0]
        .metadata
        .get("sender")
        .cloned()
        .unwrap_or_default();
    assert!(sender.contains("agent.b"), "highest score should win");
    Ok(())
}
