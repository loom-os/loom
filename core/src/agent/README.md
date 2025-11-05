# Agent Module

Stateful, event-driven agents with routing and action execution.

## Overview

This module defines the agent runtime building blocks:

- AgentBehavior: your business logic (async trait)
- Agent: a running actor that receives events, applies routing, and executes actions
- AgentRuntime: a manager that creates/starts/stops Agents and wires subscriptions

It integrates with the EventBus, ModelRouter, and ActionBroker.

## Key components

### AgentBehavior

```rust
#[async_trait]
pub trait AgentBehavior: Send + Sync {
    async fn on_event(&mut self, event: Event, state: &mut AgentState) -> Result<Vec<Action>>;
    async fn on_init(&mut self, config: &AgentConfig) -> Result<()>;
    async fn on_shutdown(&mut self) -> Result<()>;
}
```

- Return a list of Actions to be executed by the Agent via ActionBroker.
- `state` carries persistent_state, ephemeral_context, metadata, and last_update_ms.

### Agent

- Owns config, state, behavior, an event mailbox, ActionBroker, EventBus, and a ModelRouter.
- Event loop:
  1. Route event via ModelRouter with an AgentContext snapshot
  2. Call behavior.on_event with route annotations in event.metadata
  3. Execute returned actions (QoS derived from action.priority)
  4. Publish observability events

Routing modes:

- Local / LocalFallback: single local pass
- Cloud: single cloud pass (metadata: routing_target=cloud)
- Hybrid: local quick then cloud refine (metadata: routing_target, phase, refine=true)
- Defer / Drop: no-op

Observability events:

- `routing_decision` on `agent.{id}` with route, reason, confidence, estimates
- `action_result` on `agent.{id}` with action_type and status, payload is action output

### AgentRuntime

- Holds a map of running agents (JoinHandles)
- Subscribes to configured topics and forwards events into each agent’s mailbox
- `create_agent(config, behavior)` returns an `agent_id`
- `delete_agent(agent_id)` aborts the task and removes it

## Routing policy overrides

`Agent.config.parameters` can override router policy:

- routing.privacy = public | sensitive | private | local-only
- routing.latency_budget_ms = u64
- routing.cost_cap = f32
- routing.quality_threshold = f32

These are logged with each routing decision for transparency.

## QoS mapping for actions

- priority >= 70 → Realtime
- 30 <= priority < 70 → Batched
- else → Background

## Minimal usage example

```rust
use loom_core::agent::{AgentRuntime, AgentBehavior};
use loom_core::{EventBus, Result};
use loom_core::proto::{AgentConfig, AgentState, Action};
use async_trait::async_trait;
use std::sync::Arc;

struct EchoBehavior;
#[async_trait]
impl AgentBehavior for EchoBehavior {
    async fn on_event(&mut self, _event: loom_core::Event, _state: &mut AgentState) -> Result<Vec<Action>> {
        Ok(vec![]) // no actions
    }
    async fn on_init(&mut self, _config: &AgentConfig) -> Result<()> { Ok(()) }
    async fn on_shutdown(&mut self) -> Result<()> { Ok(()) }
}

# async fn run() -> Result<()> {
let bus = Arc::new(EventBus::new());
let broker = Arc::new(loom_core::action_broker::ActionBroker::new());
let router = loom_core::router::ModelRouter::default();
let mut rt = AgentRuntime::new(Arc::clone(&bus), Arc::clone(&broker), router).await?;
rt.start().await?;

let cfg = AgentConfig { agent_id: "agent1".into(), subscribed_topics: vec!["mic.primary.speech".into()], parameters: Default::default() };
let _id = rt.create_agent(cfg, Box::new(EchoBehavior)).await?;
# Ok(()) }
```

## Notes

- Behavior code can publish actions like `llm.generate` and `tts.speak`—the ActionBroker resolves the provider and returns output which is emitted as an `action_result` event.
- For the P0 voice demo, the Agent typically subscribes to `transcript.final` and orchestrates LLM→TTS when the wake word is active.
