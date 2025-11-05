use super::{MemoryReader, MemoryWriter, PromptBundle, TokenBudget};
use crate::Result;
use std::sync::Arc;
use tracing::debug;

/// Input that triggers context construction
#[derive(Debug, Clone)]
pub struct TriggerInput {
    pub session_id: String,
    pub goal: Option<String>,
    pub tool_hints: Vec<String>,
    pub budget: TokenBudget,
}

/// ContextBuilder assembles a PromptBundle from memory and recent events
pub struct ContextBuilder<R: MemoryReader, W: MemoryWriter> {
    reader: Arc<R>,
    writer: Arc<W>,
}

impl<R: MemoryReader, W: MemoryWriter> ContextBuilder<R, W> {
    pub fn new(reader: Arc<R>, writer: Arc<W>) -> Self {
        Self { reader, writer }
    }

    /// Build a minimal prompt bundle; this is a skeleton to be expanded
    pub async fn build(&self, trigger: TriggerInput) -> Result<PromptBundle> {
        debug!(target: "context_builder", session = %trigger.session_id, "Building prompt bundle");

        // Pull episodic summary and run simple retrieval against in-memory store
        let mut context_docs: Vec<String> = Vec::new();

        if let Ok(Some(summary)) = self.writer.summarize_episode(&trigger.session_id).await {
            if !summary.is_empty() {
                context_docs.push(format!("Recent episode summary:\n{}", summary));
            }
        }

        let retrieved = self
            .reader
            .retrieve(trigger.goal.as_deref().unwrap_or(""), 4, None)
            .await
            .unwrap_or_default();
        if !retrieved.is_empty() {
            context_docs.push("Retrieved context:".to_string());
            for r in retrieved {
                context_docs.push(r);
            }
        }

        // Assemble prompt bundle; history is left empty at P0 (no dialog turns tracked yet)
        Ok(PromptBundle {
            system: "You are Loom Agent. Be concise and precise.".to_string(),
            instructions: trigger.goal.unwrap_or_default(),
            tools_json_schema: None,
            context_docs,
            history: vec![],
        })
    }
}
