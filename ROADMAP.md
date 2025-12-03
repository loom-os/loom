# Loom Roadmap

**Vision**: Build an event-driven AI agent runtime that enables long-lifecycle, desktop/edge agents with proper context engineering and observable reasoning.

**Key Insight**: Loom is a **Runtime**, not a library. The differentiation from LangChain/CrewAI is:

- Long-running agents (not script execution)
- Event-driven triggers (not code calls)
- System integration (hotkeys, files, clipboard)
- Cross-process agent collaboration

---

## Architecture Principles

### Brain/Hand Separation

```
┌─────────────────────────────────────────────────────────────────────┐
│  Python (Brain 🧠)                  Rust Core (Hands 🤚)            │
│  ════════════════                   ══════════════════              │
│  • LLM calls (direct HTTP)          • Event Bus                     │
│  • Cognitive Loop (ReAct/CoT)       • Tool Registry + Sandbox       │
│  • Context Engineering              • Agent Lifecycle               │
│  • Memory strategies                • Persistent Store              │
│  • Business logic                   • System Integration            │
│                                     • MCP Proxy                     │
│  Fast iteration needed              Stable infrastructure           │
└─────────────────────────────────────────────────────────────────────┘
```

### Responsibility Matrix

| Component           | Rust Core    | Python SDK       | Agent Code |
| ------------------- | ------------ | ---------------- | ---------- |
| Event Bus           | ✅           | -                | -          |
| Tool Execution      | ✅ (sandbox) | -                | -          |
| Agent Lifecycle     | ✅           | -                | -          |
| Persistent Store    | ✅           | -                | -          |
| MCP Proxy           | ✅           | -                | -          |
| LLM Calls           | ❌           | ✅ (direct HTTP) | -          |
| Cognitive Loop      | ❌           | ✅               | -          |
| Context Engineering | ❌           | ✅               | -          |
| Business Logic      | -            | -                | ✅         |

---

## Demo Progression

```
Demo 1: Chat Agent (MVP)              ✅ Working
    ↓ validates: brain/hand separation, direct LLM calls

Demo 2: DeepResearch                  🚧 In Progress
    ↓ unlocks: multi-agent, context isolation, report generation

Demo 3: Market Analyst                📋 Planned
    ↓ unlocks: long lifecycle, proactive agents, memory tiers

Demo 4: Desktop Assistant             📋 Planned
    ↓ unlocks: hotkeys, clipboard, system integration
```

---

## Phase 1: Foundation Refactor (Current)

**Objective**: Establish clean brain/hand separation. Python owns cognition, Rust owns execution.

### ✅ Completed

- [x] Python `LLMProvider` direct HTTP calls (bypass Rust `llm:generate`)
- [x] Chat Agent demo working with new architecture
- [x] `loom.toml` configuration for LLM providers

### 🚧 In Progress

**1.1 Python SDK Refactor (loom-py)**

- [ ] Context Engineering module
  - [ ] `ContextBuilder` - assemble prompts from memory
  - [ ] `TokenBudget` - manage context window limits
  - [ ] `MemoryStore` - in-memory conversation history
- [ ] Cognitive Loop improvements
  - [ ] Better ReAct parsing
  - [ ] Configurable tool schemas
  - [ ] Step-by-step streaming

**1.2 Rust Core Cleanup**

- [ ] Deprecate `cognitive/llm/` module (keep for Rust-native agents only)
- [ ] Ensure `llm:generate` tool still works for backward compat
- [ ] Document that Python agents should use direct HTTP

**1.3 Documentation**

- [x] Update ARCHITECTURE.md with brain/hand model
- [x] Update ROADMAP.md with new direction
- [ ] Python SDK guide for cognitive agents

---

## Phase 2: DeepResearch Demo (2-3 weeks)

**Objective**: Multi-agent research system with context isolation.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Lead Agent                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Cognitive Loop (Python)                                │   │
│  │  • Decompose query into sub-tasks                       │   │
│  │  • Spawn researcher agents via Event Bus                │   │
│  │  • Aggregate results into final report                  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│              ┌───────────────┼───────────────┐                 │
│              ▼               ▼               ▼                 │
│         Researcher 1    Researcher 2    Researcher 3           │
│         (isolated ctx)  (isolated ctx)  (isolated ctx)         │
│              │               │               │                 │
│              └───────────────┴───────────────┘                 │
│                              │                                  │
│                              ▼                                  │
│                      Final Report (MD)                          │
└─────────────────────────────────────────────────────────────────┘
```

### Tasks

**2.1 Multi-Agent Communication**

- [ ] Agent spawning via events (`research.spawn`)
- [ ] Result collection via events (`research.result`)
- [ ] Context isolation per agent (no cross-contamination)

**2.2 Tool Integration**

- [ ] Web search tool (Brave Search MCP)
- [ ] File system tools (`fs:write` for reports)
- [ ] Citation extraction and formatting

**2.3 Report Generation**

- [ ] Markdown report structure
- [ ] Source deduplication
- [ ] Table of contents generation

**Acceptance Criteria**:

- ✅ User asks "What are the latest AI agent frameworks?"
- ✅ Lead spawns 3 researchers with different sub-queries
- ✅ Each researcher has isolated context
- ✅ Final report written to `workspace/reports/`
- ✅ Full traces visible in dashboard

---

## Phase 3: Market Analyst Demo (3-4 weeks)

**Objective**: Long-lifecycle trading system with proactive monitoring.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Market Analyst System                         │
│                    (runs 24/7)                                   │
│                                                                  │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐│
│   │ Data Agent  │  │ Sentiment   │  │ Lead Agent              ││
│   │             │  │ Agent       │  │                         ││
│   │ • Price     │  │ • News      │  │ • Decision making       ││
│   │   monitoring│  │   scraping  │  │ • Trading execution     ││
│   │ • Alerts    │  │ • Analysis  │  │ • Risk management       ││
│   └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘│
│          │                │                      │              │
│          └────────────────┴──────────────────────┘              │
│                           │                                      │
│                    Event Bus (Rust Core)                         │
│                           │                                      │
│                    Persistent Memory                             │
└─────────────────────────────────────────────────────────────────┘
```

