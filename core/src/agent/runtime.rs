use std::sync::Arc;

use dashmap::DashMap;
use opentelemetry::metrics::{Counter, UpDownCounter};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::action_broker::ActionBroker;
use crate::proto::AgentConfig;
use crate::router::ModelRouter;
use crate::{proto, Event, EventBus, LoomError, Result};

use super::behavior::AgentBehavior;
use super::instance::Agent;

/// Subscription handle for an agent
#[derive(Debug)]
struct AgentSubscription {
    /// Subscription ID from EventBus
    subscription_id: String,
    /// Topic being subscribed to
    #[allow(dead_code)]
    topic: String,
    /// Forwarder task handle
    forwarder_handle: tokio::task::JoinHandle<()>,
}

/// Agent metadata tracked by runtime
struct AgentMetadata {
    /// Agent task handle
    task_handle: tokio::task::JoinHandle<()>,
    /// Event sender channel to agent mailbox
    event_tx: mpsc::Sender<Event>,
    /// Active subscriptions
    subscriptions: Arc<DashMap<String, AgentSubscription>>,
}

/// Agent runtime manager
pub struct AgentRuntime {
    agents: Arc<DashMap<String, AgentMetadata>>,
    event_bus: Arc<EventBus>,
    action_broker: Arc<ActionBroker>,
    model_router: ModelRouter,
    // OpenTelemetry metrics
    agents_active_gauge: UpDownCounter<i64>,
    agents_created_counter: Counter<u64>,
    agents_deleted_counter: Counter<u64>,
    subscriptions_counter: Counter<u64>,
    unsubscriptions_counter: Counter<u64>,
}

impl AgentRuntime {
    pub async fn new(
        event_bus: Arc<EventBus>,
        action_broker: Arc<ActionBroker>,
        model_router: ModelRouter,
    ) -> Result<Self> {
        // Initialize OpenTelemetry metrics
        let meter = opentelemetry::global::meter("loom.agent_runtime");

        let agents_active_gauge = meter
            .i64_up_down_counter("agent_runtime.agents.active")
            .with_description("Number of active agents")
            .init();

        let agents_created_counter = meter
            .u64_counter("agent_runtime.agents.created")
            .with_description("Total number of agents created")
            .init();

        let agents_deleted_counter = meter
            .u64_counter("agent_runtime.agents.deleted")
            .with_description("Total number of agents deleted")
            .init();

        let subscriptions_counter = meter
            .u64_counter("agent_runtime.subscriptions.total")
            .with_description("Total number of topic subscriptions")
            .init();

        let unsubscriptions_counter = meter
            .u64_counter("agent_runtime.unsubscriptions.total")
            .with_description("Total number of topic unsubscriptions")
            .init();

        Ok(Self {
            agents: Arc::new(DashMap::new()),
            event_bus,
            action_broker,
            model_router,
            agents_active_gauge,
            agents_created_counter,
            agents_deleted_counter,
            subscriptions_counter,
            unsubscriptions_counter,
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
            // Abort forwarder tasks
            for sub in entry.value().subscriptions.iter() {
                sub.value().forwarder_handle.abort();
            }
            // Abort agent task
            entry.value().task_handle.abort();
        }
        self.agents.clear();

        Ok(())
    }

    /// Create and start an Agent
    #[tracing::instrument(skip(self, behavior), fields(agent_id = %config.agent_id, topic_count = config.subscribed_topics.len()))]
    pub async fn create_agent(
        &self,
        config: AgentConfig,
        behavior: Box<dyn AgentBehavior>,
    ) -> Result<String> {
        let agent_id = config.agent_id.clone();

        // Create event receiving channel for agent
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(1000);

        // Track subscriptions for this agent
        let subscriptions = Arc::new(DashMap::new());

        // Auto-subscribe to private agent reply topic
        // This enables point-to-point agent communication
        let private_reply_topic = crate::envelope::agent_reply_topic(&agent_id);
        {
            let (sub_id, mut rx) = self
                .event_bus
                .subscribe(
                    private_reply_topic.clone(),
                    vec![],
                    proto::QoSLevel::QosBatched,
                )
                .await?;

            let tx = event_tx.clone();
            let forwarder_handle = tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = tx.send(event).await;
                }
            });

