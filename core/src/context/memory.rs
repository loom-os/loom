use super::{MemoryReader, MemoryWriter};
use crate::proto::Event;
use crate::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// A simple in-memory memory store for demo/testing.
/// Stores textual summaries of events keyed by session id.
#[derive(Default)]
pub struct InMemoryMemory {
    // session -> list of lines
    store: DashMap<String, Vec<String>>,
}

impl InMemoryMemory {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: DashMap::new(),
        })
    }

    fn summarize_event(event: &Event) -> String {
        // Minimal summary without parsing payload
        format!(
            "[{ts}] {ty} from {src}",
            ts = event.timestamp_ms,
            ty = event.r#type,
            src = event.source
        )
    }
}

#[async_trait]
impl MemoryWriter for InMemoryMemory {
    async fn append_event(&self, session: &str, event: Event) -> Result<()> {
        let line = Self::summarize_event(&event);
        self.store
            .entry(session.to_string())
            .or_default()
            .value_mut()
            .push(line);
        Ok(())
    }

    async fn summarize_episode(&self, session: &str) -> Result<Option<String>> {
        if let Some(list) = self.store.get(session) {
            let tail = list.iter().rev().take(10).cloned().collect::<Vec<_>>();
            let summary = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl MemoryReader for InMemoryMemory {
    async fn retrieve(
        &self,
        query: &str,
        k: usize,
        _filters: Option<serde_json::Value>,
    ) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for entry in self.store.iter() {
            for line in entry.iter() {
                if line.contains(query) {
                    out.push(line.clone());
                    if out.len() >= k {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }
}
