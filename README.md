# Loom — Event-Driven AI Agent Runtime

_Weaving intelligence into the fabric of reality_

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

**Loom is not another agent library.** It's a **runtime** that enables AI agents to run continuously, respond to real-world events, and collaborate across processes.

## Why Loom?

|                         | Python Libraries           | Loom                                             |
| ----------------------- | -------------------------- | ------------------------------------------------ |
| **Nature**              | Library (you call it)      | Runtime (it runs your agents)                    |
| **Lifecycle**           | Script execution (seconds) | Long-running service (hours/days)                |
| **Triggers**            | Code calls only            | Events: hotkeys, file changes, timers, clipboard |
| **Agent Communication** | In-process function calls  | Event Bus (cross-process, cross-language)        |
| **Tool Safety**         | None                       | Sandboxed execution in Rust                      |
| **Desktop Integration** | None                       | Native: system tray, notifications, hotkeys      |

## Architecture: Brain/Hand Separation

The key insight: **LLM reasoning needs rapid iteration; tool execution needs security**.

```
┌─────────────────────────────────────────────────────────────────────┐
│  Python Agent (Brain 🧠)                Rust Core (Hands 🤚)        │
│  ════════════════════════               ═══════════════════════     │
│  • LLM Calls (direct HTTP)              • Event Bus (pub/sub, QoS)  │
│  • Cognitive Loop (ReAct/CoT)           • Tool Registry + Sandbox   │
│  • Context Engineering                  • Agent Lifecycle           │
│  • Business Logic                       • Persistent Memory         │
│                                         • System Integration        │
│  ─────────────────────────────────────────────────────────────────  │
│  Fast iteration, daily changes          Stable infrastructure       │
└─────────────────────────────────────────────────────────────────────┘
```

**Why this split?**

- Prompt engineering changes daily → Python (edit, reload, test)
- Tool execution needs security → Rust (sandbox, permissions)
- System integration needs native access → Rust (hotkeys, clipboard)
- Multi-agent coordination needs performance → Rust (Event Bus)

## Multi-Agent System

Loom agents collaborate via **Event Bus**, not function calls:

```
┌──────────────────────────────────────────────────────────────────┐
│                         Event Bus                                 │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐       │
│  │ Agent A │───▶│ topic.* │◀───│ Agent B │    │ Agent C │       │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘       │
│       │              │              │              │              │
│       └──────────────┴──────────────┴──────────────┘              │
│                   Async, Cross-Process                            │
└──────────────────────────────────────────────────────────────────┘
```

**Collaboration Primitives:**

- `request/reply` — Correlated request-response with timeout
- `fanout/fanin` — Broadcast to N agents, collect K responses
- `contract-net` — Call for proposals, bid, award
- `barrier` — Synchronize N agents before proceeding

## Quick Start

```bash
pip install loom-py
loom up              # Start Rust runtime (auto-downloads)
```

### Example 1: Desktop Assistant (General Agent)

A cognitive agent with tool use, web search, and file operations:

```python
from loom import Agent, CognitiveAgent, LLMProvider

agent = Agent(agent_id="assistant")
await agent.start()

cognitive = CognitiveAgent(
    ctx=agent._ctx,
    llm=LLMProvider.from_env("deepseek"),
    available_tools=["web:search", "fs:read_file", "fs:write_file", "system:shell"],
)

# Interactive chat with ReAct reasoning
result = await cognitive.run("Research Bitcoin price trends and save a summary to report.md")
```

Run it: `cd apps/chat-assistant && loom chat`

### Example 2: Market Analyst (Business-Specific Multi-Agent)

6 specialized agents collaborating in real-time:

```
┌──────────────────────────────────────────────────────────────────┐
│                    Market Analyst System                          │
│                                                                   │
│   ┌────────────┐                                                 │
│   │ Data Agent │──market.price.*──┬──▶ Trend Agent               │
│   │ (OKX API)  │                  ├──▶ Risk Agent                │
│   └────────────┘                  └──▶ Sentiment Agent           │
│                                           │                       │
│                                    analysis.*                     │
│                                           ▼                       │
│                                   ┌──────────────┐               │
│                                   │Planner Agent │               │
│                                   │  (DeepSeek)  │               │
│                                   └──────┬───────┘               │
│                                          │ plan.ready            │
│                                          ▼                       │
│                                   ┌──────────────┐               │
│                                   │Executor Agent│               │
│                                   │ (OKX Trade)  │               │
│                                   └──────────────┘               │
└──────────────────────────────────────────────────────────────────┘
```

```toml
# apps/market-analyst/loom.toml
[agents.data-agent]
topics = ["market.price.BTC", "market.price.ETH"]
data_source = "okx"

[agents.planner-agent]
topics = ["analysis.trend", "analysis.risk", "analysis.sentiment"]
llm_provider = "deepseek"
aggregation_strategy = "complete_or_timeout"

[agents.executor-agent]
topics = ["plan.ready"]
enable_trading = true
```

Run it: `cd apps/market-analyst && loom run`

## What Loom Provides (That Libraries Can't)

**1. Long-Running Agent Lifecycle** — `loom up` starts runtime as background service, agents run continuously

**2. System Event Triggers** — Agents respond to `hotkey.ctrl+space`, `clipboard.changed`, `file.downloads/*`

**3. Secure Tool Execution** — Tools run in Rust sandbox with human-in-the-loop approval

**4. Cross-Process Collaboration** — Agents in different processes/languages communicate via Event Bus

**5. Observable Execution** — Built-in dashboard with real-time events, traces, and agent topology

## Project Structure

```
loom/
├── core/           # Rust runtime: EventBus, Tools, Agent Lifecycle
├── bridge/         # gRPC service connecting Python/JS agents
├── loom-py/        # Python SDK: Agent, CognitiveAgent, LLMProvider
└── apps/
    ├── chat-assistant/   # Desktop cognitive agent
    └── market-analyst/   # Multi-agent trading system
```

## Native Tools

| Tool                                                        | Description                     |
| ----------------------------------------------------------- | ------------------------------- |
| `fs:read_file`, `fs:write_file`, `fs:list_dir`, `fs:delete` | File operations (sandboxed)     |
| `system:shell`                                              | Shell command (allowlist-based) |
| `web:search`                                                | Web search (Brave Search API)   |
| `weather:get`                                               | Weather data (Open-Meteo)       |

## Configuration

```toml
# loom.toml
[bridge]
address = "127.0.0.1:50051"

[llm.deepseek]
type = "http"
api_key = "${DEEPSEEK_API_KEY}"
api_base = "https://api.deepseek.com/v1"
model = "deepseek-chat"
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache License 2.0

---

_Loom — Not a library. A runtime for AI agents that live in the real world._
