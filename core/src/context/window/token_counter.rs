//! Token Counting
//!
//! Provides token counting for context window management.

use std::sync::Arc;

/// Token counting interface for different LLM models
pub trait TokenCounter: Send + Sync {
    /// Count tokens in text
    fn count_text(&self, text: &str) -> usize;

    /// Estimate tokens for JSON content (conservative estimate)
    fn count_json(&self, json: &serde_json::Value) -> usize {
        self.count_text(&json.to_string())
    }
}

/// Tiktoken-based counter (placeholder - requires tiktoken crate)
///
/// For now, uses simple character-based estimation:
/// - Average 4 characters per token (common for English)
/// - More conservative for code/JSON (3 chars/token)
pub struct TiktokenCounter {
    model: String,
}

impl TiktokenCounter {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }

    pub fn gpt4() -> Self {
        Self::new("gpt-4")
    }

    pub fn gpt35_turbo() -> Self {
        Self::new("gpt-3.5-turbo")
    }

    fn chars_per_token(&self) -> f32 {
        // Conservative estimates for different content types
        // In production, use tiktoken crate for accurate counting
        if self.model.contains("gpt-4") {
            4.0 // GPT-4 models
        } else {
            3.5 // GPT-3.5 and others
        }
    }
}

impl TokenCounter for TiktokenCounter {
    fn count_text(&self, text: &str) -> usize {
        // Simple estimation: divide char count by chars_per_token
        // Add 10% buffer for special tokens
        let base_estimate = (text.len() as f32 / self.chars_per_token()).ceil() as usize;
        base_estimate + (base_estimate / 10)
    }

    fn count_json(&self, json: &serde_json::Value) -> usize {
        // JSON has more overhead (brackets, quotes, etc.)
        // Use more conservative 3 chars/token estimate
        let json_str = json.to_string();
        let base_estimate = (json_str.len() as f32 / 3.0).ceil() as usize;
        base_estimate + (base_estimate / 10)
    }
}

/// Create a shared token counter for a model
pub fn create_counter(model: &str) -> Arc<dyn TokenCounter> {
    if model.contains("gpt-4") {
        Arc::new(TiktokenCounter::gpt4())
    } else if model.contains("gpt-3.5") {
        Arc::new(TiktokenCounter::gpt35_turbo())
    } else {
        // Default to GPT-4 estimates
        Arc::new(TiktokenCounter::new(model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_counting() {
        let counter = TiktokenCounter::gpt4();

        // Short text
        let text = "Hello, world!";
        let tokens = counter.count_text(text);
        assert!(tokens > 0);
        assert!(tokens < 10); // Should be ~3-4 tokens

        // Longer text
        let long_text = "The quick brown fox jumps over the lazy dog. ".repeat(10);
        let long_tokens = counter.count_text(&long_text);
        assert!(long_tokens > tokens * 5); // Should scale roughly with length
    }

    #[test]
    fn test_json_counting() {
        let counter = TiktokenCounter::gpt4();

        let json = serde_json::json!({
            "type": "message",
            "content": "Hello, world!",
            "metadata": {
                "timestamp": 1234567890,
                "importance": 0.8
            }
        });

        let tokens = counter.count_json(&json);
        assert!(tokens > 0);
        // JSON should have more tokens than just the content
        assert!(tokens > 3);
    }

    #[test]
    fn test_model_variants() {
        let gpt4 = TiktokenCounter::gpt4();
        let gpt35 = TiktokenCounter::gpt35_turbo();

        let text = "The quick brown fox";

        let tokens_4 = gpt4.count_text(text);
        let tokens_35 = gpt35.count_text(text);

        // Both should give reasonable estimates
        assert!(tokens_4 > 0);
        assert!(tokens_35 > 0);
        // GPT-3.5 might have slightly different tokenization
        // but should be in similar range
        assert!((tokens_4 as i32 - tokens_35 as i32).abs() < 3);
    }

    #[test]
    fn test_create_counter() {
        let gpt4 = create_counter("gpt-4-turbo");
        let gpt35 = create_counter("gpt-3.5-turbo");
        let other = create_counter("claude-3");

        let text = "Test message";

        // All should be functional
        assert!(gpt4.count_text(text) > 0);
        assert!(gpt35.count_text(text) > 0);
        assert!(other.count_text(text) > 0);
    }
}
