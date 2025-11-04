//! Agent runtime module split into smaller files for readability.
//! - behavior.rs: AgentBehavior trait
//! - instance.rs: Agent struct and core logic (routing, execution)
//! - runtime.rs: AgentRuntime manager

// Publicly re-export common types expected by external users
pub use crate::proto::{Action, AgentConfig, AgentState};

mod behavior;
mod instance;
mod runtime;

// Public re-exports so external code keeps using crate::agent::{Agent, AgentRuntime}
pub use behavior::AgentBehavior;
pub use instance::Agent;
pub use runtime::AgentRuntime;
