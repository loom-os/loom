//! Context Engineering System
//!
//! This module provides a modern context pipeline for building LLM-ready prompts
//! from agent state, memory, and tool traces.
//!
//! # Architecture
//!
//! - **Types**: Core types (ContextItem, ContextContent, ContextMetadata)
//! - **Memory**: Storage and retrieval of context items
//! - **Retrieval**: Strategies for finding relevant context
//! - **Ranking**: Strategies for ordering context by relevance
//! - **Window**: Token budget management
//! - **Pipeline**: Orchestration of full retrieval→ranking→windowing flow
//! - **AgentContext**: High-level API for agents
//! - **Builder**: Legacy prompt bundle builder (will be replaced by pipeline)
//!
//! # Design Principles
//!
//! 1. **Everything is Retrievable**: No irreversible summarization
//! 2. **Full Traceability**: All items linked via OpenTelemetry traces
//! 3. **Tool-First**: Tool calls and results are first-class citizens
//! 4. **Intelligent Selection**: Dynamic context windowing based on relevance

pub mod agent_context;
pub mod builder;
pub mod memory;
pub mod pipeline;
pub mod ranking;
pub mod retrieval;
pub mod types;
pub mod window;

pub use types::{
    ContextContent, ContextItem, ContextItemType, ContextMetadata, MemoryQuery, MessageRole,
};

pub use memory::{InMemoryStore, MemoryStore};

pub use retrieval::{
    CompositeRetrieval, ImportanceRetrieval, RecencyRetrieval, RetrievalStrategy, RetrievalTrigger,
    TypeFilteredRetrieval,
};

pub use ranking::{CompositeRanker, ContextRanker, ImportanceRanker, TemporalRanker};

pub use window::{TiktokenCounter, TokenCounter, WindowConfig, WindowManager};

pub use pipeline::{ContextPipeline, PipelineConfig, PipelineResult};

pub use agent_context::AgentContext;

use serde::{Deserialize, Serialize};

/// Token budget to control prompt assembly size
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenBudget {
    pub max_input_tokens: usize,
    pub max_output_tokens: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_input_tokens: 2048,
            max_output_tokens: 512,
        }
    }
}

/// A bundle of prompt components for an LLM call
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptBundle {
    pub system: String,
    pub instructions: String,
    pub tools_json_schema: Option<String>,
    pub context_docs: Vec<String>,
    pub history: Vec<String>,
}

/// Abstraction for writing memory (events, summaries)
#[async_trait::async_trait]
pub trait MemoryWriter: Send + Sync {
    async fn append_event(&self, session: &str, event: crate::proto::Event) -> crate::Result<()>;
    async fn summarize_episode(&self, session: &str) -> crate::Result<Option<String>>;
}

/// Abstraction for reading memory (retrieval)
#[async_trait::async_trait]
pub trait MemoryReader: Send + Sync {
    async fn retrieve(
        &self,
        query: &str,
        k: usize,
        filters: Option<serde_json::Value>,
    ) -> crate::Result<Vec<String>>;
}
