# Positioning

Loom is an event-driven AI agent runtime — not a library like LangChain. We provide the infrastructure for long-lifecycle, desktop/edge agents that respond to real-world events.

## One-liner

**Loom is an agent runtime** — long-running services that respond to hotkeys, file changes, and timers, with cross-process collaboration and system integration. LangChain runs scripts; Loom runs agents.

## What it is vs. isn't

**Is:**
- A runtime for long-lifecycle AI agents (hours/days, not seconds)
- An event-driven architecture (hotkeys, file watch, timers, clipboard)
- A cross-process agent communication system (Event Bus)
- A secure tool execution environment (sandboxed)
- A desktop/edge integration layer (system tray, notifications)

**Isn't:**
- A library you import and call (that's LangChain)
- A host operating system
- A one-shot script executor
- A cloud-only solution

## Why different from LangChain/CrewAI

| Aspect | LangChain/CrewAI | Loom |
|--------|------------------|------|
| Nature | **Library** (you call it) | **Runtime** (it runs your agents) |
| Lifecycle | Script execution (seconds) | Service (hours/days) |
| Trigger | Code call only | Events (hotkey, file, timer) |
| Agent communication | In-process function calls | Event Bus (cross-process) |
| Tool safety | None | Sandboxed execution |
| Desktop integration | None | Native (tray, notify, hotkey) |
| Language | Python only | Polyglot (Python, JS, Rust) |

## Architecture: Brain/Hand Separation

```
Python Agent (Brain 🧠)              Rust Core (Hands 🤚)
═══════════════════════              ════════════════════
• LLM calls (direct HTTP)            • Event Bus
• Cognitive Loop (ReAct)             • Tool Registry + Sandbox
• Context Engineering                • Agent Lifecycle
• Business Logic                     • Persistent Store
                                     • System Integration
Fast iteration needed                Stable infrastructure
```

**Why this split?**
- LLM/Cognitive needs rapid experimentation → Python
- Tool execution needs security/performance → Rust
- System integration needs native access → Rust
- Agent logic needs flexibility → Python

## Category name

**Event-Driven Agent Runtime**

## Elevator pitch

> "LangChain is a library for building chatbots. Loom is a runtime for running agents."
>
> Your agent starts when you press a hotkey. It monitors your clipboard. It watches your downloads folder. It remembers conversations from last week. It collaborates with other agents via events. It runs 24/7 as a background service.
>
> That's what Loom does that LangChain can't.

## Target users

1. **Desktop automation** — Personal AI assistant triggered by hotkeys
2. **Trading systems** — Long-running market monitoring agents
3. **Research tools** — Multi-agent collaboration for deep research
4. **Edge deployment** — Agents running on local hardware with privacy

## Key differentiators

1. **Long lifecycle** — Agents are services, not scripts
2. **Event-driven** — React to system events, not just API calls
3. **Desktop-native** — Hotkeys, clipboard, notifications, system tray
4. **Secure execution** — Sandboxed tool execution in Rust
5. **Polyglot** — Write agent logic in Python, JS, or Rust
6. **Observable** — Built-in tracing, metrics, dashboard
