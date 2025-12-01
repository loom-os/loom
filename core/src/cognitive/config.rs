//! Configuration for cognitive agents.

use serde::{Deserialize, Serialize};

/// Strategy for the thinking phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThinkingStrategy {
    /// Simple single-shot LLM call
    #[default]
    SingleShot,
    /// ReAct pattern: Reason → Act → Observe → Repeat
    ReAct,
    /// Chain of Thought with explicit reasoning steps
    ChainOfThought,
}

/// Configuration for a cognitive agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveConfig {
    /// Maximum number of think-act iterations (for ReAct pattern)
    pub max_iterations: usize,

    /// Whether to enable reflection after acting
    pub enable_reflection: bool,

    /// Maximum items to keep in working memory
    pub memory_window_size: usize,

    /// Thinking strategy to use
    pub thinking_strategy: ThinkingStrategy,

    /// Timeout for each tool invocation in milliseconds
    pub tool_timeout_ms: u64,

    /// Whether to refine the answer after tool use
    pub refine_after_tools: bool,

    /// Maximum number of tools to expose to the LLM
    pub max_tools_exposed: usize,

    /// System prompt template
    pub system_prompt: Option<String>,

    /// Temperature for LLM calls
    pub temperature: Option<f32>,
}

impl Default for CognitiveConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            enable_reflection: false,
            memory_window_size: 20,
            thinking_strategy: ThinkingStrategy::default(),
            tool_timeout_ms: 30_000,
            refine_after_tools: true,
            max_tools_exposed: 32,
            system_prompt: None,
            temperature: None,
        }
    }
}

impl CognitiveConfig {
    /// Create a new config with ReAct strategy
    pub fn react() -> Self {
        Self {
            thinking_strategy: ThinkingStrategy::ReAct,
            max_iterations: 5,
            ..Default::default()
        }
    }

    /// Create a new config with single-shot strategy
    pub fn single_shot() -> Self {
        Self {
            thinking_strategy: ThinkingStrategy::SingleShot,
            max_iterations: 1,
            ..Default::default()
        }
    }

    /// Create a new config with chain-of-thought strategy
    pub fn chain_of_thought() -> Self {
        Self {
            thinking_strategy: ThinkingStrategy::ChainOfThought,
            max_iterations: 3,
            ..Default::default()
        }
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the max iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Enable reflection
    pub fn with_reflection(mut self) -> Self {
        self.enable_reflection = true;
        self
    }

    /// Set memory window size
    pub fn with_memory_window(mut self, size: usize) -> Self {
        self.memory_window_size = size;
        self
    }
}
