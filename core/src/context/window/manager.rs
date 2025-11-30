//! Window Management
//!
//! Manages context window budgets and item selection based on token limits.

use crate::context::types::{ContextItem, ContextItemType, MessageRole};
use crate::context::window::token_counter::TokenCounter;
use std::sync::Arc;

/// Configuration for context window management
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Maximum tokens for the entire context window
    pub max_tokens: usize,

    /// Reserve tokens for system prompt
    pub system_reserve: usize,

    /// Reserve tokens for LLM response
    pub response_reserve: usize,

    /// Reserve tokens for current query/request
    pub query_reserve: usize,

    /// Priority allocation: tool results (recent failures are critical)
    pub tool_results_budget: f32, // 0.0-1.0 fraction of available tokens

    /// Priority allocation: recent messages
    pub messages_budget: f32, // 0.0-1.0

    /// Remaining budget for observations and events
    pub observations_budget: f32, // 0.0-1.0
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8000,         // Conservative default (fits most models)
            system_reserve: 500,      // System prompt
            response_reserve: 1000,   // LLM response
            query_reserve: 500,       // Current query
            tool_results_budget: 0.4, // 40% for tool context
            messages_budget: 0.4,     // 40% for conversation
            observations_budget: 0.2, // 20% for events/observations
        }
    }
}

impl WindowConfig {
    /// Available tokens after reserves
    pub fn available_tokens(&self) -> usize {
        self.max_tokens
            .saturating_sub(self.system_reserve)
            .saturating_sub(self.response_reserve)
            .saturating_sub(self.query_reserve)
    }

    /// Get token budget for a specific item type
    pub fn budget_for_type(&self, item_type: &ContextItemType) -> usize {
        let available = self.available_tokens() as f32;

        match item_type {
            ContextItemType::ToolCall { .. } | ContextItemType::ToolResult { .. } => {
                (available * self.tool_results_budget) as usize
            }
            ContextItemType::Message { .. } => (available * self.messages_budget) as usize,
            ContextItemType::Event { .. } | ContextItemType::Observation { .. } => {
                (available * self.observations_budget) as usize
            }
        }
    }
}

/// Result of window selection
#[derive(Debug)]
pub struct WindowSelection {
    /// Items that fit within the window
    pub selected: Vec<ContextItem>,

    /// Total tokens used
    pub tokens_used: usize,

    /// Items that didn't fit (for logging/debugging)
    pub overflow: Vec<ContextItem>,

    /// Token budget that was available
    pub budget: usize,
}

/// Manages context window selection based on token budgets
pub struct WindowManager {
    counter: Arc<dyn TokenCounter>,
    config: WindowConfig,
}

impl WindowManager {
    pub fn new(counter: Arc<dyn TokenCounter>, config: WindowConfig) -> Self {
        Self { counter, config }
    }

    /// Create with default configuration
    pub fn with_counter(counter: Arc<dyn TokenCounter>) -> Self {
        Self::new(counter, WindowConfig::default())
    }

    /// Count tokens for a single item
    pub fn count_item(&self, item: &ContextItem) -> usize {
        // Count main content
        let content_tokens = match &item.content.raw {
            serde_json::Value::String(s) => self.counter.count_text(s),
            json => self.counter.count_json(json),
        };

        // Add text representation
        let text_tokens = self.counter.count_text(&item.content.text);

        // Add metadata overhead (approximate)
        let metadata_tokens = 20; // ~20 tokens for metadata fields

        content_tokens + text_tokens + metadata_tokens
    }

    /// Select items that fit within the configured window
    ///
    /// Items are assumed to be pre-sorted by relevance (from retrieval + ranking).
    /// Selection respects per-type budgets to ensure diverse context.
    pub fn select_items(&self, items: Vec<ContextItem>) -> WindowSelection {
        let total_budget = self.config.available_tokens();
        let mut selected = Vec::new();
        let mut overflow = Vec::new();
        let mut tokens_used = 0;

        // Helper to get budget for item type
        let get_budget = |item_type: &ContextItemType| -> usize {
            match item_type {
                ContextItemType::ToolCall { .. } | ContextItemType::ToolResult { .. } => {
                    (self.config.available_tokens() as f32 * self.config.tool_results_budget)
                        as usize
                }
                ContextItemType::Message { .. } => {
                    (self.config.available_tokens() as f32 * self.config.messages_budget) as usize
                }
                ContextItemType::Event { .. } | ContextItemType::Observation { .. } => {
                    (self.config.available_tokens() as f32 * self.config.observations_budget)
                        as usize
                }
            }
        };

        // Track per-type remaining budgets
        let mut tool_budget = get_budget(&ContextItemType::ToolCall {
            tool_name: String::new(),
        });
        let mut message_budget = get_budget(&ContextItemType::Message {
            role: MessageRole::User,
        });
        let mut observation_budget = get_budget(&ContextItemType::Observation {
            source: String::new(),
        });

        // Select items respecting both total and per-type budgets
        for item in items {
            let item_tokens = self.count_item(&item);

            // Check total budget
            if tokens_used + item_tokens > total_budget {
                overflow.push(item);
                continue;
            }

            // Check per-type budget
            let type_budget = match &item.item_type {
                ContextItemType::ToolCall { .. } | ContextItemType::ToolResult { .. } => {
                    &mut tool_budget
                }
                ContextItemType::Message { .. } => &mut message_budget,
                ContextItemType::Event { .. } | ContextItemType::Observation { .. } => {
                    &mut observation_budget
                }
            };

            if item_tokens > *type_budget {
                overflow.push(item);
                continue;
            }

            // Item fits - add it
            tokens_used += item_tokens;
            *type_budget = type_budget.saturating_sub(item_tokens);
            selected.push(item);
        }

        WindowSelection {
            selected,
            tokens_used,
            overflow,
            budget: total_budget,
        }
    }

