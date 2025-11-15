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

# Set DeepSeek API key (required for LLM-based planning)
export DEEPSEEK_API_KEY="sk-your-key-here"

# Optional: For sentiment analysis web search
export BRAVE_API_KEY="your-brave-search-key"

# Note: Real market data from Binance works out-of-the-box (no API key needed)
```

### 2. Run the Demo

```bash
cd demo/market-analyst
loom run
```

This single command will:

- Download and start Loom Core + Dashboard
- Start all 5 Python agents (data, trend, risk, sentiment, planner)
- Connect to Binance API for real market data
- Use DeepSeek LLM for intelligent planning (if API key set)
- Display Dashboard URL: http://localhost:3030

### 3. Watch It Work

Open the Dashboard to see:

- **Real-time event stream** with price updates from Binance
- **Agent network graph** showing message flows between agents
- **LLM tool invocations** (DeepSeek generating trading recommendations)
- **Plan generation** with intelligent reasoning or rule-based fallback
- **Market metrics**: live BTC price, 24h change, volume

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
type = "http"
api_key = "${DEEPSEEK_API_KEY}"
api_base = "https://api.deepseek.com"
model = "deepseek-chat"
max_tokens = 4096
temperature = 0.7
```

**How it works**: The planner agent uses the Python SDK's `LLMProvider` class, which dynamically configures the Core's LLM client via headers. This allows agents to use different models/providers without restarting Core.

### Market Data

```toml
[agents.data-agent]
exchange = "binance"
symbols = ["BTCUSDT"]
refresh_interval_sec = 1
```

The data agent automatically connects to Binance's public API. No API key required! Falls back to simulation if the API is unavailable.

### Agent Behavior

```toml
[agents.planner-agent]
llm_provider = "deepseek"  # or "openai", "local"
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

2. Run: `loom run` (auto-discovers agents/\*.py)

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

## Features

### Real Market Data from Binance

The data agent fetches live cryptocurrency prices from Binance's public API:

- **No API key required**: Uses public REST endpoints
- **Real-time prices**: BTC/USDT ticker data
- **Rich metrics**: 24h high/low, volume, price change %
- **Automatic fallback**: Uses simulation if API unavailable

Example output:

```
[data] BINANCE | BTC $45,234.56 | 24h: +3.42%
```

### LLM-Powered Planning

The planner agent uses DeepSeek (or other LLMs) for intelligent trading decisions:

- **Structured prompts**: Provides trend, risk, and sentiment analysis to LLM
- **JSON output parsing**: Extracts action, confidence, reasoning
- **Graceful fallback**: Falls back to rule-based logic if LLM fails
- **Configurable**: Switch between DeepSeek, OpenAI, or local models

Example LLM output:

```json
{
  "action": "BUY",
  "confidence": 0.82,
  "reasoning": "Strong upward trend with low risk and bullish sentiment"
}
```

### Multi-Agent Collaboration

- **Fan-out**: Price updates broadcast to 3 analysis agents simultaneously
- **Parallel processing**: Trend, risk, and sentiment analysis happen concurrently
- **Smart aggregation**: Planner waits for all results or times out at 3 seconds
- **Partial data handling**: Generates plan even if some agents are slow

## Troubleshooting

### "LLM error" in planner output

The planner will automatically fall back to rule-based logic. To use LLM:

```bash
# Set your DeepSeek API key
export DEEPSEEK_API_KEY="sk-..."

# Restart the demo
loom run
```

### "Warning: aiohttp not installed"

Install the required dependency:

```bash
pip install aiohttp
```

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

### Binance API rate limits

If you see "429 Too Many Requests", increase the refresh interval:

```toml
[agents.data-agent]
refresh_interval_sec = 2  # Slow down to 2 seconds
```

## Development

### Using Local Core Build

If you're developing on Loom Core, the SDK will automatically use your local build:

```bash
# Terminal 1: Build Core from source
cd /path/to/loom/repo
cargo build --release -p loom-core

# Terminal 2: Run demo (will detect and use local build)
cd /path/to/loom/demo/market-analyst
loom run
```

The SDK searches for binaries in this priority:

1. **Cached** (`~/.cache/loom/bin/`)
2. **Local build** (`target/debug/` or `target/release/`)
3. **GitHub Releases** (auto-download)

See `../../docs/BUILD_LOCAL.md` for complete build instructions.

### Running Individual Agents

For debugging, you can run agents separately:

```bash
# Start runtime only
loom up

# In separate terminals
python agents/data.py
python agents/trend.py
python agents/risk.py
python agents/sentiment.py
python agents/planner.py
```

## Next Steps

### Extend the System

- **More symbols**: Add ETH, SOL, etc. in `loom.toml`
- **Execution agent**: Implement paper trading with order tracking
- **Memory system**: Add agent memory for historical context
- **More MCP tools**: Integrate news APIs, social sentiment, on-chain data
- **WebSocket streaming**: Upgrade to Binance WebSocket for real-time updates

### Production Deployment

- **Docker**: Use `infra/docker-compose.yml` for containerized deployment
- **Monitoring**: Enable OpenTelemetry metrics (see `docs/observability/`)
- **Rate limiting**: Configure request throttling for API calls
- **High availability**: Run multiple Core instances with load balancing

### Advanced LLM Usage

- **Chain of thought**: Multi-step reasoning for complex decisions
- **Tool use**: Let LLM decide when to call analysis agents
- **Ensemble**: Combine multiple LLM providers for robust decisions
- **Fine-tuning**: Train custom models on your trading strategy

## Learn More

- [Loom Documentation](../../docs/)
- [Build from Source](../../docs/BUILD_LOCAL.md)
- [ROADMAP](../../docs/ROADMAP.md)
- [SDK Guide](../../loom-py/docs/SDK_GUIDE.md)
