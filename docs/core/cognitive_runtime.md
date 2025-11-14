## Cognitive Runtime & Agent Pattern (Design Draft)

**Status**: Design / draft — this document describes the planned cognitive layer on top of the existing Agent Runtime. It does not reflect a fully implemented feature set yet.

---

### Goals

- **Lift agents from purely reactive callbacks to an explicit perceive–think–act loop** without breaking the existing `AgentRuntime` and `AgentBehavior` abstractions.
- **Keep the core runtime simple**: EventBus + AgentRuntime remain general-purpose infrastructure; cognitive behavior is an opt‑in pattern.
- **Make cognition observable**: planning steps, policy decisions, and memory usage should be inspectable from logs, traces, and the Dashboard.
- **Reuse existing building blocks**: `ContextBuilder`, `MemoryReader/Writer`, `ModelRouter`, `ActionBroker`, and collaboration primitives.

---

### High‑Level Design

The cognitive layer is modeled as a **pattern** and a small set of traits that sit *on top of* the Agent Runtime:

- A `CognitiveLoop` trait describing the perceive/think/act stages.
- A `CognitiveAgent` adapter that implements `AgentBehavior` by delegating to a `CognitiveLoop` implementation.
- Optional helpers for integrating memory, planning, and routing into the loop.

The existing components keep their responsibilities:

- `AgentRuntime` still owns lifecycle, mailboxing, and topic subscriptions.
- `AgentBehavior` remains the core callback interface for non‑cognitive agents.
- The cognitive API is a convenience layer that composes with `AgentBehavior`, not a replacement.

---

### Core Interfaces (proposed)

The core of the design is a **loop trait**:

```rust
/// High-level cognitive loop: perceive events, think, then act.
#[async_trait::async_trait]
pub trait CognitiveLoop: Send + Sync {
    async fn perceive(&mut self, event: crate::Event);
    async fn think(&mut self) -> crate::Result<()>;
    async fn act(&mut self) -> crate::Result<Vec<crate::proto::Action>>;
}
```

This trait is deliberately minimal:

- It does **not** dictate how memory, planning, or tools are wired; those are provided via dependencies passed into the loop implementation.
- Implementations can choose to be strictly single‑event, batched, or maintain their own internal state machine.

A `CognitiveAgent` adapter can then bridge this trait with the existing `AgentBehavior`:

```rust
/// Adapter that lets a CognitiveLoop be used as an AgentBehavior.
pub struct CognitiveAgent<L: CognitiveLoop> {
    loop_impl: L,
}

#[async_trait::async_trait]
impl<L: CognitiveLoop> crate::agent::AgentBehavior for CognitiveAgent<L> {
    async fn on_event(
        &mut self,
        event: crate::Event,
        _state: &mut crate::proto::AgentState,
    ) -> crate::Result<Vec<crate::proto::Action>> {
        self.loop_impl.perceive(event).await;
        self.loop_impl.think().await?;
        self.loop_impl.act().await
    }

    async fn on_init(&mut self, _config: &crate::proto::AgentConfig) -> crate::Result<()> {
        Ok(())
    }

    async fn on_shutdown(&mut self) -> crate::Result<()> {
        Ok(())
    }
}
```

> **Note**: The exact signatures may evolve; the goal is to keep the adapter thin and avoid constraining application‑level cognitive architectures.

---

### Dependencies and Composition

A typical cognitive agent will depend on several core services:

- **Memory**: via `context::MemoryReader` and `MemoryWriter`.
- **Context building**: via `context::builder::ContextBuilder` to assemble LLM‑ready prompts.
- **Planning / LLM calls**: via `llm::LlmClient` and the Tool Orchestrator.
- **Routing**: via `router::ModelRouter` to choose between local/cloud/hybrid inference.
- **Actions & tools**: via `ActionBroker` to invoke capabilities and tools.

The `CognitiveLoop` implementation is free to hold these as fields and orchestrate them internally:

- `perceive` can write to memory (append events, update episodic context).
- `think` can run planning logic (LLM prompts, rule‑based policies, tool suggestions).
- `act` can emit actions (including downstream events, tool calls, or replies).

---

### Example: Planner‑Style Cognitive Agent (conceptual)

This is an **illustrative** flow, not a fixed API:

1. **Perceive**
   - Receive an event on a thread topic.
   - Append a textual summary to episodic memory via `MemoryWriter`.
2. **Think**
   - Use `ContextBuilder` with the current thread/session id and goal string to build a `PromptBundle`.
   - Call `LlmClient` with the bundle, optionally enabling tools.
   - Parse the model output into a simple plan or next action description.
3. **Act**
   - Translate the plan into concrete `Action` values (tool calls, emit/reply, etc.).
   - Optionally update internal state to remember progress on multi‑step plans.

Over time, more sophisticated patterns (hierarchical planners, policy graphs, etc.) can be layered on the same interface.

---

### Observability

To make cognition debuggable and observable:

- Emit tracing spans for `perceive`, `think`, and `act`, including:
  - Thread id / correlation id (from `Envelope`).
  - Planning decisions (chosen tools, routes, or policies).
  - Memory usage (which summaries or retrieved items were used).
- Integrate with the Dashboard:
  - Mark cognitive agents in the topology graph.
  - Surface “current phase” or recent plan steps in the agent detail view (future work).

These hooks will be added incrementally as the cognitive pattern solidifies.

---

### Rollout Plan

1. **P1 (experimental)**
   - Land the `CognitiveLoop` trait and `CognitiveAgent` adapter in `core`.
   - Provide a minimal example cognitive agent (e.g., planner/orchestrator) for demos.
   - Document the pattern and constraints here and in the main `ROADMAP.md`.
2. **P2+**
   - Add richer helper utilities (plan serialization, policy configuration).
   - Tighten observability integration (Dashboard, OpenTelemetry spans and metrics).
   - Explore higher‑level “cognitive templates” for common agent roles.
