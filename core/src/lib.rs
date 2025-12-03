// Loom Core Library
// Event-driven AI operating system runtime

pub mod agent;
pub mod cognitive; // LLM + Cognitive Loop (perceive-think-act)
pub mod context; // Context Engineering system
pub mod dashboard; // Real-time event flow visualization
pub mod messaging; // Event Bus, Envelope, Collab
pub mod telemetry;
pub mod tools; // Unified tool system (Native + MCP)

// Export agent types
pub use agent::directory::{AgentDirectory, AgentInfo, AgentStatus, CapabilityDirectory};
pub use agent::{Agent, AgentBehavior, AgentRuntime};

// Export agent state from proto
pub use proto::{AgentConfig, AgentState};

// Export cognitive types
pub use cognitive::llm::router::{
    ConfidenceEstimator, DummyConfidenceEstimator, ModelRouter, Route, RoutingDecision,
};
pub use cognitive::llm::{LlmClient, LlmClientConfig, LlmResponse};
pub use cognitive::{
    CognitiveAgent, CognitiveConfig, CognitiveLoop, MemoryBuffer, SimpleCognitiveLoop,
    ThinkingStrategy,
};

// Export context types
pub use context::{builder::ContextBuilder, PromptBundle, TokenBudget};
pub use context::{AgentContext, ContextPipeline, InMemoryStore, MemoryStore, RocksDbStore};

// Export messaging types
pub use messaging::collab::{types as collab_types, Collaborator};
pub use messaging::{
    agent_reply_topic, Envelope, EventBus, EventBusStats, EventExt, EventHandler, ThreadTopicKind,
};

// Export tool types
pub use tools::mcp::{McpClient, McpManager, McpToolAdapter};
pub use tools::native::{ReadFileTool, ShellTool, WeatherTool, WebSearchTool};
pub use tools::{Tool, ToolError, ToolRegistry};

// Export telemetry
pub use telemetry::{init_telemetry, shutdown_telemetry, SpanCollector, SpanData};

// Re-export proto types (Event, QoSLevel, etc.)
pub use proto::{Event, QoSLevel};

// Generated proto code
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
    pub tool_registry: std::sync::Arc<ToolRegistry>,
    pub mcp_manager: std::sync::Arc<tools::mcp::McpManager>,
    pub agent_directory: std::sync::Arc<AgentDirectory>,
}

impl Loom {
    pub async fn new() -> Result<Self> {
        let event_bus = std::sync::Arc::new(EventBus::new().await?);
        let tool_registry = std::sync::Arc::new(ToolRegistry::new());
        let agent_directory = std::sync::Arc::new(AgentDirectory::new());
        let model_router = ModelRouter::new().await?;

        // Register built-in tools
        {
            use crate::cognitive::llm::LlmGenerateProvider;
            use crate::tools::native::{ReadFileTool, ShellTool, WeatherTool, WebSearchTool};
            use std::sync::Arc as SyncArc;

            if let Ok(provider) = LlmGenerateProvider::new(None) {
                tool_registry.register(SyncArc::new(provider)).await;
            } else {
                tracing::warn!(
                    target = "loom",
                    "Failed to initialize LLM provider from env; llm:generate not registered"
                );
            }

            let workspace_root = std::env::current_dir().unwrap_or_default();
            tool_registry
                .register(SyncArc::new(ReadFileTool::new(workspace_root)))
                .await;

            tool_registry
                .register(SyncArc::new(ShellTool::new(vec![
                    "ls".to_string(),
                    "echo".to_string(),
                    "cat".to_string(),
                    "grep".to_string(),
                ])))
                .await;

            tool_registry
                .register(SyncArc::new(WeatherTool::new()))
                .await;
            tool_registry
                .register(SyncArc::new(WebSearchTool::new()))
                .await;
        }

        let mcp_manager = std::sync::Arc::new(tools::mcp::McpManager::new(std::sync::Arc::clone(
            &tool_registry,
        )));

        // Auto-load MCP servers from environment variable
        if let Err(e) = mcp_manager.load_from_env().await {
            tracing::warn!(
                target = "loom",
                error = %e,
                "Failed to load MCP servers from LOOM_MCP_SERVERS"
            );
        }

        Ok(Self {
            agent_runtime: AgentRuntime::new(
                std::sync::Arc::clone(&event_bus),
                std::sync::Arc::clone(&tool_registry),
                model_router.clone(),
            )
            .await?,
            model_router,
            event_bus,
            tool_registry,
            mcp_manager,
            agent_directory,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting Loom...");
        self.event_bus.start().await?;
        self.agent_runtime.start().await?;
        self.model_router.start().await?;
        tracing::info!("Loom started successfully");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Shutting down Loom...");
        self.mcp_manager.shutdown().await;
        self.model_router.shutdown().await?;
        self.agent_runtime.shutdown().await?;
        self.event_bus.shutdown().await?;
        telemetry::shutdown_telemetry();
        tracing::info!("Loom shut down successfully");
        Ok(())
    }
}
