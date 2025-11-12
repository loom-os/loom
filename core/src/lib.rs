// Loom Core Library
// Event-driven AI operating system runtime

pub mod action_broker;
pub mod agent;
pub mod collab; // Collaboration primitives built on EventBus + Envelope
pub mod context;
pub mod dashboard; // Real-time event flow visualization
pub mod directory; // Agent & Capability directories
pub mod envelope; // Unified metadata envelope for events/actions threads
pub mod event;
pub mod llm;
pub mod mcp; // Model Context Protocol client and adapters
pub mod plugin;
pub mod providers;
pub mod router;
pub mod storage;
pub mod telemetry;

// Export core types
pub use action_broker::{ActionBroker, CapabilityProvider};
pub use agent::{Agent, AgentRuntime, AgentState};
pub use collab::{types as collab_types, Collaborator};
pub use context::{builder::ContextBuilder, PromptBundle, TokenBudget};
pub use directory::{AgentDirectory, AgentInfo, CapabilityDirectory};
pub use envelope::{agent_reply_topic, Envelope, ThreadTopicKind};
pub use event::{Event, EventBus, EventExt, EventHandler, QoSLevel};
pub use llm::{LlmClient, LlmClientConfig, LlmResponse};
pub use mcp::{McpClient, McpManager, McpToolAdapter};
pub use plugin::{Plugin, PluginManager};
pub use providers::{WeatherProvider, WebSearchProvider};
pub use router::{
    ConfidenceEstimator, DummyConfidenceEstimator, ModelRouter, Route, RoutingDecision,
};

// Generated proto code
// Re-export proto types from the shared crate so existing paths `crate::proto::...` continue to work.
pub use loom_proto as proto;

// Error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoomError {
    #[error("Event bus error: {0}")]
    EventBusError(String),

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Router error: {0}")]
    RouterError(String),

    #[error("Plugin error: {0}")]
    PluginError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, LoomError>;

/// Core runtime
pub struct Loom {
    pub event_bus: std::sync::Arc<EventBus>,
    pub agent_runtime: AgentRuntime,
    pub model_router: ModelRouter,
    pub plugin_manager: PluginManager,
    pub action_broker: std::sync::Arc<ActionBroker>,
    pub mcp_manager: std::sync::Arc<mcp::McpManager>,
}

impl Loom {
    pub async fn new() -> Result<Self> {
        // Note: OpenTelemetry should be initialized BEFORE creating Loom
        // by calling telemetry::init_telemetry() in your main function.
        // This ensures the global tracing subscriber is set up correctly.

        let event_bus = std::sync::Arc::new(EventBus::new().await?);
        let action_broker = std::sync::Arc::new(ActionBroker::new());
        // Initialize router first so we can pass a clone to the agent runtime
        let model_router = ModelRouter::new().await?;
        // Register built-in capability providers
        {
            use crate::llm::LlmGenerateProvider;
            use std::sync::Arc as SyncArc;
            if let Ok(provider) = LlmGenerateProvider::new(None) {
                action_broker.register_provider(SyncArc::new(provider));
            } else {
                tracing::warn!(
                    target = "loom",
                    "Failed to initialize LLM provider from env; llm.generate not registered"
                );
            }

            // Note: Audio (e.g., TTS) providers have moved to the separate crate `loom-audio`.
            // Core no longer registers audio providers by default to avoid circular dependencies.
        }

        // Initialize MCP manager
        let mcp_manager =
            std::sync::Arc::new(mcp::McpManager::new(std::sync::Arc::clone(&action_broker)));

        Ok(Self {
            agent_runtime: AgentRuntime::new(
                std::sync::Arc::clone(&event_bus),
                std::sync::Arc::clone(&action_broker),
                model_router.clone(),
            )
            .await?,
            model_router,
            plugin_manager: PluginManager::new().await?,
            event_bus,
            action_broker,
            mcp_manager,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting Loom...");

        self.event_bus.start().await?;
        self.agent_runtime.start().await?;
        self.model_router.start().await?;
        self.plugin_manager.start().await?;

        tracing::info!("Loom started successfully");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutting down Loom...");

        self.mcp_manager.shutdown().await;
        self.plugin_manager.shutdown().await?;
        self.model_router.shutdown().await?;
        self.agent_runtime.shutdown().await?;
        self.event_bus.shutdown().await?;

        // Shutdown OpenTelemetry (flushes pending telemetry)
        telemetry::shutdown_telemetry();

        tracing::info!("Loom shut down successfully");
        Ok(())
    }
}
