//! Adapter that wraps a CognitiveLoop as an AgentBehavior.

use async_trait::async_trait;

use crate::agent::AgentBehavior;
use crate::proto::{Action, AgentConfig, AgentState, Event};
use crate::Result;

use super::loop_trait::CognitiveLoop;

/// Adapter that lets a [`CognitiveLoop`] be used as an [`AgentBehavior`].
///
/// This adapter bridges the cognitive architecture with the existing
/// AgentRuntime system, allowing cognitive agents to be managed
/// like any other agent.
///
/// # Example
///
/// ```rust,ignore
/// use loom_core::agent::cognitive::{CognitiveAgent, SimpleCognitiveLoop, CognitiveConfig};
///
/// // Create a cognitive loop
/// let loop_impl = SimpleCognitiveLoop::new(config, llm_client, action_broker);
///
/// // Wrap it as an AgentBehavior
/// let behavior = CognitiveAgent::new(loop_impl);
///
/// // Use with AgentRuntime
/// let agent_id = runtime.create_agent(agent_config, Box::new(behavior)).await?;
/// ```
pub struct CognitiveAgent<L: CognitiveLoop> {
    /// The underlying cognitive loop implementation
    loop_impl: L,

    /// Whether initialization has completed
    initialized: bool,
}

impl<L: CognitiveLoop> CognitiveAgent<L> {
    /// Create a new cognitive agent adapter
    pub fn new(loop_impl: L) -> Self {
        Self {
            loop_impl,
            initialized: false,
        }
    }

    /// Get a reference to the underlying cognitive loop
    pub fn inner(&self) -> &L {
        &self.loop_impl
    }

    /// Get a mutable reference to the underlying cognitive loop
    pub fn inner_mut(&mut self) -> &mut L {
        &mut self.loop_impl
    }

    /// Access the working memory
    pub fn working_memory(&self) -> &super::WorkingMemory {
        self.loop_impl.working_memory()
    }
}

#[async_trait]
impl<L: CognitiveLoop + 'static> AgentBehavior for CognitiveAgent<L> {
    async fn on_event(&mut self, event: Event, state: &mut AgentState) -> Result<Vec<Action>> {
        // Create a tracing span for the cognitive cycle
        let span = tracing::info_span!(
            "cognitive_cycle",
            event_id = %event.id,
            event_type = %event.r#type,
            agent_id = %state.agent_id,
        );
        let _guard = span.enter();

        tracing::debug!(
            target = "cognitive",
            event_id = %event.id,
            "Starting cognitive cycle"
        );

        // Run the complete cognitive cycle
        let result = self.loop_impl.run_cycle(event, state).await?;

        tracing::debug!(
            target = "cognitive",
            actions = result.actions.len(),
            goal_achieved = result.goal_achieved,
            "Cognitive cycle complete"
        );

        Ok(result.into_actions())
    }

    async fn on_init(&mut self, config: &AgentConfig) -> Result<()> {
        tracing::info!(
            target = "cognitive",
            agent_id = %config.agent_id,
            "Initializing cognitive agent"
        );

        // Set session ID in working memory if available
        if let Some(session) = config.parameters.get("session_id") {
            self.loop_impl
                .working_memory_mut()
                .set_session_id(session.clone());
        } else {
            // Use agent_id as default session
            self.loop_impl
                .working_memory_mut()
                .set_session_id(&config.agent_id);
        }

        self.initialized = true;
        Ok(())
    }

    async fn on_shutdown(&mut self) -> Result<()> {
        tracing::info!(target = "cognitive", "Shutting down cognitive agent");

        // Clear working memory on shutdown
        self.loop_impl.working_memory_mut().clear();

        Ok(())
    }
}
