//! Memory storage interface and implementations.
//!
//! Provides a unified interface for storing and retrieving context items,
//! with support for queries, indexing, and relationships.

use crate::context::types::{ContextItem, ContextItemType, MemoryQuery};
use crate::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, trace};

/// Unified storage interface for context items.
///
/// Items are stored immutably - they are never modified or deleted.
/// This ensures full traceability and allows for temporal queries.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a single context item
    async fn store(&self, item: ContextItem) -> Result<()>;

    /// Store multiple context items in a batch
    async fn store_batch(&self, items: Vec<ContextItem>) -> Result<()>;

    /// Retrieve a specific item by ID
    async fn get(&self, id: &str) -> Result<Option<ContextItem>>;

    /// Query items based on filters
    async fn query(&self, query: &MemoryQuery) -> Result<Vec<ContextItem>>;

    /// Get items related to a specific item
    async fn get_related(&self, item_id: &str) -> Result<Vec<ContextItem>>;

    /// Get the total count of items (for metrics)
    async fn count(&self) -> Result<usize>;
}

/// In-memory implementation of MemoryStore.
///
/// Uses DashMap for concurrent access. Suitable for development and testing.
/// For production, consider implementing a persistent backend (PostgreSQL, etc.).
pub struct InMemoryStore {
    /// Main storage: item_id -> ContextItem
    items: DashMap<String, ContextItem>,

    /// Index by session_id for fast session queries
    session_index: DashMap<String, Vec<String>>,

    /// Index by item type for fast type filtering
    type_index: DashMap<String, Vec<String>>,

    /// Index by timestamp for temporal queries (timestamp_ms -> item_ids)
    time_index: DashMap<i64, Vec<String>>,
}

