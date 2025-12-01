//! Ranking strategies for context items.
//!
//! After retrieval, items need to be ranked to determine which ones should be
//! included in the final context window. Different ranking strategies can be
//! combined for sophisticated selection.

use crate::context::retrieval::strategy::RetrievalTrigger;
use crate::context::types::ContextItem;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

/// Strategy for ranking retrieved context items.
#[async_trait]
pub trait ContextRanker: Send + Sync {
    /// Rank items, returning them in order from most to least relevant.
    async fn rank(
        &self,
        items: Vec<ContextItem>,
        trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>>;

    /// Get a human-readable name for this ranker
    fn name(&self) -> &str;
}

/// Ranks items by timestamp (newest first).
///
/// This is the default ranking strategy - most recent items are most relevant.
pub struct TemporalRanker {
    /// Whether to sort newest first (true) or oldest first (false)
    pub newest_first: bool,
}

impl TemporalRanker {
    pub fn new(newest_first: bool) -> Arc<Self> {
        Arc::new(Self { newest_first })
    }

    pub fn newest_first() -> Arc<Self> {
        Self::new(true)
    }

    pub fn oldest_first() -> Arc<Self> {
        Self::new(false)
    }
}

#[async_trait]
impl ContextRanker for TemporalRanker {
    async fn rank(
        &self,
        mut items: Vec<ContextItem>,
        _trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>> {
        debug!(
            ranker = self.name(),
            count = items.len(),
            newest_first = self.newest_first,
            "Ranking items temporally"
        );

        items.sort_by(|a, b| {
            if self.newest_first {
                b.metadata.timestamp_ms.cmp(&a.metadata.timestamp_ms)
            } else {
                a.metadata.timestamp_ms.cmp(&b.metadata.timestamp_ms)
            }
        });

        Ok(items)
    }

    fn name(&self) -> &str {
        if self.newest_first {
            "TemporalRanker(newest_first)"
        } else {
            "TemporalRanker(oldest_first)"
        }
    }
}

/// Ranks items by importance score (highest first).
pub struct ImportanceRanker;

impl ImportanceRanker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl Default for ImportanceRanker {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl ContextRanker for ImportanceRanker {
    async fn rank(
        &self,
        mut items: Vec<ContextItem>,
        _trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>> {
        debug!(
            ranker = self.name(),
            count = items.len(),
            "Ranking items by importance"
        );

        items.sort_by(|a, b| {
            b.metadata
                .importance
                .partial_cmp(&a.metadata.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(items)
    }

    fn name(&self) -> &str {
        "ImportanceRanker"
    }
}

/// Composite ranker that combines multiple ranking strategies.
///
/// Each strategy is applied with a weight, and final scores are computed
/// as a weighted sum. Items are then sorted by final score.
pub struct CompositeRanker {
    /// (ranker, weight) pairs
    rankers: Vec<(Arc<dyn ContextRanker>, f32)>,
}

impl CompositeRanker {
    pub fn new(rankers: Vec<(Arc<dyn ContextRanker>, f32)>) -> Arc<Self> {
        Arc::new(Self { rankers })
    }

    /// Normalize weights to sum to 1.0
    fn normalize_weights(weights: &[f32]) -> Vec<f32> {
        let sum: f32 = weights.iter().sum();
        if sum > 0.0 {
            weights.iter().map(|w| w / sum).collect()
        } else {
            vec![1.0 / weights.len() as f32; weights.len()]
        }
    }

    /// Compute score for an item based on its position in a ranked list
    fn position_score(position: usize, total: usize) -> f32 {
        if total == 0 {
            return 0.0;
        }
        1.0 - (position as f32 / total as f32)
    }
}

#[async_trait]
impl ContextRanker for CompositeRanker {
    async fn rank(
        &self,
        items: Vec<ContextItem>,
        trigger: &RetrievalTrigger,
    ) -> Result<Vec<ContextItem>> {
        debug!(
            ranker = self.name(),
            count = items.len(),
            num_rankers = self.rankers.len(),
            "Ranking items with composite strategy"
        );

        if items.is_empty() {
            return Ok(items);
        }

        // Get item IDs for tracking
        let _item_ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();

        // Collect rankings from each ranker
        let mut rankings = Vec::new();
        for (ranker, _weight) in &self.rankers {
            let ranked = ranker.rank(items.clone(), trigger).await?;
            rankings.push(ranked);
        }

        // Normalize weights
        let weights: Vec<f32> = self.rankers.iter().map(|(_, w)| *w).collect();
        let norm_weights = Self::normalize_weights(&weights);

        // Compute composite scores
        let mut scores: std::collections::HashMap<String, f32> = std::collections::HashMap::new();

        for (ranking, weight) in rankings.iter().zip(norm_weights.iter()) {
            for (pos, item) in ranking.iter().enumerate() {
                let score = Self::position_score(pos, ranking.len()) * weight;
                *scores.entry(item.id.clone()).or_insert(0.0) += score;
            }
        }

        // Sort items by composite score
        let mut scored_items: Vec<(ContextItem, f32)> = items
            .into_iter()
            .map(|item| {
                let score = scores.get(&item.id).copied().unwrap_or(0.0);
                (item, score)
            })
            .collect();

        scored_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored_items.into_iter().map(|(item, _)| item).collect())
    }

    fn name(&self) -> &str {
        "CompositeRanker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::{ContextContent, ContextItemType, ContextMetadata, MessageRole};

    fn create_test_items(count: usize) -> Vec<ContextItem> {
        (0..count)
            .map(|i| {
                // Alternate importance scores
                let importance = if i % 2 == 0 { 0.8 } else { 0.3 };

                ContextItem::new(
                    ContextItemType::Message {
                        role: MessageRole::User,
                    },
                    ContextContent::from_string(format!("Message {}", i)),
                    ContextMetadata::new("s1".to_string(), "a1".to_string())
                        .with_importance(importance),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn test_temporal_ranker_newest_first() {
        let items = create_test_items(5);
        let ranker = TemporalRanker::newest_first();

        let trigger = RetrievalTrigger::new("s1".to_string(), "a1".to_string());

        let ranked = ranker.rank(items, &trigger).await.unwrap();

        // Check that items are sorted newest first
        for i in 1..ranked.len() {
            assert!(ranked[i - 1].metadata.timestamp_ms >= ranked[i].metadata.timestamp_ms);
        }
    }

    #[tokio::test]
    async fn test_temporal_ranker_oldest_first() {
        let items = create_test_items(5);
        let ranker = TemporalRanker::oldest_first();

        let trigger = RetrievalTrigger::new("s1".to_string(), "a1".to_string());

        let ranked = ranker.rank(items, &trigger).await.unwrap();

        // Check that items are sorted oldest first
        for i in 1..ranked.len() {
            assert!(ranked[i - 1].metadata.timestamp_ms <= ranked[i].metadata.timestamp_ms);
        }
    }

    #[tokio::test]
    async fn test_importance_ranker() {
        let items = create_test_items(6); // 0.8, 0.3, 0.8, 0.3, 0.8, 0.3
        let ranker = ImportanceRanker::new();

        let trigger = RetrievalTrigger::new("s1".to_string(), "a1".to_string());

        let ranked = ranker.rank(items, &trigger).await.unwrap();

        // Check that items are sorted by importance (highest first)
        for i in 1..ranked.len() {
            assert!(
                ranked[i - 1].metadata.importance >= ranked[i].metadata.importance,
                "Item {} importance {} should be >= item {} importance {}",
                i - 1,
                ranked[i - 1].metadata.importance,
                i,
                ranked[i].metadata.importance
            );
        }

        // First item should have high importance
        assert!(ranked[0].metadata.importance >= 0.8);
    }

    #[tokio::test]
    async fn test_composite_ranker() {
        let items = create_test_items(5);

        let temporal = TemporalRanker::newest_first();
        let importance = ImportanceRanker::new();

        let ranker = CompositeRanker::new(vec![
            (temporal as Arc<dyn ContextRanker>, 0.5),
            (importance as Arc<dyn ContextRanker>, 0.5),
        ]);

        let trigger = RetrievalTrigger::new("s1".to_string(), "a1".to_string());

        let ranked = ranker.rank(items, &trigger).await.unwrap();

        // Should have all items
        assert_eq!(ranked.len(), 5);

        // Result should be a blend of temporal and importance ranking
        // Hard to test exact order, but verify no duplicates
        let ids: Vec<String> = ranked.iter().map(|i| i.id.clone()).collect();
        let unique_ids: std::collections::HashSet<String> = ids.iter().cloned().collect();
        assert_eq!(ids.len(), unique_ids.len(), "No duplicate items");
    }
}
