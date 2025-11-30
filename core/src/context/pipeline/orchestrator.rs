//! Context Pipeline Orchestrator
//!
//! Coordinates retrieval → ranking → windowing → assembly flow

use crate::context::memory::MemoryStore;
use crate::context::ranking::ContextRanker;
use crate::context::retrieval::{RetrievalStrategy, RetrievalTrigger};
use crate::context::types::ContextItem;
use crate::context::window::WindowManager;
use crate::Result;
use std::sync::Arc;
use tracing::{debug, info, instrument};

/// Configuration for the context pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Maximum items to retrieve before ranking
    pub max_retrieval_items: usize,

    /// Minimum importance score for retrieval (0.0-1.0)
    pub min_importance: f32,

    /// Whether to include related items (expands context)
    pub include_related: bool,

    /// Maximum depth for related item expansion
    pub related_depth: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_retrieval_items: 100,
            min_importance: 0.0,
            include_related: true,
            related_depth: 1,
        }
    }
}

/// Result of pipeline execution
#[derive(Debug)]
pub struct PipelineResult {
    /// Final selected context items (ready for LLM prompt)
    pub items: Vec<ContextItem>,

    /// Total tokens used
    pub tokens_used: usize,

    /// Items that were retrieved but didn't fit in window
    pub overflow_items: Vec<ContextItem>,

    /// Token budget available
    pub budget: usize,

    /// Number of items retrieved before ranking
    pub retrieved_count: usize,

    /// Number of items after ranking
    pub ranked_count: usize,
}

/// Main context pipeline orchestrator
///
/// Coordinates the full flow:
/// 1. **Retrieve** relevant items from memory (configurable strategy)
/// 2. **Rank** items by relevance/importance
/// 3. **Window** selection based on token budget
/// 4. **Assemble** final context for LLM
pub struct ContextPipeline {
    store: Arc<dyn MemoryStore>,
    retrieval: Arc<dyn RetrievalStrategy>,
    ranker: Arc<dyn ContextRanker>,
    window: WindowManager,
    config: PipelineConfig,
}

impl ContextPipeline {
    pub fn new(
        store: Arc<dyn MemoryStore>,
        retrieval: Arc<dyn RetrievalStrategy>,
        ranker: Arc<dyn ContextRanker>,
        window: WindowManager,
        config: PipelineConfig,
    ) -> Self {
        Self {
            store,
            retrieval,
            ranker,
            window,
            config,
        }
    }

    /// Execute the full pipeline for a given query
    #[instrument(skip(self, trigger), fields(session = %trigger.session_id))]
    pub async fn execute(&self, trigger: RetrievalTrigger) -> Result<PipelineResult> {
        info!("Pipeline execution started: session={}", trigger.session_id);

        // Phase 1: Retrieval
        debug!("Phase 1: Retrieving items (max={})", trigger.max_items);
        let mut retrieved_items = self.retrieval.retrieve(&*self.store, &trigger).await?;
        let retrieved_count = retrieved_items.len();
        debug!("Retrieved {} items", retrieved_count);

        // Phase 1.5: Related items expansion (optional)
        if self.config.include_related && !retrieved_items.is_empty() {
            debug!(
                "Expanding with related items (depth={})",
                self.config.related_depth
            );
            retrieved_items = self.expand_related(retrieved_items).await?;
            debug!("After expansion: {} items", retrieved_items.len());
        }

        // Phase 2: Ranking
        debug!("Phase 2: Ranking items");
        let ranked_items = self.ranker.rank(retrieved_items, &trigger).await?;
        let ranked_count = ranked_items.len();
        debug!("Ranked {} items", ranked_count);

        // Phase 3: Window selection
        debug!(
            "Phase 3: Window selection (budget={})",
            self.window.config().available_tokens()
        );
        let selection = self.window.select_items(ranked_items);

        info!(
            "Pipeline complete: {}/{} items selected, {}/{} tokens used",
            selection.selected.len(),
            retrieved_count,
            selection.tokens_used,
            selection.budget
        );

        Ok(PipelineResult {
            items: selection.selected,
            tokens_used: selection.tokens_used,
            overflow_items: selection.overflow,
            budget: selection.budget,
            retrieved_count,
            ranked_count,
        })
    }

    /// Expand context with related items
    async fn expand_related(&self, items: Vec<ContextItem>) -> Result<Vec<ContextItem>> {
        let mut all_items = items;
        let mut seen_ids: std::collections::HashSet<String> =
            all_items.iter().map(|i| i.id.clone()).collect();

        for depth in 1..=self.config.related_depth {
            let mut new_items = Vec::new();

            // Get related items for current set
            for item in &all_items {
                if item.metadata.related_items.is_empty() {
                    continue;
                }

                for related_id in &item.metadata.related_items {
                    if seen_ids.contains(related_id) {
                        continue;
                    }

                    if let Some(related) = self.store.get(related_id).await? {
                        seen_ids.insert(related_id.clone());
                        new_items.push(related);
                    }
                }
            }

            if new_items.is_empty() {
                debug!("No new related items at depth {}", depth);
                break;
            }

            debug!("Added {} related items at depth {}", new_items.len(), depth);
            all_items.extend(new_items);
        }

        Ok(all_items)
    }

