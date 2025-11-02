// Storage layer implementation (RocksDB)
use crate::{LoomError, Result};
use rocksdb::{Options, DB};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use tracing::info;

/// 持久化存储
pub struct Storage {
    db: DB,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = DB::open(&opts, path).map_err(|e| LoomError::StorageError(e.to_string()))?;

        info!("Storage initialized");
        Ok(Self { db })
    }

    /// Store key-value pair
    pub fn put<K: AsRef<[u8]>, V: Serialize>(&self, key: K, value: &V) -> Result<()> {
        let serialized = serde_json::to_vec(value)?;
        self.db
            .put(key, serialized)
            .map_err(|e| LoomError::StorageError(e.to_string()))
    }

    /// Get value by key
    pub fn get<K: AsRef<[u8]>, V: DeserializeOwned>(&self, key: K) -> Result<Option<V>> {
        match self.db.get(key) {
            Ok(Some(data)) => {
                let value = serde_json::from_slice(&data)?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(LoomError::StorageError(e.to_string())),
        }
    }

    /// Delete key
    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<()> {
        self.db
            .delete(key)
            .map_err(|e| LoomError::StorageError(e.to_string()))
    }

    /// Batch put operation
    pub fn batch_put<K: AsRef<[u8]>, V: Serialize>(&self, items: Vec<(K, V)>) -> Result<()> {
        let mut batch = rocksdb::WriteBatch::default();
        for (key, value) in items {
            let serialized = serde_json::to_vec(&value)?;
            batch.put(key, serialized);
        }
        self.db
            .write(batch)
            .map_err(|e| LoomError::StorageError(e.to_string()))
    }
}
