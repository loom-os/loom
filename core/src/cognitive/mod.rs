//! Cognitive Module - LLM-Powered Intelligent Agent System
//!
//! This module provides the complete cognitive architecture for LLM-powered agents:
//!
//! - **LLM**: HTTP client, model routing, and provider abstraction
//! - **Loop**: Perceive-Think-Act cognitive loop pattern
//! - **Orchestrator**: Tool execution and multi-step reasoning
//!
//! # Architecture
//!
//! ```text
//!                     ┌─────────────────────────────────────────────────┐
//!                     │               COGNITIVE LOOP                     │
//!                     │                                                  │
//!    Event ──────▶   │  ┌──────────┐   ┌──────────┐   ┌──────────┐     │
//!                     │  │ PERCEIVE │──▶│  THINK   │──▶│   ACT    │──────────▶ Actions
//!                     │  │          │   │          │   │          │     │
//!                     │  │ Context  │   │ LLM +    │   │ Execute  │     │
//!                     │  │ Pipeline │   │ Router   │   │ Tools    │     │
//!                     │  └──────────┘   └──────────┘   └──────────┘     │
//!                     │        │              │              │          │
//!                     │        ▼              ▼              ▼          │
//!                     │  ┌──────────────────────────────────────────┐   │
//!                     │  │            CONTEXT PIPELINE               │   │
//!                     │  └──────────────────────────────────────────┘   │
//!                     └─────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use loom_core::cognitive::{CognitiveLoop, CognitiveAgent, CognitiveConfig};
//! use loom_core::cognitive::llm::{LlmClient, ModelRouter};
//!
//! // Create a cognitive loop implementation
//! let loop_impl = SimpleCognitiveLoop::new(config, llm_client, tool_registry);
//!
//! // Wrap it as an AgentBehavior
//! let behavior = CognitiveAgent::new(loop_impl);
//!
//! // Use with AgentRuntime as usual
//! runtime.create_agent(agent_config, Box::new(behavior)).await?;
//! ```

// LLM subsystem (client, router, providers)
pub mod llm;

// Cognitive loop components
mod agent_adapter;
mod config;
mod loop_trait;
mod memory_buffer;
mod simple_loop;
mod thought;

// Core cognitive types
pub use agent_adapter::CognitiveAgent;
pub use config::{CognitiveConfig, ThinkingStrategy};
pub use loop_trait::{CognitiveLoop, ExecutionResult, Perception};
pub use memory_buffer::{MemoryBuffer, MemoryItem, MemoryItemType};
pub use simple_loop::SimpleCognitiveLoop;
pub use thought::{Observation, Plan, Thought, ThoughtStep, ToolCall};

// Re-export key LLM types for convenience
pub use llm::router::{ModelRouter, Route, RoutingDecision};
pub use llm::{LlmClient, LlmClientConfig, LlmResponse};
