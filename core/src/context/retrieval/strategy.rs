//! Retrieval strategies for context items.
//!
//! This module provides different strategies for retrieving relevant context items
//! from memory. Strategies can be combined and weighted for hybrid retrieval.

use crate::context::memory::MemoryStore;
use crate::context::types::{ContextItem, ContextItemType, MemoryQuery};
use crate::proto::Event;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

/// Trigger for context retrieval.
///
/// Contains all the information needed to decide what context is relevant.
#[derive(Debug, Clone)]
pub struct RetrievalTrigger {
    /// Current session ID
    pub session_id: String,

    /// Current agent ID
    pub agent_id: String,

    /// The event that triggered this retrieval (if any)
    pub event: Option<Event>,

    /// Current task or goal (if any)
    pub goal: Option<String>,

    /// Names of available tools
    pub available_tools: Vec<String>,

    /// Maximum number of items to retrieve
    pub max_items: usize,
}

impl RetrievalTrigger {
    /// Create a new trigger with default max_items
    pub fn new(session_id: String, agent_id: String) -> Self {
        Self {
            session_id,
            agent_id,
            event: None,
            goal: None,
            available_tools: Vec::new(),
            max_items: 50,
        }
    }

    /// Set the triggering event
    pub fn with_event(mut self, event: Event) -> Self {
        self.event = Some(event);
        self
    }

    /// Set the current goal
    pub fn with_goal(mut self, goal: String) -> Self {
        self.goal = Some(goal);
        self
    }

    /// Set available tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.available_tools = tools;
        self
    }

    /// Set max items
    pub fn with_max_items(mut self, max: usize) -> Self {
        self.max_items = max;
        self
    }
}

/// Strategy for retrieving context items from memory.
#[async_trait]
pub trait RetrievalStrategy: Send + Sync {
    /// Retrieve relevant context items based on the trigger
    async fn retrieve(
        &self,
        store: &dyn MemoryStore,
        trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>>;

    /// Get a human-readable name for this strategy
    fn name(&self) -> &str;
}

/// Retrieves the most recent N items.
///
/// This is the simplest strategy - just get the latest items in the session.
pub struct RecencyRetrieval {
    /// Number of items to retrieve
    pub window_size: usize,
}

impl RecencyRetrieval {
    pub fn new(window_size: usize) -> Arc<Self> {
        Arc::new(Self { window_size })
    }
}

#[async_trait]
impl RetrievalStrategy for RecencyRetrieval {
    async fn retrieve(
        &self,
        store: &dyn MemoryStore,
        trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>> {
        debug!(
            strategy = self.name(),
            session = %trigger.session_id,
            window_size = self.window_size,
            "Retrieving recent items"
        );

        let query = MemoryQuery::new()
            .for_session(trigger.session_id.clone())
            .limit(self.window_size);

        store.query(&query).await
    }

    fn name(&self) -> &str {
        "RecencyRetrieval"
    }
}

/// Retrieves items filtered by type.
///
/// Useful for getting only messages, only tool calls, etc.
pub struct TypeFilteredRetrieval {
    /// Types to include
    pub item_types: Vec<ContextItemType>,

    /// Maximum items to retrieve
    pub max_items: usize,
}

impl TypeFilteredRetrieval {
    pub fn new(item_types: Vec<ContextItemType>, max_items: usize) -> Arc<Self> {
        Arc::new(Self {
            item_types,
            max_items,
        })
    }

    /// Retrieve only messages
    pub fn messages_only(max_items: usize) -> Arc<Self> {
        use crate::context::types::MessageRole;
        Self::new(
            vec![
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                ContextItemType::Message {
                    role: MessageRole::Assistant,
                },
                ContextItemType::Message {
                    role: MessageRole::System,
                },
            ],
            max_items,
        )
    }

    /// Retrieve only tool-related items
    pub fn tools_only(max_items: usize) -> Arc<Self> {
        // Note: This will match any tool name, which might not be ideal
        // In practice, you'd want to get actual tool names from the trigger
        Self::new(vec![], max_items)
    }
}

#[async_trait]
impl RetrievalStrategy for TypeFilteredRetrieval {
    async fn retrieve(
        &self,
        store: &dyn MemoryStore,
        trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>> {
        debug!(
            strategy = self.name(),
            session = %trigger.session_id,
            types = ?self.item_types,
            "Retrieving type-filtered items"
        );

        let query = MemoryQuery::new()
            .for_session(trigger.session_id.clone())
            .with_types(self.item_types.clone())
            .limit(self.max_items);

        store.query(&query).await
    }

    fn name(&self) -> &str {
        "TypeFilteredRetrieval"
    }
}

/// Retrieves items based on importance threshold.
pub struct ImportanceRetrieval {
    /// Minimum importance score (0.0 to 1.0)
    pub min_importance: f32,

