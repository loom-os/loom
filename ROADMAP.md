# Loom OS Roadmap

**Vision**: Build an event-driven AI agent operating system that enables intelligent multi-agent collaboration with proper context engineering, observable reasoning, and long-lifecycle support.

**Strategy**: Incremental demos that progressively unlock core capabilities.

**Core Principles** (Inspired by [Anthropic](https://www.anthropic.com/engineering/building-effective-agents) & [DeepAgents](https://github.com/langchain-ai/deepagents)):

- **Context Isolation**: Each sub-agent operates with its own context window, preventing information bleed
- **True Parallelism**: Multiple agents work simultaneously via event-driven architecture
- **Unified CLI**: `loom run` starts everything — Core, Bridge, Dashboard, and Agents

---

## 🎯 Demo Progression

```
Demo 1: DeepResearch (MVP)           ← Current Focus
    ↓ unlocks: spawn agents, cognitive loop, report aggregation
Demo 2: DeepResearch (Enhanced)
    ↓ unlocks: file system reports, working memory, tool ecosystem
Demo 3: Market Analyst (Long Lifecycle)
    ↓ unlocks: memory tiers, proactive agents, real-time data
Demo 4: Production Market Analyst
    ↓ unlocks: trading execution, risk management, 24/7 operation
```

---

## Phase 1: DeepResearch MVP (4 weeks)

**Objective**: Build a single, fully functional Research Agent with complete cognitive loop, tool calling, and context engineering. Then extend to multi-agent orchestration with "subagent as a tool" pattern.

**Philosophy**: A single useful agent first, then composition. Agent spawning is just another tool.

### Architecture Evolution

```
Stage 1: Single Agent (Weeks 1-2)
┌─────────────────────────────────────────────┐
│              Research Agent                  │
│  ┌─────────────────────────────────────┐    │
│  │         Cognitive Loop              │    │
│  │   perceive → think → act → reflect  │    │
│  └─────────────────────────────────────┘    │
│                    │                         │
│        ┌──────────┼──────────┐              │
│        ▼          ▼          ▼              │
│   web.search   fs.write   llm.think         │
│        │          │          │              │
│        └──────────┴──────────┘              │
│                    │                         │
│           Context Window                     │
│   [system prompt + memory + tool results]   │
└─────────────────────────────────────────────┘
                    │
                    ▼
            Research Report (Markdown)

Stage 2: Multi-Agent (Weeks 3-4)
┌─────────────────────────────────────────────┐
│              Lead Agent                      │
│         (same cognitive loop)               │
│                    │                         │
│        ┌──────────┼──────────┐              │
│        ▼          ▼          ▼              │
│   web.search   fs.write  subagent.spawn ◄── NEW TOOL
│                              │              │
│              ┌───────────────┼───────────┐  │
│              ▼               ▼           ▼  │
│         Researcher 1    Researcher 2   ...  │
│         (isolated)      (isolated)          │
└─────────────────────────────────────────────┘
```

**Key Insight**: `subagent.spawn` is just another tool. The cognitive loop doesn't change.

---

### Week 1: Single Agent Foundation (Cognitive Loop + Tools)

**1.1 Python SDK: Cognitive Loop**

- [ ] `CognitiveAgent` class with perceive-think-act-reflect loop
- [ ] System prompt configuration
- [ ] ReAct strategy with configurable max iterations
- [ ] Tool result handling and context accumulation
- [ ] Automatic context recording to memory

**Files to create/modify**:

- `loom-py/src/loom/cognitive.py` - CognitiveAgent implementation
- `loom-py/src/loom/context.py` - Context accumulation helpers

**1.2 Tool Calling Infrastructure**

- [ ] Fix MCP tool invocation in Python SDK
- [ ] Native tool bridge (Rust → Python)
- [ ] Tool result parsing and error handling
- [ ] Tool timeout and retry logic

**Files to modify**:

- `loom-py/src/loom/context.py` - Fix `ctx.tool()` for MCP
- `bridge/src/lib.rs` - Improve tool call handling

**1.3 Web Search Tool**

- [ ] Integrate Brave Search MCP server
- [ ] Result extraction and summarization
- [ ] Handle rate limits gracefully
- [ ] Source citation formatting

**Files to modify**:

- `demo/deep-research/loom.toml` - MCP server config

### Week 2: Context Engineering + File Output

**2.1 Context Engineering**

- [ ] Working memory: auto-managed context window
- [ ] Context budget management (token counting)
- [ ] Intelligent summarization when context exceeds budget
- [ ] Priority-based context pruning

**Files to create**:

- `loom-py/src/loom/memory.py` - Memory management
- `loom-py/src/loom/context_engineering.py` - Context budget & pruning

**2.2 Native File Tool**

- [ ] `fs.write` - Write report sections to workspace
- [ ] `fs.read` - Read existing reports
- [ ] `fs.list` - List workspace files
- [ ] Workspace sandboxing (prevent path traversal)

**Files to create**:

- `core/src/tools/native/fs.rs` - File system tool

**2.3 Single Agent Demo**

- [ ] CLI interface for user queries
- [ ] Agent receives query, does web search
- [ ] Generates comprehensive report with citations
- [ ] Writes report to `workspace/reports/`

**Files to create**:

- `demo/deep-research/agents/researcher.py` - Complete research agent
- `demo/deep-research/query.py` - CLI interface

**Milestone 1 Acceptance Criteria**:

- ✅ Single agent receives "What are AI agents?"
- ✅ Agent does 3+ web searches autonomously
- ✅ Agent writes structured report with citations
- ✅ Context stays within budget (no overflow)
- ✅ Full trace visible in Jaeger

---

### Week 3: Subagent as a Tool

**3.1 Subagent Tool**

- [ ] `subagent.spawn(agent_type, task, timeout)` - Spawn and wait
- [ ] `subagent.spawn_async(agent_type, task)` - Spawn without blocking
- [ ] `subagent.wait(agent_id)` - Wait for completion
- [ ] `subagent.cancel(agent_id)` - Cancel running agent
- [ ] Agent lifecycle events for observability

**Files to modify**:

- `loom-py/src/loom/context.py` - Add subagent tool methods
- `bridge/src/lib.rs` - Add SpawnAgent/WaitAgent RPC
- `core/src/agent/runtime.rs` - Expose spawn API via Bridge

**3.2 Context Isolation**

- [ ] Each subagent gets fresh context window
- [ ] Parent passes task description only (not full context)
- [ ] Child returns structured result only
- [ ] No context bleed between siblings

**3.3 Lead Agent Implementation**

- [ ] Query decomposition into sub-tasks
- [ ] Spawn researchers using `subagent.spawn` tool
- [ ] Aggregate results from children
- [ ] Synthesize final report

**Files to create**:

- `demo/deep-research/agents/lead.py` - Orchestrating agent

### Week 4: Polish + Multi-Agent Demo

**4.1 Report Aggregation**

- [ ] Collect all researcher outputs
- [ ] LLM-based synthesis pass
- [ ] Deduplicate sources across agents
- [ ] Table of contents generation

**4.2 Error Handling**

- [ ] Timeout handling for slow agents
- [ ] Retry logic for failed searches
- [ ] Graceful degradation (partial reports)

**4.3 Observability**

- [ ] Full span instrumentation in Python SDK
- [ ] Dashboard shows agent spawning tree
- [ ] Report preview in Dashboard

**4.4 Testing**

- [ ] Unit tests for cognitive loop
- [ ] Unit tests for context engineering
- [ ] Integration test: single agent query → report
- [ ] Integration test: multi-agent query → report

**Milestone 2 Acceptance Criteria**:

- ✅ User asks "What are the latest developments in AI agents?"
- ✅ Lead decomposes into 3 sub-queries
- ✅ 3 Research Agents spawned via `subagent.spawn` tool
- ✅ Each agent has isolated context (no bleed)
- ✅ Final report written to `workspace/reports/`
- ✅ Full trace visible in Jaeger (10+ spans)
- ✅ Dashboard shows agent topology

---

## Phase 2: DeepResearch Enhanced (3 weeks)

**Objective**: Add working memory, shell execution, and interactive refinement.

### Week 5: Working Memory

**2.1 Memory Tiers (Python SDK)**

- [ ] `ctx.memory.working` - Current task context (auto-managed)
- [ ] `ctx.memory.session` - Conversation history
- [ ] `ctx.memory.save(key, value)` / `ctx.memory.get(key)`
- [ ] Auto-summarization when working memory exceeds budget

**Files to modify**:

- `loom-py/src/loom/context.py` - Memory API
- `loom-py/src/loom/memory.py` - Memory tiers implementation

**2.2 Memory in Cognitive Loop**

- [ ] Auto-record LLM calls and tool results
- [ ] Context retrieval before each LLM call
- [ ] Memory pruning based on relevance

### Week 6: Shell + Interactive Mode

**2.3 Shell Tool**

- [ ] `shell.exec` - Execute bash commands
- [ ] Output capture and truncation
- [ ] Timeout and resource limits
- [ ] Allowlist/blocklist for safety

**Files to create**:

- `core/src/tools/native/shell.rs`

**2.4 Interactive Refinement**

- [ ] User can ask follow-up questions
- [ ] Lead remembers previous research
- [ ] Incremental report updates
- [ ] "Expand on section X" capability

### Week 7: Report Quality

**2.5 Source Verification**

- [ ] Deduplicate sources across agents
- [ ] Rank sources by relevance
- [ ] Include publication dates
- [ ] Flag conflicting information

**2.6 Report Formatting**

- [ ] Code block syntax highlighting
- [ ] Bullet points and numbered lists
- [ ] Inline citations `[1]`, `[2]`
- [ ] Summary section at top

**Acceptance Criteria (Phase 2)**:

- ✅ Multi-turn conversation with Lead
- ✅ Working memory persists across turns
- ✅ Shell commands can be executed (e.g., `ls`, `cat`)
- ✅ Reports have proper citations

---

## Phase 3: Market Analyst - Long Lifecycle (4 weeks)

**Objective**: Transform Market Analyst into a long-running system with proactive agents.

### Week 8-9: Architecture Refactor

**3.1 Lead Agent (formerly Planner)**

- [ ] Proactive monitoring loop (not just reactive)
- [ ] Periodic context refresh from sub-agents
- [ ] Decision-making with full context
- [ ] Spawn Research Agents for deep dives

**3.2 Data Agent Refactor**

- [ ] Internal price buffer (no per-tick events)
- [ ] Smart alerting (significant moves only)
- [ ] Write data reports to file system
- [ ] Configurable alert thresholds

**3.3 Sentiment Agent Refactor**

- [ ] Periodic web search (every N minutes)
- [ ] News aggregation and deduplication
- [ ] Sentiment scoring with LLM
- [ ] Write sentiment reports to file system

### Week 10-11: Memory + Context

**3.4 Memory Tiers**

- [ ] Working memory: current market state
- [ ] Short-term: recent decisions (1 hour)
- [ ] Long-term: historical patterns (persistent)

**3.5 Context Compression**

- [ ] Summarize old tool calls
- [ ] Compress repeated patterns
- [ ] Priority-based context windowing

**3.6 Executor as Tool**

- [ ] Convert Executor Agent to `@tool("trading.execute")`
- [ ] Lead directly invokes trading
- [ ] Order tracking in memory

**Acceptance Criteria (Phase 3)**:

- ✅ System runs for 1+ hour without restart
- ✅ Reports generated every 5 minutes
- ✅ Memory persists across report cycles
- ✅ Dashboard shows file system reports
- ✅ Lead makes informed decisions from reports

---

## Phase 4: Production Market Analyst (4 weeks)

**Objective**: Production-ready trading system with safety controls.

### Features

- [ ] Real OKX trading integration
- [ ] Position management
- [ ] Risk limits and circuit breakers
- [ ] 24-hour continuous operation
- [ ] Alerting and notifications

---

## Technical Debt & Infrastructure

### Python SDK Improvements (Ongoing)

- [ ] Full span instrumentation
- [ ] Type hints for all APIs
- [ ] Async context managers
- [ ] Better error messages
- [ ] `pip install loom` ready

### Core Improvements (Ongoing)

- [ ] Topic wildcard subscription fix
- [ ] LLM config from loom.toml
- [ ] Persistent memory backend (SQLite)
- [ ] Dashboard report viewer

### Documentation (Ongoing)

- [ ] DeepResearch tutorial
- [ ] Cognitive Agent guide
- [ ] Memory system docs
- [ ] Tool development guide

---

## Quality Gates

### DeepResearch MVP Must:

- ✅ 3+ agents collaborate on a query
- ✅ Web search returns real results
- ✅ Report written to file system
- ✅ Full traces in Jaeger
- ✅ < 60s end-to-end latency

### DeepResearch Enhanced Must:

- ✅ Multi-turn conversation works
- ✅ Memory persists across turns
- ✅ Shell tool executes safely
- ✅ Reports have proper citations

### Market Analyst Must:

- ✅ 1+ hour continuous operation
- ✅ Reports generated periodically
- ✅ Context engineering prevents drift
- ✅ Observable decision-making

---

## Responsibility Matrix

| Component        | Loom Core (Rust) | Python SDK   | Business Code |
| ---------------- | ---------------- | ------------ | ------------- |
| EventBus + QoS   | ✅               | -            | -             |
| Agent Runtime    | ✅               | -            | -             |
| Cognitive Loop   | ✅ (base)        | ✅ (wrapper) | -             |
| Memory Storage   | ✅               | -            | -             |
| Memory API       | -                | ✅           | -             |
| Tool Registry    | ✅               | -            | -             |
| Tool Definition  | -                | ✅ (@tool)   | ✅ (custom)   |
| Agent Logic      | -                | -            | ✅            |
| Report Templates | -                | -            | ✅            |
| Trading Strategy | -                | -            | ✅            |

---

## Timeline Summary

| Phase   | Duration | Deliverable                   |
| ------- | -------- | ----------------------------- |
| Phase 1 | 4 weeks  | DeepResearch MVP              |
| Phase 2 | 3 weeks  | DeepResearch Enhanced         |
| Phase 3 | 4 weeks  | Market Analyst Long Lifecycle |
| Phase 4 | 4 weeks  | Production Market Analyst     |

**Total**: ~15 weeks to production-ready Market Analyst

---

## Current Sprint (Week 1: Single Agent Foundation)

**Goal**: One fully functional Research Agent that can take a query and produce a report.

| Day | Task                        | Deliverable                        |
| --- | --------------------------- | ---------------------------------- |
| Mon | CognitiveAgent class        | Python cognitive loop skeleton     |
| Tue | Tool calling infrastructure | `ctx.tool()` works with MCP        |
| Wed | Web search integration      | Brave Search returns results       |
| Thu | Context engineering basics  | Working memory + budget management |
| Fri | Single agent demo           | Query → web search → report        |

### Success Criteria (End of Week 1)

```bash
# Run single agent research demo
cd demo/deep-research
loom run

# In another terminal
python query.py "What are AI agents?"

# Expected output:
# - Agent does 3+ web searches
# - Report written to workspace/reports/ai_agents_*.md
# - Report has introduction, body sections, citations
# - Traces visible in Jaeger
```

### Key Files to Create This Week

```
loom-py/src/loom/
├── cognitive.py          # CognitiveAgent class
├── memory.py             # Working memory management
└── context_engineering.py # Context budget & pruning

demo/deep-research/
├── loom.toml             # MCP config (Brave Search)
├── query.py              # CLI interface
└── agents/
    └── researcher.py     # Single research agent
```

---

_Last updated: 2025-12-03_

---

## References

- [Anthropic: Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents)
- [LangChain DeepAgents](https://github.com/langchain-ai/deepagents) - Similar architecture with subagent isolation
- [METR: Measuring AI Task Length](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/) - Agent task complexity research
