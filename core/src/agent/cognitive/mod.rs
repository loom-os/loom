//! Cognitive Loop Module
//!
//! This module provides a structured perceive-think-act cognitive loop pattern
//! for building intelligent agents. It sits on top of the existing `AgentRuntime`
//! and `AgentBehavior` abstractions, providing an opt-in cognitive architecture.
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
//!                     │  │ Builder  │   │ Planning │   │ Tools    │     │
//!                     │  └──────────┘   └──────────┘   └──────────┘     │
//!                     │        │              │              │          │
//!                     │        ▼              ▼              ▼          │
//!                     │  ┌──────────────────────────────────────────┐   │
//!                     │  │              WORKING MEMORY               │   │
//!                     │  └──────────────────────────────────────────┘   │
//!                     └─────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use loom_core::agent::cognitive::{CognitiveLoop, CognitiveAgent, CognitiveConfig};
//!
//! // Create a cognitive loop implementation
//! let loop_impl = SimpleCognitiveLoop::new(config, llm_client, action_broker);
//!
//! // Wrap it as an AgentBehavior
//! let behavior = CognitiveAgent::new(loop_impl);
//!
//! // Use with AgentRuntime as usual
//! runtime.create_agent(agent_config, Box::new(behavior)).await?;
//! ```

mod agent_adapter;
mod config;
mod loop_trait;
mod simple_loop;
mod thought;
mod working_memory;

// Core types
pub use agent_adapter::CognitiveAgent;
pub use config::{CognitiveConfig, ThinkingStrategy};
pub use loop_trait::{CognitiveLoop, ExecutionResult, Perception};
pub use simple_loop::SimpleCognitiveLoop;
pub use thought::{Observation, Plan, Thought, ThoughtStep, ToolCall};
pub use working_memory::{MemoryItem, MemoryItemType, WorkingMemory};
