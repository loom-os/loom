//! Persistent RocksDB-based storage for context items.
//!
//! This provides a production-ready implementation of `MemoryStore` using RocksDB.
//! It maintains the same indexing patterns as `InMemoryStore` but persists data to disk.

use crate::context::types::{ContextItem, ContextItemType, MemoryQuery};
use crate::{LoomError, Result};
use async_trait::async_trait;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

use super::MemoryStore;

/// Column family names for indexing
const CF_ITEMS: &str = "items";
const CF_SESSION_INDEX: &str = "session_index";
const CF_TYPE_INDEX: &str = "type_index";
const CF_TIME_INDEX: &str = "time_index";

/// Persistent RocksDB-based implementation of MemoryStore.
///
/// Uses column families for efficient indexing:
/// - `items`: Main storage (item_id -> ContextItem)
/// - `session_index`: Session to item IDs mapping
/// - `type_index`: Item type to item IDs mapping
/// - `time_index`: Timestamp bucket to item IDs mapping
pub struct RocksDbStore {
    db: DB,
}

impl RocksDbStore {
    /// Create a new RocksDB store at the given path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Arc<Self>> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new(CF_ITEMS, Options::default()),
            ColumnFamilyDescriptor::new(CF_SESSION_INDEX, Options::default()),
            ColumnFamilyDescriptor::new(CF_TYPE_INDEX, Options::default()),
            ColumnFamilyDescriptor::new(CF_TIME_INDEX, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cf_descriptors)
            .map_err(|e| LoomError::StorageError(e.to_string()))?;

        info!("RocksDbStore initialized");
        Ok(Arc::new(Self { db }))
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

