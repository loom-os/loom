// Loom Core Library
// Event-driven AI operating system runtime

pub mod agent;
pub mod event;
pub mod plugin;
pub mod router;
pub mod storage;
pub mod telemetry;

// Export core types
pub use agent::{Agent, AgentRuntime, AgentState};
pub use event::{Event, EventBus, EventHandler, QoSLevel};
pub use plugin::{Plugin, PluginManager};
pub use router::{ModelRouter, Route, RoutingDecision};

// Generated proto code
pub mod proto {
    include!(concat!(env!("OUT_DIR"),concat!("/","loom.v1",".rs")));
}

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
    pub event_bus: EventBus,
    pub agent_runtime: AgentRuntime,
    pub model_router: ModelRouter,
    pub plugin_manager: PluginManager,
}

impl Loom {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            event_bus: EventBus::new().await?,
            agent_runtime: AgentRuntime::new().await?,
            model_router: ModelRouter::new().await?,
            plugin_manager: PluginManager::new().await?,
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

        self.plugin_manager.shutdown().await?;
        self.model_router.shutdown().await?;
        self.agent_runtime.shutdown().await?;
        self.event_bus.shutdown().await?;

        tracing::info!("Loom shut down successfully");
        Ok(())
    }
}