    /// Update pipeline configuration
    pub fn set_config(&mut self, config: PipelineConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Get window manager (for config updates)
    pub fn window_mut(&mut self) -> &mut WindowManager {
        &mut self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::memory::InMemoryStore;
    use crate::context::ranking::TemporalRanker;
    use crate::context::retrieval::RecencyRetrieval;
    use crate::context::types::{ContextContent, ContextItemType, ContextMetadata, MessageRole};
    use crate::context::window::{TiktokenCounter, WindowConfig};

    async fn create_test_pipeline() -> ContextPipeline {
        let store: Arc<dyn MemoryStore> = InMemoryStore::new();
        let retrieval: Arc<dyn RetrievalStrategy> = RecencyRetrieval::new(100);
        let ranker = TemporalRanker::newest_first();
        let counter = Arc::new(TiktokenCounter::gpt4());
        let window = WindowManager::new(counter, WindowConfig::default());

        ContextPipeline::new(store, retrieval, ranker, window, PipelineConfig::default())
    }

    fn create_test_item(session: &str, content: &str, importance: f32) -> ContextItem {
        // Use timestamp-based ID to avoid rand dependency
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        ContextItem {
            id: format!("item_{}", timestamp),
            item_type: ContextItemType::Message {
                role: MessageRole::User,
            },
            content: ContextContent {
                raw: serde_json::json!(content),
                text: content.to_string(),
                token_count: None,
                embedding: None,
            },
            metadata: ContextMetadata::new(session.to_string(), "test".to_string())
                .with_importance(importance),
        }
    }

    #[tokio::test]
    async fn test_pipeline_basic_flow() {
        let pipeline = create_test_pipeline().await;

        // Add some test items
        for i in 0..5 {
            let item = create_test_item("session1", &format!("Message {}", i), 0.5);
            pipeline.store.store(item).await.unwrap();
        }

        // Execute pipeline
        let trigger = RetrievalTrigger::new("session1".to_string(), "test".to_string());

        let result = pipeline.execute(trigger).await.unwrap();

        // Should retrieve all items
        assert_eq!(result.retrieved_count, 5);
        assert_eq!(result.ranked_count, 5);

        // All should fit in default window
        assert_eq!(result.items.len(), 5);
        assert!(result.tokens_used > 0);
        assert!(result.tokens_used < result.budget);
    }

    #[tokio::test]
    async fn test_pipeline_with_overflow() {
        let mut pipeline = create_test_pipeline().await;

        // Set very small window
        let mut small_config = WindowConfig::default();
        small_config.max_tokens = 500; // Very small
        pipeline.window_mut().set_config(small_config);

        // Add many large items
        for i in 0..10 {
            let large_content = format!("Message {} - {}", i, "x".repeat(500));
            let item = create_test_item("session1", &large_content, 0.5);
            pipeline.store.store(item).await.unwrap();
        }

        let trigger = RetrievalTrigger::new("session1".to_string(), "test".to_string());

        let result = pipeline.execute(trigger).await.unwrap();

        // Should have retrieved all
        assert_eq!(result.retrieved_count, 10);

        // But not all fit in window
        assert!(result.items.len() < 10);
        assert!(!result.overflow_items.is_empty());
        assert!(result.tokens_used <= result.budget);
    }

    #[tokio::test]
    async fn test_pipeline_with_related_items() {
        let pipeline = create_test_pipeline().await;

        // Create items with relationships
        let item1 = create_test_item("session1", "First message", 0.8);
        let id1 = item1.id.clone();

        let mut item2 = create_test_item("session1", "Second message", 0.6);
        item2.metadata.related_items.push(id1.clone());
        let id2 = item2.id.clone();

        let mut item3 = create_test_item("session1", "Third message", 0.4);
        item3.metadata.related_items.push(id2.clone());

        pipeline.store.store(item1).await.unwrap();
        pipeline.store.store(item2).await.unwrap();
        pipeline.store.store(item3).await.unwrap();

        let trigger = RetrievalTrigger::new("session1".to_string(), "test".to_string());

        let result = pipeline.execute(trigger).await.unwrap();

        // Should retrieve all items (RecencyRetrieval gets them all)
        // Then expand related items
        assert!(result.retrieved_count >= 3);
        assert!(!result.items.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_empty_store() {
        let pipeline = create_test_pipeline().await;

        let trigger = RetrievalTrigger::new("session1".to_string(), "test".to_string());

        let result = pipeline.execute(trigger).await.unwrap();

        assert_eq!(result.retrieved_count, 0);
        assert_eq!(result.items.len(), 0);
        assert_eq!(result.tokens_used, 0);
    }

    #[tokio::test]
    async fn test_pipeline_config_update() {
        let mut pipeline = create_test_pipeline().await;

        // Update config
        let mut new_config = PipelineConfig::default();
        new_config.max_retrieval_items = 50;
        new_config.include_related = false;
        pipeline.set_config(new_config);

        assert_eq!(pipeline.config().max_retrieval_items, 50);
        assert!(!pipeline.config().include_related);
    }
}