    /// Append an ID to an index (stored as JSON array)
    fn append_to_index(&self, cf_name: &str, key: &str, item_id: &str) -> Result<()> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| LoomError::StorageError(format!("Missing CF: {}", cf_name)))?;

        // Get existing IDs or create empty vec
        let mut ids: Vec<String> = match self.db.get_cf(&cf, key) {
            Ok(Some(data)) => serde_json::from_slice(&data)?,
            Ok(None) => vec![],
            Err(e) => return Err(LoomError::StorageError(e.to_string())),
        };

        ids.push(item_id.to_string());

        let serialized = serde_json::to_vec(&ids)?;
        self.db
            .put_cf(&cf, key, serialized)
            .map_err(|e| LoomError::StorageError(e.to_string()))
    }

    /// Get IDs from an index
    fn get_index_ids(&self, cf_name: &str, key: &str) -> Result<Vec<String>> {
        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or_else(|| LoomError::StorageError(format!("Missing CF: {}", cf_name)))?;

        match self.db.get_cf(&cf, key) {
            Ok(Some(data)) => Ok(serde_json::from_slice(&data)?),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(LoomError::StorageError(e.to_string())),
        }
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

#[async_trait]
impl MemoryStore for RocksDbStore {
    async fn store(&self, item: ContextItem) -> Result<()> {
        let cf = self
            .db
            .cf_handle(CF_ITEMS)
            .ok_or_else(|| LoomError::StorageError("Missing items CF".to_string()))?;

        // Store the item
        let serialized = serde_json::to_vec(&item)?;
        self.db
            .put_cf(&cf, &item.id, serialized)
            .map_err(|e| LoomError::StorageError(e.to_string()))?;

        // Update indices
        self.append_to_index(CF_SESSION_INDEX, &item.metadata.session_id, &item.id)?;

        let type_key = Self::type_key(&item.item_type);
        self.append_to_index(CF_TYPE_INDEX, &type_key, &item.id)?;

        let time_bucket = (item.metadata.timestamp_ms / 1000).to_string();
        self.append_to_index(CF_TIME_INDEX, &time_bucket, &item.id)?;

        debug!(item_id = %item.id, "Stored context item");
        Ok(())
    }

    async fn store_batch(&self, items: Vec<ContextItem>) -> Result<()> {
        for item in items {
            self.store(item).await?;
        }
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<ContextItem>> {
        let cf = self
            .db
            .cf_handle(CF_ITEMS)
            .ok_or_else(|| LoomError::StorageError("Missing items CF".to_string()))?;

        match self.db.get_cf(&cf, id) {
            Ok(Some(data)) => Ok(Some(serde_json::from_slice(&data)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(LoomError::StorageError(e.to_string())),
        }
    }

    async fn query(&self, query: &MemoryQuery) -> Result<Vec<ContextItem>> {
        // Use session index if session filter is provided
        let candidate_ids: Vec<String> = if let Some(ref session_id) = query.session_id {
            self.get_index_ids(CF_SESSION_INDEX, session_id)?
        } else {
            // Scan all items (expensive for large datasets)
            let cf = self
                .db
                .cf_handle(CF_ITEMS)
                .ok_or_else(|| LoomError::StorageError("Missing items CF".to_string()))?;

            let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
            iter.filter_map(|r| r.ok())
                .map(|(k, _)| String::from_utf8_lossy(&k).to_string())
                .collect()
        };

        // Filter and collect matching items
        let mut results = Vec::new();
        for id in candidate_ids {
            if let Some(item) = self.get(&id).await? {
                if self.matches_query(&item, query) {
                    results.push(item);
                }
            }

            // Apply limit
            if results.len() >= query.limit {
                break;
            }
        }

        // Sort by timestamp (most recent first)
        results.sort_by(|a, b| b.metadata.timestamp_ms.cmp(&a.metadata.timestamp_ms));

        Ok(results)
    }

    async fn get_related(&self, item_id: &str) -> Result<Vec<ContextItem>> {
        // Get the source item to find its related items
        let source = self.get(item_id).await?;
        let mut related = Vec::new();

        if let Some(item) = source {
            // Get related items
            for related_id in &item.metadata.related_items {
                if let Some(rel_item) = self.get(related_id).await? {
                    related.push(rel_item);
                }
            }
        }

        Ok(related)
    }

    async fn count(&self) -> Result<usize> {
        let cf = self
            .db
            .cf_handle(CF_ITEMS)
            .ok_or_else(|| LoomError::StorageError("Missing items CF".to_string()))?;

        let count = self
            .db
            .iterator_cf(&cf, rocksdb::IteratorMode::Start)
            .count();

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::{ContextContent, ContextMetadata, MessageRole};
    use tempfile::tempdir;

    fn make_test_item(id: &str, session: &str, content: &str) -> ContextItem {
        ContextItem {
            id: id.to_string(),
            item_type: ContextItemType::Message {
                role: MessageRole::User,
            },
            content: ContextContent::from_string(content.to_string()),
            metadata: ContextMetadata::new(session.to_string(), "test-agent".to_string())
                .with_importance(0.5),
        }
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::new(dir.path()).unwrap();

        let item = make_test_item("item-1", "session-1", "Hello world");
        store.store(item.clone()).await.unwrap();

        let retrieved = store.get("item-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "item-1");
    }

    #[tokio::test]
    async fn test_query_by_session() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::new(dir.path()).unwrap();

        // Store items in different sessions
        store
            .store(make_test_item("item-1", "session-a", "Hello"))
            .await
            .unwrap();
        store
            .store(make_test_item("item-2", "session-a", "World"))
            .await
            .unwrap();
        store
            .store(make_test_item("item-3", "session-b", "Other"))
            .await
            .unwrap();

        // Query session-a
        let query = MemoryQuery::new().for_session("session-a".to_string());
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_count() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::new(dir.path()).unwrap();

        assert_eq!(store.count().await.unwrap(), 0);

        store
            .store(make_test_item("item-1", "s1", "Hello"))
            .await
            .unwrap();
        store
            .store(make_test_item("item-2", "s1", "World"))
            .await
            .unwrap();

        assert_eq!(store.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_store_batch() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::new(dir.path()).unwrap();

        let items = vec![
            make_test_item("batch-1", "session", "First"),
            make_test_item("batch-2", "session", "Second"),
            make_test_item("batch-3", "session", "Third"),
        ];

        store.store_batch(items).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let dir = tempdir().unwrap();
        let store = RocksDbStore::new(dir.path()).unwrap();

        for i in 0..10 {
            store
                .store(make_test_item(&format!("item-{}", i), "session", "Content"))
                .await
                .unwrap();
        }

        let query = MemoryQuery::new()
            .for_session("session".to_string())
            .limit(3);
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
