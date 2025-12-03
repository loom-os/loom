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

## App Progression

```
App 1: Chat Assistant (MVP)           ✅ Working
    ↓ validates: brain/hand separation, direct LLM calls, tool use

App 2: Chat Assistant + Research      🚧 In Progress
    ↓ enhances: workspace, file system, agent spawning

App 3: Market Analyst                 📋 Planned
    ↓ unlocks: long lifecycle, proactive agents, memory tiers

App 4: Desktop Assistant              📋 Planned
    ↓ unlocks: hotkeys, clipboard, system integration
```

---

## Phase 1: Foundation ✅ Complete

**Objective**: Establish clean brain/hand separation. Python owns cognition, Rust owns execution.

### ✅ Completed

- [x] Python `LLMProvider` direct HTTP calls (bypass Rust `llm:generate`)
- [x] Chat Assistant app working with new architecture
- [x] `loom.toml` configuration for LLM providers
- [x] Cognitive Loop with ReAct pattern
- [x] Tool calling via Rust Bridge (weather, shell, fs:read_file)
- [x] Multi-turn conversation with memory
- [x] Streaming support (`run_stream`, `loom chat /stream`)
- [x] Comprehensive unit tests (cognitive, LLM provider)
- [x] Update ARCHITECTURE.md with brain/hand model

---

## Phase 2: Chat Assistant Enhancement (Current)

**Objective**: Extend chat assistant with workspace, file system, and research capabilities.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Chat Assistant (Enhanced)                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Cognitive Loop (Python)                                │   │
│  │  • Interactive chat with tool use                       │   │
│  │  • Deep research mode (spawn sub-agents)                │   │
│  │  • Workspace file management                            │   │
│  │  • Report generation                                    │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│              ┌───────────────┼───────────────┐                 │
│              ▼               ▼               ▼                 │
│         fs:write        fs:read         agent:spawn            │
│         fs:list         web:search      agent:result           │
│              │               │               │                 │
│              └───────────────┴───────────────┘                 │
│                              │                                  │
│                    workspace/reports/                           │
└─────────────────────────────────────────────────────────────────┘
```

### Tasks

**2.1 Workspace & File System**

- [ ] `fs:write` - Write files to workspace
- [ ] `fs:list` - List directory contents
- [ ] `fs:delete` - Delete files
- [ ] Workspace isolation (agents can only access their workspace)

**2.2 Agent Spawning (Research Mode)**

- [ ] `/research` command to enter research mode
- [ ] Agent spawning via events (`agent.spawn`)
- [ ] Result collection via events (`agent.result`)
- [ ] Context isolation per sub-agent

**2.3 Web Search Integration**

- [ ] Web search tool (Brave Search MCP)
- [ ] Citation extraction and formatting

**2.4 Report Generation**

- [ ] Markdown report structure
- [ ] Save to `workspace/reports/`

**Acceptance Criteria**:

- ✅ User can chat normally with tool use
- ✅ User types `/research "AI frameworks"` → spawns researchers
- ✅ Researchers have isolated context
- ✅ Final report saved to workspace

---

## Phase 3: Market Analyst App (3-4 weeks)

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

## Phase 4: Desktop Assistant App (4 weeks)

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

| Phase   | Duration  | Deliverable                |
| ------- | --------- | -------------------------- |
| Phase 1 | 1 week    | Foundation ✅              |
| Phase 2 | 2 weeks   | Chat Assistant Enhancement |
| Phase 3 | 3-4 weeks | Market Analyst app         |
| Phase 4 | 4 weeks   | Desktop Assistant app      |
| Phase 5 | Ongoing   | Architecture cleanup       |

**Total**: ~11 weeks to Desktop Assistant

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

_Last updated: 2025-12-03_