    /// Select items with a custom budget (useful for testing or special cases)
    pub fn select_with_budget(&self, items: Vec<ContextItem>, budget: usize) -> WindowSelection {
        let mut selected = Vec::new();
        let mut overflow = Vec::new();
        let mut tokens_used = 0;

        for item in items {
            let item_tokens = self.count_item(&item);

            if tokens_used + item_tokens > budget {
                overflow.push(item);
            } else {
                tokens_used += item_tokens;
                selected.push(item);
            }
        }

        WindowSelection {
            selected,
            tokens_used,
            overflow,
            budget,
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &WindowConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: WindowConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::types::{ContextContent, ContextMetadata, MessageRole};
    use crate::context::window::token_counter::TiktokenCounter;

    fn create_test_item(item_type: ContextItemType, content: &str) -> ContextItem {
        ContextItem {
            id: "test".to_string(),
            item_type,
            content: ContextContent {
                raw: serde_json::json!(content),
                text: content.to_string(),
                token_count: None,
                embedding: None,
            },
            metadata: ContextMetadata::new("test_session".to_string(), "test_agent".to_string()),
        }
    }

    #[test]
    fn test_window_config_budgets() {
        let config = WindowConfig::default();

        let available = config.available_tokens();
        assert!(available > 0);
        assert!(available < config.max_tokens);

        // Check per-type budgets sum to available (approximately)
        let tool_budget = config.budget_for_type(&ContextItemType::ToolResult {
            tool_name: String::new(),
            success: true,
        });
        let msg_budget = config.budget_for_type(&ContextItemType::Message {
            role: MessageRole::User,
        });
        let obs_budget = config.budget_for_type(&ContextItemType::Observation {
            source: String::new(),
        });

        let total_allocated = tool_budget + msg_budget + obs_budget;
        assert!((total_allocated as i32 - available as i32).abs() < 10);
    }

    #[test]
    fn test_count_item() {
        let counter = Arc::new(TiktokenCounter::gpt4());
        let manager = WindowManager::with_counter(counter);

        let item = create_test_item(
            ContextItemType::Message {
                role: MessageRole::User,
            },
            "This is a test message with some content",
        );

        let tokens = manager.count_item(&item);
        assert!(tokens > 0);
        // Should have content + metadata overhead
        assert!(tokens > 10);
    }

    #[test]
    fn test_select_items_within_budget() {
        let counter = Arc::new(TiktokenCounter::gpt4());
        let manager = WindowManager::with_counter(counter);

        let items = vec![
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                "First message",
            ),
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::Assistant,
                },
                "Second message",
            ),
            create_test_item(
                ContextItemType::ToolResult {
                    tool_name: "test_tool".to_string(),
                    success: true,
                },
                "Tool result",
            ),
        ];

        let selection = manager.select_items(items);

        // All should fit with default config
        assert_eq!(selection.selected.len(), 3);
        assert_eq!(selection.overflow.len(), 0);
        assert!(selection.tokens_used > 0);
        assert!(selection.tokens_used < selection.budget);
    }

    #[test]
    fn test_select_items_with_overflow() {
        let counter = Arc::new(TiktokenCounter::gpt4());
        let manager = WindowManager::with_counter(counter);

        // Create very large items
        let large_content = "x".repeat(10000); // Very large message
        let items = vec![
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                &large_content,
            ),
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                &large_content,
            ),
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                &large_content,
            ),
        ];

        let selection = manager.select_items(items);

        // Should have some overflow
        assert!(selection.overflow.len() > 0);
        assert!(selection.tokens_used <= selection.budget);
    }

    #[test]
    fn test_custom_budget() {
        let counter = Arc::new(TiktokenCounter::gpt4());
        let manager = WindowManager::with_counter(counter);

        let items = vec![
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                "Test 1",
            ),
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                "Test 2",
            ),
        ];

        let selection = manager.select_with_budget(items, 50); // Very small budget

        // Should select at least one, might overflow second
        assert!(selection.selected.len() > 0);
        assert!(selection.tokens_used <= 50);
    }

    #[test]
    fn test_per_type_budget_enforcement() {
        let counter = Arc::new(TiktokenCounter::gpt4());
        let mut config = WindowConfig::default();

        // Give messages tiny budget
        config.messages_budget = 0.1; // Only 10% of available
        config.tool_results_budget = 0.5;

        let manager = WindowManager::new(counter, config);

        let content = "x".repeat(1000); // Large content
        let items = vec![
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                &content,
            ),
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                &content,
            ),
            create_test_item(
                ContextItemType::Message {
                    role: MessageRole::User,
                },
                &content,
            ),
            create_test_item(
                ContextItemType::ToolResult {
                    tool_name: "test_tool".to_string(),
                    success: true,
                },
                &content,
            ),
        ];

        let selection = manager.select_items(items);

        // Messages should hit their budget limit faster
        let msg_count = selection
            .selected
            .iter()
            .filter(|i| matches!(i.item_type, ContextItemType::Message { .. }))
            .count();

        let tool_count = selection
            .selected
            .iter()
            .filter(|i| matches!(i.item_type, ContextItemType::ToolResult { .. }))
            .count();

        // Should select fewer messages due to smaller budget
        assert!(msg_count < 3);
        // Tool result should fit in its larger budget
        assert_eq!(tool_count, 1);
    }
}
