// Agent Runtime Implementation
use crate::{proto, Event, EventBus, LoomError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub use crate::proto::{Action, AgentConfig, AgentState};

/// Agent behavior trait
#[async_trait]
pub trait AgentBehavior: Send + Sync {
    async fn on_event(&mut self, event: Event, state: &mut AgentState) -> Result<Vec<Action>>;
    async fn on_init(&mut self, config: &AgentConfig) -> Result<()>;
    async fn on_shutdown(&mut self) -> Result<()>;
}

/// Agent instance
pub struct Agent {
    config: AgentConfig,
    state: Arc<RwLock<AgentState>>,
    behavior: Box<dyn AgentBehavior>,
    event_rx: tokio::sync::mpsc::Receiver<Event>,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        behavior: Box<dyn AgentBehavior>,
        event_rx: tokio::sync::mpsc::Receiver<Event>,
    ) -> Self {
        let state = AgentState {
            agent_id: config.agent_id.clone(),
            persistent_state: vec![],
            ephemeral_context: vec![],
            last_update_ms: chrono::Utc::now().timestamp_millis(),
            metadata: config.parameters.clone(),
        };

        Self {
            config,
            state: Arc::new(RwLock::new(state)),
            behavior,
            event_rx,
        }
    }

    /// Start agent event loop
    pub async fn run(mut self) -> Result<()> {
        info!("Agent {} starting", self.config.agent_id);

        // Initialize
        self.behavior.on_init(&self.config).await?;

        // Event loop
        while let Some(event) = self.event_rx.recv().await {
            debug!("Agent {} received event {}", self.config.agent_id, event.id);

            let mut state = self.state.write().await;

            match self.behavior.on_event(event, &mut state).await {
                Ok(actions) => {
                    // Execute actions
                    for action in actions {
                        self.execute_action(action).await?;
                    }
                }
                Err(e) => {
                    warn!("Agent {} error handling event: {}", self.config.agent_id, e);
                }
            }

            // Update timestamp
            state.last_update_ms = chrono::Utc::now().timestamp_millis();
        }

        // Cleanup
        self.behavior.on_shutdown().await?;
        info!("Agent {} stopped", self.config.agent_id);

        Ok(())
    }

    async fn execute_action(&self, action: Action) -> Result<()> {
        debug!("Executing action: {}", action.action_type);
        // TODO: Implement action execution logic
        Ok(())
    }
}

/// Agent runtime manager
pub struct AgentRuntime {
    agents: Arc<DashMap<String, tokio::task::JoinHandle<()>>>,
    event_bus: Arc<EventBus>,
}
impl AgentRuntime {
    pub async fn new(event_bus: Arc<EventBus>) -> Result<Self> {
        Ok(Self {
            agents: Arc::new(DashMap::new()),
            event_bus,
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
        let agent = Agent::new(config, behavior, event_rx);
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