impl InMemoryStore {
    /// Create a new in-memory store
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            items: DashMap::new(),
            session_index: DashMap::new(),
            type_index: DashMap::new(),
            time_index: DashMap::new(),
        })
    }

    /// Get the type key for indexing
    fn type_key(item_type: &ContextItemType) -> String {
        match item_type {
            ContextItemType::Message { role } => format!("message:{:?}", role),
            ContextItemType::ToolCall { tool_name } => format!("tool_call:{}", tool_name),
            ContextItemType::ToolResult { tool_name, success } => {
                format!("tool_result:{}:{}", tool_name, success)
            }
            ContextItemType::Event { event_type } => format!("event:{}", event_type),
            ContextItemType::Observation { source } => format!("observation:{}", source),
        }
    }

    /// Update indices for a new item
    fn update_indices(&self, item: &ContextItem) {
        // Session index
        self.session_index
            .entry(item.metadata.session_id.clone())
            .or_default()
            .push(item.id.clone());

        // Type index
        let type_key = Self::type_key(&item.item_type);
        self.type_index
            .entry(type_key)
            .or_default()
            .push(item.id.clone());

        // Time index (group by second for efficiency)
        let time_bucket = item.metadata.timestamp_ms / 1000;
        self.time_index
            .entry(time_bucket)
            .or_default()
            .push(item.id.clone());
    }

    /// Match an item against query filters
    fn matches_query(&self, item: &ContextItem, query: &MemoryQuery) -> bool {
        // Session filter
        if let Some(ref session_id) = query.session_id {
            if &item.metadata.session_id != session_id {
                return false;
            }
        }

        // Agent filter
        if let Some(ref agent_id) = query.agent_id {
            if &item.metadata.agent_id != agent_id {
                return false;
            }
        }

        // Type filter
        if let Some(ref types) = query.item_types {
            if !types.contains(&item.item_type) {
                return false;
            }
        }

        // Time range filter
        if let Some((start, end)) = query.time_range {
            let ts = item.metadata.timestamp_ms;
            if ts < start || ts > end {
                return false;
            }
        }

        // Tags filter (all must match)
        if let Some(ref tags) = query.tags {
            for (key, value) in tags {
                if item.metadata.tags.get(key) != Some(value) {
                    return false;
                }
            }
        }

        // Importance filter
        if let Some(min_importance) = query.min_importance {
            if item.metadata.importance < min_importance {
                return false;
            }
        }

        true
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self {
            items: DashMap::new(),
            session_index: DashMap::new(),
            type_index: DashMap::new(),
            time_index: DashMap::new(),
        }
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn store(&self, item: ContextItem) -> Result<()> {
        trace!(
            item_id = %item.id,
            item_type = ?item.item_type,
            session = %item.metadata.session_id,
            "Storing context item"
        );

        self.update_indices(&item);
        self.items.insert(item.id.clone(), item);

        Ok(())
    }

    async fn store_batch(&self, items: Vec<ContextItem>) -> Result<()> {
        debug!(count = items.len(), "Storing batch of context items");

        for item in items {
            self.store(item).await?;
        }

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<ContextItem>> {
        Ok(self.items.get(id).map(|entry| entry.value().clone()))
    }

    async fn query(&self, query: &MemoryQuery) -> Result<Vec<ContextItem>> {
        trace!(
            session = ?query.session_id,
            limit = query.limit,
            "Querying context items"
        );

        // Start with session index if session_id is specified (most common case)
        let candidate_ids: Vec<String> = if let Some(ref session_id) = query.session_id {
            self.session_index
                .get(session_id)
                .map(|entry| entry.value().clone())
                .unwrap_or_default()
        } else {
            // Otherwise iterate all items
            self.items.iter().map(|entry| entry.key().clone()).collect()
        };

        // Filter and collect matching items
        let mut results: Vec<ContextItem> = candidate_ids
            .iter()
            .filter_map(|id| self.items.get(id).map(|entry| entry.value().clone()))
            .filter(|item| self.matches_query(item, query))
            .collect();

        // Sort by timestamp (newest first by default)
        results.sort_by(|a, b| b.metadata.timestamp_ms.cmp(&a.metadata.timestamp_ms));

        // Apply offset and limit
        let start = query.offset.min(results.len());
        let end = (start + query.limit).min(results.len());
        let results = results[start..end].to_vec();

        debug!(
            matched = results.len(),
            total_candidates = candidate_ids.len(),
            "Query completed"
        );

        Ok(results)
    }

    async fn get_related(&self, item_id: &str) -> Result<Vec<ContextItem>> {
        let item = match self.get(item_id).await? {
            Some(item) => item,
            None => return Ok(Vec::new()),
        };

        let mut related = Vec::new();
        for related_id in &item.metadata.related_items {
            if let Some(related_item) = self.get(related_id).await? {
                related.push(related_item);
            }
        }

        Ok(related)
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.items.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::{ContextContent, ContextMetadata, MessageRole};

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = InMemoryStore::new();

        let item = ContextItem::new(
            ContextItemType::Message {
                role: MessageRole::User,
            },
            ContextContent::from_string("Hello".to_string()),
            ContextMetadata::new("session1".to_string(), "agent1".to_string()),
        );

        let item_id = item.id.clone();
        store.store(item).await.unwrap();

        let retrieved = store.get(&item_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content.text, "Hello");
    }

    #[tokio::test]
    async fn test_query_by_session() {
        let store = InMemoryStore::new();

        // Store items in different sessions
        for i in 0..5 {
            let session = if i < 3 { "session1" } else { "session2" };
            let item = ContextItem::new(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                ContextContent::from_string(format!("Message {}", i)),
                ContextMetadata::new(session.to_string(), "agent1".to_string()),
            );
            store.store(item).await.unwrap();
        }

        // Query session1
        let query = MemoryQuery::new().for_session("session1".to_string());
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 3);

        // Query session2
        let query = MemoryQuery::new().for_session("session2".to_string());
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_query_by_type() {
        let store = InMemoryStore::new();

        // Store different types
        let msg = ContextItem::new(
            ContextItemType::Message {
                role: MessageRole::User,
            },
            ContextContent::from_string("Hello".to_string()),
            ContextMetadata::new("s1".to_string(), "a1".to_string()),
        );

        let tool_call = ContextItem::new(
            ContextItemType::ToolCall {
                tool_name: "search".to_string(),
            },
            ContextContent::from_string("search query".to_string()),
            ContextMetadata::new("s1".to_string(), "a1".to_string()),
        );

        store.store(msg).await.unwrap();
        store.store(tool_call).await.unwrap();

        // Query only messages
        let query = MemoryQuery::new()
            .for_session("s1".to_string())
            .with_types(vec![ContextItemType::Message {
                role: MessageRole::User,
            }]);

        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_message());
    }

    #[tokio::test]
    async fn test_related_items() {
        let store = InMemoryStore::new();

        // Create a tool call
        let call = ContextItem::new(
            ContextItemType::ToolCall {
                tool_name: "search".to_string(),
            },
            ContextContent::from_string("search query".to_string()),
            ContextMetadata::new("s1".to_string(), "a1".to_string()),
        );

        let call_id = call.id.clone();
        store.store(call).await.unwrap();

        // Create a related result
        let result = ContextItem::new(
            ContextItemType::ToolResult {
                tool_name: "search".to_string(),
                success: true,
            },
            ContextContent::from_string("search results".to_string()),
            ContextMetadata::new("s1".to_string(), "a1".to_string())
                .with_related_item(call_id.clone()),
        );

        store.store(result).await.unwrap();

        // Query related items
        let related = store.get_related(&call_id).await.unwrap();
        assert_eq!(related.len(), 0); // call doesn't reference result

        // But result references call
        let all_items = store
            .query(&MemoryQuery::new().for_session("s1".to_string()))
            .await
            .unwrap();
        let result_item = all_items.iter().find(|i| i.is_tool_result()).unwrap();
        assert!(result_item.metadata.related_items.contains(&call_id));
    }

    #[tokio::test]
    async fn test_batch_store() {
        let store = InMemoryStore::new();

        let items: Vec<ContextItem> = (0..10)
            .map(|i| {
                ContextItem::new(
                    ContextItemType::Message {
                        role: MessageRole::User,
                    },
                    ContextContent::from_string(format!("Message {}", i)),
                    ContextMetadata::new("s1".to_string(), "a1".to_string()),
                )
            })
            .collect();

        store.store_batch(items).await.unwrap();

        let count = store.count().await.unwrap();
        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_query_with_limit_and_offset() {
        let store = InMemoryStore::new();

        // Store 20 items
        for i in 0..20 {
            let item = ContextItem::new(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                ContextContent::from_string(format!("Message {}", i)),
                ContextMetadata::new("s1".to_string(), "a1".to_string()),
            );
            store.store(item).await.unwrap();
        }

        // Page 1: first 10
        let query = MemoryQuery::new().for_session("s1".to_string()).limit(10);
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 10);

        // Page 2: next 10
        let query = MemoryQuery::new()
            .for_session("s1".to_string())
            .limit(10)
            .offset(10);
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 10);

        // Page 3: empty
        let query = MemoryQuery::new()
            .for_session("s1".to_string())
            .limit(10)
            .offset(20);
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 0);
    }
}
