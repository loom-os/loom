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

**Objective**: Build a conversational deep research system where a Lead Agent orchestrates dynamically spawned Research Agents, each with their own cognitive loop, to produce comprehensive research reports.

### Architecture

```
User ──► loom run ──► Loom Core (EventBus + Runtime)
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
         Lead Agent   Researcher 1  Researcher N
         (orchestrate)  (isolated)   (isolated)
              │            │            │
              │            ▼            ▼
              │      [own context]  [own context]
              │         search        search
              │         analyze       analyze
              │            │            │
              └────────────┴────────────┘
                           │
                           ▼
                    Final Report (Markdown)
```

**Key**: Each Researcher has isolated context — no cross-contamination.

### Week 1: Foundation (Agent Spawning + Basic Cognitive Loop)

**1.1 Python SDK: Agent Spawning API**

- [ ] `ctx.spawn_agent(agent_type, config, timeout_sec)` - Create sub-agent
- [ ] `ctx.wait_for_agent(agent_id)` - Wait for agent completion
- [ ] `ctx.terminate_agent(agent_id)` - Clean up agent
- [ ] Agent lifecycle events: `agent.spawned`, `agent.completed`, `agent.failed`

**Files to modify**:

- `loom-py/src/loom/context.py` - Add spawn/wait/terminate methods
- `bridge/src/lib.rs` - Add SpawnAgent/TerminateAgent RPC
- `core/src/agent/runtime.rs` - Expose spawn API via Bridge

**1.2 Python SDK: Cognitive Loop Integration**

- [ ] `CognitiveAgent` class with built-in perceive-think-act loop
- [ ] System prompt + tool configuration
- [ ] ReAct strategy with configurable max iterations
- [ ] Automatic context recording

**Files to create**:

- `loom-py/src/loom/cognitive.py` - CognitiveAgent implementation

**1.3 Basic Lead Agent**

- [ ] CLI interface for user queries
- [ ] Query decomposition into sub-tasks
- [ ] Spawn 2-3 research agents per query
- [ ] Simple aggregation of results

**Files to create**:

- `demo/deep-research/agents/lead.py`
- `demo/deep-research/agents/researcher.py`
- `demo/deep-research/loom.toml`

### Week 2: Tools + Web Search

**2.1 MCP Web Search Integration**

- [ ] Fix MCP tool invocation in Python SDK
- [ ] Integrate Brave Search MCP server
- [ ] Handle rate limits and errors gracefully

**Files to modify**:

- `loom-py/src/loom/context.py` - Fix `ctx.tool()` for MCP
- `demo/deep-research/loom.toml` - MCP server config

**2.2 Native File Tool**

- [ ] `fs.write` - Write report sections to workspace
- [ ] `fs.read` - Read existing reports
- [ ] `fs.list` - List workspace files
- [ ] Workspace sandboxing (prevent path traversal)

**Files to create**:

- `core/src/tools/native/fs.rs` - File system tool

**2.3 Research Agent Loop**

- [ ] Receive sub-query from Lead
- [ ] Web search → extract relevant content
- [ ] LLM summarization of findings
- [ ] Return structured section report

### Week 3: Report Generation + Aggregation

**3.1 Report Template System**

- [ ] Markdown report structure
- [ ] Section headers from sub-queries
- [ ] Source citations with URLs
- [ ] Table of contents generation

**3.2 Lead Agent Aggregation**

- [ ] Collect all researcher outputs
- [ ] LLM-based synthesis pass
- [ ] Coherence editing
- [ ] Final report generation

**3.3 File Output**

- [ ] Write report to `workspace/reports/{query_slug}_{timestamp}.md`
- [ ] Include metadata (agents used, time taken, sources)

### Week 4: Polish + Testing

**4.1 Error Handling**

- [ ] Timeout handling for slow agents
- [ ] Retry logic for failed searches
- [ ] Graceful degradation (partial reports)

**4.2 Observability**

- [ ] Full span instrumentation in Python SDK
- [ ] Dashboard shows agent spawning tree
- [ ] Report preview in Dashboard

**4.3 Testing**

- [ ] Unit tests for cognitive loop
- [ ] Integration test: query → report
- [ ] Performance benchmark: 3 agents, 10 sources

**Acceptance Criteria (Phase 1)**:

- ✅ User asks "What are the latest developments in AI agents?"
- ✅ Lead decomposes into 3 sub-queries
- ✅ 3 Research Agents spawned, each with cognitive loop
- ✅ Each agent does web search, produces section
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

## Current Sprint (Week 1)

| Day     | Task                     | Deliverable               |
| ------- | ------------------------ | ------------------------- |
| Mon-Tue | Agent Spawning API       | `ctx.spawn_agent()` works |
| Wed     | CognitiveAgent class     | Python cognitive loop     |
| Thu     | Lead + Researcher shells | Basic demo structure      |
| Fri     | Integration test         | Query → 3 agents → output |

### Quick Start

```bash
# Run the DeepResearch demo
cd demo/deep-research
loom run                    # Starts Core + Bridge + Agents + Dashboard

# In another terminal
python query.py "What are AI agents?"

# View results
cat workspace/reports/latest.md
open http://localhost:3030  # Dashboard
```

---

_Last updated: 2025-12-03_

---

## References

- [Anthropic: Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents)
- [LangChain DeepAgents](https://github.com/langchain-ai/deepagents) - Similar architecture with subagent isolation
- [METR: Measuring AI Task Length](https://metr.org/blog/2025-03-19-measuring-ai-ability-to-complete-long-tasks/) - Agent task complexity research