    /// Maximum items to retrieve
    pub max_items: usize,
}

impl ImportanceRetrieval {
    pub fn new(min_importance: f32, max_items: usize) -> Arc<Self> {
        Arc::new(Self {
            min_importance: min_importance.clamp(0.0, 1.0),
            max_items,
        })
    }
}

#[async_trait]
impl RetrievalStrategy for ImportanceRetrieval {
    async fn retrieve(
        &self,
        store: &dyn MemoryStore,
        trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>> {
        debug!(
            strategy = self.name(),
            session = %trigger.session_id,
            min_importance = self.min_importance,
            "Retrieving important items"
        );

        let mut query = MemoryQuery::new()
            .for_session(trigger.session_id.clone())
            .limit(self.max_items);

        query.min_importance = Some(self.min_importance);

        store.query(&query).await
    }

    fn name(&self) -> &str {
        "ImportanceRetrieval"
    }
}

/// Combines multiple retrieval strategies with weights.
///
/// Results are merged and deduplicated based on item IDs.
pub struct CompositeRetrieval {
    /// (strategy, weight) pairs
    strategies: Vec<(Arc<dyn RetrievalStrategy>, f32)>,
}

impl CompositeRetrieval {
    pub fn new(strategies: Vec<(Arc<dyn RetrievalStrategy>, f32)>) -> Arc<Self> {
        Arc::new(Self { strategies })
    }
}

#[async_trait]
impl RetrievalStrategy for CompositeRetrieval {
    async fn retrieve(
        &self,
        store: &dyn MemoryStore,
        trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>> {
        debug!(
            strategy = self.name(),
            num_strategies = self.strategies.len(),
            "Retrieving with composite strategy"
        );

        let mut all_items = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for (strategy, _weight) in &self.strategies {
            let items = strategy.retrieve(store, trigger).await?;

            for item in items {
                if seen_ids.insert(item.id.clone()) {
                    all_items.push(item);
                }
            }
        }

        // Note: weights are not yet applied to ranking
        // That would be done in the ranking phase
        Ok(all_items)
    }

    fn name(&self) -> &str {
        "CompositeRetrieval"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::memory::InMemoryStore;
    use crate::context::types::{ContextContent, ContextMetadata, MessageRole};

    async fn setup_test_store() -> Arc<InMemoryStore> {
        let store = InMemoryStore::new();

        // Add some test items
        for i in 0..10 {
            let item = ContextItem::new(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                ContextContent::from_string(format!("Message {}", i)),
                ContextMetadata::new("session1".to_string(), "agent1".to_string())
                    .with_importance(i as f32 / 10.0),
            );
            store.store(item).await.unwrap();
        }

        store
    }

    #[tokio::test]
    async fn test_recency_retrieval() {
        let store = setup_test_store().await;
        let strategy = RecencyRetrieval::new(5);

        let trigger = RetrievalTrigger::new("session1".to_string(), "agent1".to_string());

        let items = strategy.retrieve(&*store, &trigger).await.unwrap();
        assert_eq!(items.len(), 5);
    }

    #[tokio::test]
    async fn test_type_filtered_retrieval() {
        let store = setup_test_store().await;
        let strategy = TypeFilteredRetrieval::messages_only(10);

        let trigger = RetrievalTrigger::new("session1".to_string(), "agent1".to_string());

        let items = strategy.retrieve(&*store, &trigger).await.unwrap();
        assert_eq!(items.len(), 10);
        assert!(items.iter().all(|item| item.is_message()));
    }

    #[tokio::test]
    async fn test_importance_retrieval() {
        let store = setup_test_store().await;
        let strategy = ImportanceRetrieval::new(0.5, 10);

        let trigger = RetrievalTrigger::new("session1".to_string(), "agent1".to_string());

        let items = strategy.retrieve(&*store, &trigger).await.unwrap();

        // Should get items with importance >= 0.5
        assert!(items.len() <= 5); // items 5-9 have importance >= 0.5
        assert!(items.iter().all(|item| item.metadata.importance >= 0.5));
    }

    #[tokio::test]
    async fn test_composite_retrieval() {
        let store = setup_test_store().await;

        let recency = RecencyRetrieval::new(3);
        let importance = ImportanceRetrieval::new(0.7, 5);

        let strategy = CompositeRetrieval::new(vec![
            (recency as Arc<dyn RetrievalStrategy>, 0.5),
            (importance as Arc<dyn RetrievalStrategy>, 0.5),
        ]);

        let trigger = RetrievalTrigger::new("session1".to_string(), "agent1".to_string());

        let items = strategy.retrieve(&*store, &trigger).await.unwrap();

        // Should get items from both strategies, deduplicated
        assert!(items.len() > 0);
        assert!(items.len() <= 8); // 3 + 5, but with potential overlap
    }
}
