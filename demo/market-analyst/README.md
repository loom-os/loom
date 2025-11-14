# Market Analyst Demo

Real-time multi-agent market analysis system powered by Loom, DeepSeek LLM, and async event-driven architecture.

## Overview

This demo showcases Loom's capabilities for building production-ready multi-agent systems:

- **Async Fan-Out**: Market data broadcasts to multiple analysis agents simultaneously
- **Parallel Processing**: Trend, risk, and sentiment analysis happen concurrently
- **Smart Aggregation**: Planner waits for all results or times out intelligently
- **Real-time Dashboard**: Visualize agent communication and event flow
- **LLM Integration**: DeepSeek API for intelligent planning and decision-making

## Architecture

```
data_agent (market prices)
    ↓ emits market.price.BTC
    ├──→ trend_agent (technical indicators)
    ├──→ risk_agent (risk metrics)
    └──→ sentiment_agent (news + social)
            ↓ emit analysis.*
            └──→ planner_agent (aggregate + LLM reasoning)
                    ↓ emit plan.ready
```

## Quick Start

### 1. Prerequisites

```bash
# Install Loom SDK
pip install loom

# Set API key
export DEEPSEEK_API_KEY="sk-your-key-here"

# Optional: For sentiment analysis
export BRAVE_API_KEY="your-brave-search-key"
```

### 2. Run the Demo

```bash
cd demo/market-analyst
loom run
```

This single command will:
- Download and start Loom Core + Dashboard
- Start all 5 Python agents
- Display Dashboard URL: http://localhost:3030

### 3. Watch It Work

Open the Dashboard to see:
- Real-time event stream
- Agent network graph with message flows
- Tool invocations (LLM calls, MCP tools)
- Plan generation with reasoning

## Project Structure

```
demo/market-analyst/
├── loom.toml           # Configuration
├── README.md           # This file
├── agents/             # Agent implementations
│   ├── data.py        # Market data ingestion
│   ├── trend.py       # Technical analysis
│   ├── risk.py        # Risk metrics
│   ├── sentiment.py   # Sentiment analysis (LLM + MCP)
│   └── planner.py     # Aggregation + planning (LLM)
└── logs/              # Created automatically

```

## Configuration

See `loom.toml` for full configuration. Key settings:

### LLM Provider

```toml
[llm.deepseek]
api_key = "${DEEPSEEK_API_KEY}"
model = "deepseek-chat"
max_tokens = 4096
```

### Agent Behavior

```toml
[agents.planner-agent]
llm_provider = "deepseek"
timeout_ms = 3000  # Wait up to 3s for all analysis
```

## Development

### Run Individual Agents

```bash
# Start runtime only
loom up

# In separate terminals, run agents
python agents/data.py
python agents/trend.py
# ...
```

### View Logs

```bash
# Enable logging
loom run --logs

# View logs
tail -f logs/*.log
```

### Use Local LLM

Update `loom.toml` to use local model:

```toml
[agents.planner-agent]
llm_provider = "local"  # Instead of "deepseek"
```

## Extending

### Add a New Agent

1. Create `agents/my_agent.py`:

```python
from loom import Agent

async def my_handler(ctx, topic, event):
    # Your logic here
    await ctx.emit("my.output", type="result", payload=b"...")

agent = Agent("my-agent", topics=["input.topic"], on_event=my_handler)

if __name__ == "__main__":
    agent.run()
```

2. Run: `loom run` (auto-discovers agents/*.py)

### Integrate New MCP Tools

Add to `loom.toml`:

```toml
[mcp.weather]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-weather"]
```

Use in agent:

```python
result = await ctx.tool("weather:get", payload={"city": "Seattle"})
```

## Performance

- **Latency**: p95 < 200ms for local analysis, < 2s including LLM
- **Throughput**: 10+ price updates/sec with 3+ parallel analyses
- **Scalability**: Add more agents without code changes

## Troubleshooting

### No API key error

```bash
export DEEPSEEK_API_KEY="sk-..."
```

### Port already in use

```bash
loom run --bridge-port 9999 --dashboard-port 8080
```

### Agent crashes

Check logs:

```bash
cat logs/planner-agent.log
```

## Next Steps

- Add more symbols (ETH, SOL, etc.)
- Implement execution agent (paper trading)
- Add memory/history for agents
- Integrate more MCP tools
- Deploy to production with Docker

## Learn More

- [Loom Documentation](../../docs/)
- [ROADMAP](../../docs/ROADMAP.md)
- [SDK Guide](../../loom-py/docs/SDK_GUIDE.md)
