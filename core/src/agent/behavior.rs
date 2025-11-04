use async_trait::async_trait;

use crate::{Event, Result};
use crate::proto::{Action, AgentConfig, AgentState};

/// Agent behavior trait
#[async_trait]
pub trait AgentBehavior: Send + Sync {
    async fn on_event(&mut self, event: Event, state: &mut AgentState) -> Result<Vec<Action>>;
    async fn on_init(&mut self, config: &AgentConfig) -> Result<()>;
    async fn on_shutdown(&mut self) -> Result<()>;
}
