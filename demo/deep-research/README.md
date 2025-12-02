# DeepResearch Demo

A multi-agent deep research system powered by Loom OS, inspired by [Anthropic's research on building effective agents](https://www.anthropic.com/engineering/building-effective-agents) and [LangChain's DeepAgents](https://github.com/langchain-ai/deepagents).

## Overview

DeepResearch demonstrates Loom's core multi-agent capabilities:

- **Context Isolation**: Each Research Agent has its own isolated context window, preventing cross-contamination of information
- **True Parallel Execution**: Multiple Research Agents work simultaneously via async event-driven architecture
- **Dynamic Agent Spawning**: Lead Agent creates Research Agents on demand
- **Cognitive Loop**: Each agent uses perceive-think-act reasoning
- **Report Aggregation**: Multiple perspectives synthesized into one report

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER                                     │
│                     "Research AI agents"                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      LEAD AGENT                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Decompose   │─▶│ Spawn       │─▶│ Aggregate   │             │
│  │ Query       │  │ Researchers │  │ Results     │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                          │                                       │
│         ┌────────────────┼────────────────┐                     │
│         ▼                ▼                ▼                     │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│  │ Researcher 1 │ │ Researcher 2 │ │ Researcher 3 │            │
│  │ "frameworks" │ │ "use cases"  │ │ "challenges" │            │
│  └──────────────┘ └──────────────┘ └──────────────┘            │
│         │                │                │                     │
│         ▼                ▼                ▼                     │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐            │
│  │ Web Search   │ │ Web Search   │ │ Web Search   │            │
│  │ (MCP)        │ │ (MCP)        │ │ (MCP)        │            │
│  └──────────────┘ └──────────────┘ └──────────────┘            │
│         │                │                │                     │
│         └────────────────┼────────────────┘                     │
│                          ▼                                       │
│                 ┌─────────────────┐                              │
│                 │  Final Report   │                              │
│                 │  (Markdown)     │                              │
│                 └─────────────────┘                              │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# 1. Set up API keys
export DEEPSEEK_API_KEY="sk-..."
export BRAVE_API_KEY="BSA..."  # For web search

# 2. Run the demo (starts Loom Core + Bridge + Agents)
cd demo/deep-research
loom run

# Loom CLI will:
#   - Start the Loom runtime (Core + gRPC Bridge)
#   - Discover and launch all agents in agents/
#   - Open Dashboard at http://localhost:3030

# 3. In another terminal, send a query
python query.py "What are the latest developments in AI agents?"
# Or use interactive mode:
python query.py --interactive

# 4. View the report
cat workspace/reports/latest.md
```

## Project Structure

```
demo/deep-research/
├── loom.toml              # Configuration
├── README.md              # This file
├── agents/
│   ├── lead.py           # Orchestrator agent
│   └── researcher.py     # Research worker agent
└── workspace/
    └── reports/          # Generated research reports
```

## Core Concepts

### Context Isolation

**Why it matters**: Each Research Agent operates with its own isolated context window. This prevents:

- Information bleed between research topics
- Context pollution from unrelated searches
- Token budget conflicts

```python
# Each researcher has isolated state:
Researcher 1: [query_1, search_results_1, analysis_1]  # 4k tokens
Researcher 2: [query_2, search_results_2, analysis_2]  # 4k tokens
Researcher 3: [query_3, search_results_3, analysis_3]  # 4k tokens

# Lead Agent aggregates final results only:
Lead: [original_query, section_1, section_2, section_3]  # 8k tokens
```

### True Parallel Execution

**How it works**: Loom's event-driven architecture enables genuine parallelism:

```
Time ──────────────────────────────────────────────────────►

Lead:      [decompose]─────────────────────────[aggregate]
                 │                                   ▲
                 ├─emit─┬─emit─┬─emit─┐              │
                 ▼      ▼      ▼      ▼              │
Researcher 1: [search]──[analyze]──[emit]───────────┤
Researcher 2:    [search]──[analyze]──[emit]────────┤
Researcher 3:       [search]──[analyze]──[emit]─────┘
```

Agents don't wait for each other - they process events as they arrive.

## How It Works

### 1. Query Decomposition

The Lead Agent receives a user query and decomposes it into sub-queries:

```python
# Example decomposition
query = "What are the latest developments in AI agents?"
sub_queries = [
    "AI agent frameworks and tools 2024",
    "AI agent real-world applications and use cases",
    "AI agent challenges and limitations"
]
```

### 2. Parallel Agent Dispatch

Lead emits research requests simultaneously (not sequentially):

```python
# All requests emitted in parallel via asyncio
for sq in sub_queries:
    await ctx.emit(
        f"research.request.{researcher_id}",
        type="research.request",
        payload={"sub_query": sq, "config": {...}}
    )
```

### 3. Isolated Research Loop

Each Research Agent runs its own cognitive loop with isolated context:

```
┌─────────────────────────────────────────┐
│        Researcher Context Window        │
├─────────────────────────────────────────┤
│ PERCEIVE: Receive sub-query             │
│ THINK: Plan search strategy             │
│ ACT: Execute web search (MCP tool)      │
│ OBSERVE: Store results in local context │
│ THINK: Summarize findings               │
│ ACT: Emit report section                │
└─────────────────────────────────────────┘
```

### 4. Report Aggregation

Lead collects all sections and synthesizes:

```python
# Lead listens for researcher.report events
# Aggregates when all researchers complete
final_report = await aggregate_report(query, completed_sections)
await save_report(report, query)  # workspace/reports/
```

## Configuration

See `loom.toml` for configuration options:

```toml
[agents.lead-agent]
llm_provider = "deepseek"
max_researchers = 5
timeout_sec = 60

[agents.researcher-agent]
llm_provider = "deepseek"
max_sources = 10

[mcp.web-search]
command = "npx"
args = ["-y", "@anthropics/mcp-brave-search"]
```

## Example Output

```markdown
# Research Report: AI Agents in 2024

## Executive Summary

AI agents have evolved significantly in 2024, with major advances in...

## 1. Frameworks and Tools

- LangChain and LangGraph continue to dominate...
- New entrants like CrewAI and AutoGen...

## 2. Real-World Applications

- Customer service automation...
- Code generation assistants...

## 3. Challenges and Limitations

- Hallucination remains a concern...
- Context window limitations...

## Sources

[1] https://example.com/ai-agents-2024
[2] https://example.com/langchain-vs-autogen
...
```

## Development

### Using Loom CLI

```bash
# Start everything with one command
loom run
# This starts:
#   - Loom Core (EventBus + AgentRuntime)
#   - gRPC Bridge (for Python agents)
#   - Dashboard (http://localhost:3030)
#   - All agents in agents/ directory

# Or start components separately:
loom up              # Start runtime only
python agents/lead.py        # Run Lead Agent
python agents/researcher.py  # Run Researcher Agent

# Send queries
python query.py "Your research question"
python query.py --interactive  # Interactive mode
```

### Debugging

```bash
# Watch reports being generated
watch -n 1 ls -la workspace/reports/

# View Dashboard for agent topology
open http://localhost:3030

# Check traces in Jaeger (if configured)
open http://localhost:16686
```

### Testing

```bash
# Run tests
pytest tests/

# Test with mock search (no API keys needed)
MOCK_SEARCH=1 loom run
```

## Comparison with DeepAgents

| Feature         | LangChain DeepAgents      | Loom DeepResearch              |
| --------------- | ------------------------- | ------------------------------ |
| Agent Isolation | Subagent context windows  | Event-driven isolated contexts |
| Parallelism     | LangGraph async           | Native async + EventBus        |
| Tools           | Middleware-based          | MCP + Native tools             |
| Memory          | StateBackend/StoreBackend | Memory tiers (WIP)             |
| Observability   | LangSmith                 | Dashboard + OpenTelemetry      |

## Roadmap

- [x] Basic query decomposition
- [x] Agent spawning via events
- [x] Context isolation per agent
- [x] Parallel execution
- [ ] True `ctx.spawn_agent()` API
- [ ] Web search integration (MCP)
- [ ] Report aggregation with LLM
- [ ] Multi-turn conversation
- [ ] Source deduplication
- [ ] Citation formatting