### Tasks

**3.1 Long Lifecycle Support**

- [ ] Agent auto-restart on crash
- [ ] State persistence across restarts
- [ ] Graceful shutdown handling

**3.2 Memory Tiers**

- [ ] Working memory (current task)
- [ ] Short-term memory (session, 1 hour)
- [ ] Long-term memory (persistent, RocksDB)

**3.3 Proactive Agents**

- [ ] Scheduled triggers (every N minutes)
- [ ] Threshold-based alerts
- [ ] Background monitoring

**3.4 Trading Integration**

- [ ] OKX API integration
- [ ] Order execution tool
- [ ] Position tracking

**Acceptance Criteria**:

- ✅ System runs 1+ hour continuously
- ✅ Memory persists across agent restarts
- ✅ Periodic reports generated automatically
- ✅ Trading decisions logged and traceable

---

## Phase 4: Desktop Assistant Demo (4 weeks)

**Objective**: Personal assistant with system integration.

### Unique Capabilities

```
┌─────────────────────────────────────────────────────────────────┐
│                    Desktop Assistant                             │
│                                                                  │
│   Triggers:                        Actions:                      │
│   ┌─────────────┐                 ┌─────────────┐               │
│   │ 🔥 Hotkey   │ ──────────────▶ │ 💬 Chat     │               │
│   │ (Cmd+L)     │                 │             │               │
│   └─────────────┘                 └─────────────┘               │
│   ┌─────────────┐                 ┌─────────────┐               │
│   │ 📋 Clipboard│ ──────────────▶ │ 📝 Summarize│               │
│   │ (copy text) │                 │             │               │
│   └─────────────┘                 └─────────────┘               │
│   ┌─────────────┐                 ┌─────────────┐               │
│   │ 📁 File     │ ──────────────▶ │ 🗂️ Organize │               │
│   │ (download)  │                 │             │               │
│   └─────────────┘                 └─────────────┘               │
│   ┌─────────────┐                 ┌─────────────┐               │
│   │ ⏰ Schedule │ ──────────────▶ │ 🔔 Notify   │               │
│   │ (timer)     │                 │             │               │
│   └─────────────┘                 └─────────────┘               │
└─────────────────────────────────────────────────────────────────┘
```

### Tasks

**4.1 System Integration (Rust Core)**

- [ ] Global hotkey registration
- [ ] Clipboard monitoring
- [ ] File system watching
- [ ] System notifications
- [ ] System tray icon

**4.2 Voice Integration (loom-audio)**

- [ ] Wake word detection
- [ ] Speech-to-text
- [ ] Text-to-speech response

**Acceptance Criteria**:

- ✅ Press Cmd+Shift+L → Agent responds to query
- ✅ Copy text → Agent offers to summarize
- ✅ File downloaded → Agent suggests organization
- ✅ Voice activation works

---

## Phase 5: Architecture Cleanup (Ongoing)

### loom-dashboard Extraction

- [ ] Extract dashboard from `core/src/dashboard/` to `loom-dashboard/`
- [ ] Standalone deployment option
- [ ] WebSocket-based real-time updates

### Rust Core Cleanup

- [ ] Remove/deprecate `cognitive/llm/` (or mark as Rust-agent-only)
- [ ] Clean up `context/` module (keep storage, remove Python-competing parts)
- [ ] Improve MCP client robustness

### Python SDK Improvements

- [ ] Full Context Engineering module
- [ ] Streaming LLM responses
- [ ] Better error messages
- [ ] Type hints throughout
- [ ] `pip install loom` ready

---

## Timeline Summary

| Phase   | Duration  | Deliverable            |
| ------- | --------- | ---------------------- |
| Phase 1 | 1 week    | Foundation refactor ✅ |
| Phase 2 | 2-3 weeks | DeepResearch demo      |
| Phase 3 | 3-4 weeks | Market Analyst demo    |
| Phase 4 | 4 weeks   | Desktop Assistant demo |
| Phase 5 | Ongoing   | Architecture cleanup   |

**Total**: ~12 weeks to Desktop Assistant

---

## Success Metrics

### Loom vs LangChain Differentiation

| Metric              | LangChain        | Loom Target                       |
| ------------------- | ---------------- | --------------------------------- |
| Agent lifecycle     | Script (seconds) | **Service (hours/days)**          |
| Trigger types       | Code only        | **Events (hotkey, file, timer)**  |
| Agent communication | In-process       | **Event Bus (cross-process)**     |
| Desktop integration | None             | **Native (tray, notify, hotkey)** |
| Tool safety         | None             | **Sandbox**                       |
| Cold start          | N/A              | **< 100ms**                       |
| Memory footprint    | N/A              | **< 50MB (Rust runtime)**         |

---

_Last updated: 2024-12-03_
