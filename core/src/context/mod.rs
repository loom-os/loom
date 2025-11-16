pub mod builder;
pub mod memory;

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