            subscriptions.insert(
                private_reply_topic.clone(),
                AgentSubscription {
                    subscription_id: sub_id,
                    topic: private_reply_topic,
                    forwarder_handle,
                },
            );
        }

        // Subscribe to initial topics from config
        for topic in &config.subscribed_topics {
            let (sub_id, mut rx) = self
                .event_bus
                .subscribe(topic.clone(), vec![], proto::QoSLevel::QosBatched)
                .await?;

            // Forward events to agent
            let tx = event_tx.clone();
            let forwarder_handle = tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let _ = tx.send(event).await;
                }
            });

            subscriptions.insert(
                topic.clone(),
                AgentSubscription {
                    subscription_id: sub_id,
                    topic: topic.clone(),
                    forwarder_handle,
                },
            );
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
        let task_handle = tokio::spawn(async move {
            if let Err(e) = agent.run().await {
                warn!("Agent error: {}", e);
            }
        });

        // Capture subscription count before moving subscriptions
        let sub_count = subscriptions.len();

        let metadata = AgentMetadata {
            task_handle,
            event_tx,
            subscriptions,
        };

        self.agents.insert(agent_id.clone(), metadata);

        // Update metrics
        self.agents_active_gauge.add(1, &[]);
        self.agents_created_counter.add(1, &[]);
        self.subscriptions_counter.add(sub_count as u64, &[]);

        info!(
            "Created agent {} with private reply topic {}",
            agent_id,
            crate::envelope::agent_reply_topic(&agent_id)
        );

        Ok(agent_id)
    }

    /// Delete an Agent
    #[tracing::instrument(skip(self), fields(agent_id = %agent_id))]
    pub async fn delete_agent(&self, agent_id: &str) -> Result<()> {
        if let Some((_, metadata)) = self.agents.remove(agent_id) {
            let sub_count = metadata.subscriptions.len();

            // Unsubscribe from all topics and abort forwarder tasks
            for entry in metadata.subscriptions.iter() {
                let sub = entry.value();
                let _ = self.event_bus.unsubscribe(&sub.subscription_id).await;
                sub.forwarder_handle.abort();
            }

            // Abort agent task
            metadata.task_handle.abort();

            // Update metrics
            self.agents_active_gauge.add(-1, &[]);
            self.agents_deleted_counter.add(1, &[]);
            self.unsubscriptions_counter.add(sub_count as u64, &[]);

            info!("Deleted agent {}", agent_id);
            Ok(())
        } else {
            Err(LoomError::AgentError(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }

    /// Subscribe an agent to a topic at runtime
    ///
    /// Enables dynamic subscription after agent creation, allowing agents to join
    /// new conversations or listen to additional event sources during execution.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The unique identifier of the agent
    /// * `topic` - The topic to subscribe to
    ///
    /// # Errors
    ///
    /// Returns error if agent doesn't exist or is already subscribed to the topic
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use loom_core::{AgentRuntime, EventBus, ActionBroker, ModelRouter};
    /// # use std::sync::Arc;
    /// # async fn example() -> loom_core::Result<()> {
    /// let event_bus = Arc::new(EventBus::new().await?);
    /// let action_broker = Arc::new(ActionBroker::new());
    /// let model_router = ModelRouter::new().await?;
    /// let runtime = AgentRuntime::new(event_bus, action_broker, model_router).await?;
    ///
    /// // Agent joins a thread mid-conversation
    /// runtime.subscribe_agent("agent-1", "thread.task-123.broadcast".to_string()).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, topic = %topic))]
    pub async fn subscribe_agent(&self, agent_id: &str, topic: String) -> Result<()> {
        let metadata = self
            .agents
            .get(agent_id)
            .ok_or_else(|| LoomError::AgentError(format!("Agent {} not found", agent_id)))?;

        // Check if already subscribed
        if metadata.subscriptions.contains_key(&topic) {
            return Err(LoomError::AgentError(format!(
                "Agent {} already subscribed to topic {}",
                agent_id, topic
            )));
        }

        // Subscribe to event bus
        let (sub_id, mut rx) = self
            .event_bus
            .subscribe(topic.clone(), vec![], proto::QoSLevel::QosBatched)
            .await?;

        // Forward events to agent mailbox
        let tx = metadata.event_tx.clone();
        let forwarder_handle = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let _ = tx.send(event).await;
            }
        });

        // Track subscription
        metadata.subscriptions.insert(
            topic.clone(),
            AgentSubscription {
                subscription_id: sub_id,
                topic: topic.clone(),
                forwarder_handle,
            },
        );

        // Update metrics
        self.subscriptions_counter.add(1, &[]);

        info!("Agent {} subscribed to topic {}", agent_id, topic);
        Ok(())
    }

    /// Unsubscribe an agent from a topic at runtime
    ///
    /// Removes a subscription added either at creation or via `subscribe_agent`.
    /// The agent will no longer receive events on this topic.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The unique identifier of the agent
    /// * `topic` - The topic to unsubscribe from
    ///
    /// # Errors
    ///
    /// Returns error if agent doesn't exist or is not subscribed to the topic
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use loom_core::{AgentRuntime, EventBus, ActionBroker, ModelRouter};
    /// # use std::sync::Arc;
    /// # async fn example() -> loom_core::Result<()> {
    /// # let event_bus = Arc::new(EventBus::new().await?);
    /// # let action_broker = Arc::new(ActionBroker::new());
    /// # let model_router = ModelRouter::new().await?;
    /// let runtime = AgentRuntime::new(event_bus, action_broker, model_router).await?;
    ///
    /// // Agent leaves a thread
    /// runtime.unsubscribe_agent("agent-1", "thread.task-123.broadcast").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, topic = %topic))]
    pub async fn unsubscribe_agent(&self, agent_id: &str, topic: &str) -> Result<()> {
        let metadata = self
            .agents
            .get(agent_id)
            .ok_or_else(|| LoomError::AgentError(format!("Agent {} not found", agent_id)))?;

        // Remove subscription
        let sub = metadata
            .subscriptions
            .remove(topic)
            .ok_or_else(|| {
                LoomError::AgentError(format!(
                    "Agent {} not subscribed to topic {}",
                    agent_id, topic
                ))
            })?
            .1;

        // Unsubscribe from event bus
        self.event_bus.unsubscribe(&sub.subscription_id).await?;

        // Abort forwarder task
        sub.forwarder_handle.abort();

        // Update metrics
        self.unsubscriptions_counter.add(1, &[]);

        info!("Agent {} unsubscribed from topic {}", agent_id, topic);
        Ok(())
    }

    /// Get list of topics an agent is currently subscribed to
    ///
    /// Returns all active subscriptions for diagnostic and coordination purposes.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - The unique identifier of the agent
    ///
    /// # Errors
    ///
    /// Returns error if agent doesn't exist
    pub fn get_agent_subscriptions(&self, agent_id: &str) -> Result<Vec<String>> {
        let metadata = self
            .agents
            .get(agent_id)
            .ok_or_else(|| LoomError::AgentError(format!("Agent {} not found", agent_id)))?;

        let topics = metadata
            .subscriptions
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        Ok(topics)
    }
}
