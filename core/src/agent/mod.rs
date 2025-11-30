//! Agent runtime module split into smaller files for readability.
//!
//! This module provides:
//! - `AgentBehavior`: Trait for implementing agent logic
//! - `Agent`: Running agent instance with event loop
//! - `AgentRuntime`: Manager for agent lifecycle
//! - `cognitive`: Perceive-Think-Act cognitive loop pattern (opt-in)
//!
//! # Basic Agent
//!
//! For simple event-driven agents, implement `AgentBehavior`:
//!
//! ```rust,ignore
//! use loom_core::agent::AgentBehavior;
//!
//! struct MyAgent;
//!
//! #[async_trait]
//! impl AgentBehavior for MyAgent {
//!     async fn on_event(&mut self, event: Event, state: &mut AgentState) -> Result<Vec<Action>> {
//!         // Handle event
//!         Ok(vec![])
//!     }
//!     // ...
//! }
//! ```
//!
//! # Cognitive Agent
//!
//! For agents with LLM-powered reasoning, use the cognitive module:
//!
//! ```rust,ignore
//! use loom_core::agent::cognitive::{SimpleCognitiveLoop, CognitiveAgent, CognitiveConfig};
//!
//! let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);
//! let behavior = CognitiveAgent::new(loop_impl);
//! runtime.create_agent(agent_config, Box::new(behavior)).await?;
//! ```

// Publicly re-export common types expected by external users
pub use crate::proto::{Action, AgentConfig, AgentState};

mod behavior;
pub mod cognitive;
mod instance;
mod runtime;

// Public re-exports so external code keeps using crate::agent::{Agent, AgentRuntime}
pub use behavior::AgentBehavior;
pub use instance::Agent;
pub use runtime::AgentRuntime;
