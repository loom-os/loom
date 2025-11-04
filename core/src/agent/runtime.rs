use std::sync::Arc;

use dashmap::DashMap;
use tracing::{info, warn};

use crate::{proto, Result, EventBus, LoomError};
use crate::action_broker::ActionBroker;
use crate::router::ModelRouter;
use crate::proto::AgentConfig;

use super::behavior::AgentBehavior;
use super::instance::Agent;

/// Agent runtime manager
pub struct AgentRuntime {
    agents: Arc<DashMap<String, tokio::task::JoinHandle<()>>>,
    event_bus: Arc<EventBus>,
    action_broker: Arc<ActionBroker>,
    model_router: ModelRouter,
}

impl AgentRuntime {
    pub async fn new(
        event_bus: Arc<EventBus>,
        action_broker: Arc<ActionBroker>,
        model_router: ModelRouter,
    ) -> Result<Self> {
        Ok(Self {
            agents: Arc::new(DashMap::new()),
            event_bus,
            action_broker,
            model_router,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Agent Runtime started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Agent Runtime shutting down");

        // Stop all agents
        for entry in self.agents.iter() {
            entry.value().abort();
        }
        self.agents.clear();

        Ok(())
    }

    /// Create and start an Agent
    pub async fn create_agent(
        &self,
        config: AgentConfig,
        behavior: Box<dyn AgentBehavior>,
    ) -> Result<String> {
        let agent_id = config.agent_id.clone();

        // Create event receiving channel for agent
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(1000);

        // Subscribe to events
        for topic in &config.subscribed_topics {
            let (_sub_id, mut rx) = self
                .event_bus
                .subscribe(topic.clone(), vec![], proto::QoSLevel::QosBatched)
                .await?;

            // Forward events to agent
            let tx = event_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = tx.send(event).await;
                }
            });
        }

        // Create and start agent
        let agent = Agent::new(
            config,
            behavior,
            event_rx,
            Arc::clone(&self.action_broker),
            Arc::clone(&self.event_bus),
            self.model_router.clone(),
        );
        let handle = tokio::spawn(async move {
            if let Err(e) = agent.run().await {
                warn!("Agent error: {}", e);
            }
        });

        self.agents.insert(agent_id.clone(), handle);
        info!("Created agent {}", agent_id);

        Ok(agent_id)
    }

    /// Delete an Agent
    pub async fn delete_agent(&self, agent_id: &str) -> Result<()> {
        if let Some((_, handle)) = self.agents.remove(agent_id) {
            handle.abort();
            info!("Deleted agent {}", agent_id);
            Ok(())
        } else {
            Err(LoomError::AgentError(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }
}
