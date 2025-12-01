// Loom Core Library
// Event-driven AI operating system runtime

pub mod agent;
pub mod cognitive; // LLM + Cognitive Loop (perceive-think-act)
pub mod collab; // Collaboration primitives built on EventBus + Envelope
pub mod context;
pub mod dashboard; // Real-time event flow visualization
pub mod envelope; // Unified metadata envelope for events/actions threads
pub mod event;
pub mod plugin;
pub mod storage;
pub mod telemetry;
pub mod tools; // Unified tool system (Native + MCP)

// Export core types
pub use agent::directory::{AgentDirectory, AgentInfo, AgentStatus, CapabilityDirectory};
pub use agent::{Agent, AgentRuntime, AgentState};
pub use cognitive::llm::router::{
    ConfidenceEstimator, DummyConfidenceEstimator, ModelRouter, Route, RoutingDecision,
};
pub use cognitive::llm::{LlmClient, LlmClientConfig, LlmResponse};
pub use collab::{types as collab_types, Collaborator};
pub use context::{builder::ContextBuilder, PromptBundle, TokenBudget};
pub use envelope::{agent_reply_topic, Envelope, ThreadTopicKind};
pub use event::{Event, EventBus, EventExt, EventHandler, QoSLevel};
#[allow(deprecated)]
pub use plugin::{Plugin, PluginManager};
pub use telemetry::{init_telemetry, shutdown_telemetry, SpanCollector, SpanData};
pub use tools::{Tool, ToolError, ToolRegistry};
// Re-export MCP types from tools
pub use tools::mcp::{McpClient, McpManager, McpToolAdapter};

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
    pub tool_registry: std::sync::Arc<ToolRegistry>,
    pub mcp_manager: std::sync::Arc<tools::mcp::McpManager>,
    pub agent_directory: std::sync::Arc<AgentDirectory>,
}

impl Loom {
    pub async fn new() -> Result<Self> {
        // Note: OpenTelemetry should be initialized BEFORE creating Loom
        // by calling telemetry::init_telemetry() in your main function.
        // This ensures the global tracing subscriber is set up correctly.

        let event_bus = std::sync::Arc::new(EventBus::new().await?);
        let tool_registry = std::sync::Arc::new(ToolRegistry::new());
        let agent_directory = std::sync::Arc::new(AgentDirectory::new());
        // Initialize router first so we can pass a clone to the agent runtime
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

            // Register native tools
            // TODO: Get workspace root from config
            let workspace_root = std::env::current_dir().unwrap_or_default();
            tool_registry
                .register(SyncArc::new(ReadFileTool::new(workspace_root)))
                .await;

            // Shell tool with limited commands for safety
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

        // Initialize MCP manager
        let mcp_manager = std::sync::Arc::new(tools::mcp::McpManager::new(std::sync::Arc::clone(
            &tool_registry,
        )));

        Ok(Self {
            agent_runtime: AgentRuntime::new(
                std::sync::Arc::clone(&event_bus),
                std::sync::Arc::clone(&tool_registry),
                model_router.clone(),
            )
            .await?,
            model_router,
            plugin_manager: PluginManager::new().await?,
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
