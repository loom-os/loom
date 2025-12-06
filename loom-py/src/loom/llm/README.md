# LLM Module

Direct HTTP client for LLM API calls (DeepSeek, OpenAI, local models).

## Overview

This module provides Python agents with **direct LLM access** for fast iteration on prompt engineering. Part of Loom's Brain/Hand separation:

- **Brain (Python)**: Makes LLM calls directly using this module
- **Hands (Rust Core)**: Handles tool execution, but NOT LLM calls

## Why Direct HTTP?

**Fast Iteration**: Python agents can modify prompts and retry without recompiling Rust
**Flexibility**: Each agent can use different providers, models, and configurations
**Simplicity**: No need to route LLM calls through Core/Bridge

## Key Components

### LLMProvider (`provider.py`)

Main class for calling LLM APIs:

```python
from loom import Agent
from loom.llm import LLMProvider

agent = Agent(agent_id="researcher", topics=["tasks"])
await agent.start()

# Use pre-configured provider
llm = LLMProvider.from_name(agent.ctx, "deepseek")

# Call LLM
response = await llm.call(messages=[
    {"role": "system", "content": "You are a research assistant."},
    {"role": "user", "content": "Summarize recent AI advances"},
])
print(response.content)
```

### LLMConfig (`config.py`)

Configuration for LLM connections:

```python
from loom.llm import LLMConfig

config = LLMConfig(
    base_url="https://api.deepseek.com/v1",
    model="deepseek-chat",
    api_key="sk-...",
    temperature=0.7,
    max_tokens=4096,
    timeout_ms=30000,
)
```

### Message & LLMResponse (`types.py`)

Type definitions for LLM interactions:

```python
from loom.llm import Message, LLMResponse

# Request message
msg = Message(role="user", content="Hello!")

# Response
response = LLMResponse(
    content="Hi there!",
    finish_reason="stop",
    usage={"prompt_tokens": 10, "completion_tokens": 5},
)
```

## Pre-configured Providers

### DeepSeek

```python
llm = LLMProvider.from_name(ctx, "deepseek")
# Model: deepseek-chat
# API: https://api.deepseek.com/v1
# Key: DEEPSEEK_API_KEY env var
```

### OpenAI

```python
llm = LLMProvider.from_name(ctx, "openai")
# Model: gpt-4o-mini
# API: https://api.openai.com/v1
# Key: OPENAI_API_KEY env var
```

### Local (vLLM/Ollama)

```python
llm = LLMProvider.from_name(ctx, "local")
# Model: qwen2.5-0.5b-instruct
# API: http://localhost:8000/v1
# Key: (none)
```

## Usage Patterns

### 1. Simple Chat

```python
response = await llm.call(messages=[
    {"role": "user", "content": "What is Loom?"}
])
print(response.content)
```

### 2. System Prompts

```python
response = await llm.call(messages=[
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Explain quantum computing"},
])
```

### 3. Streaming Responses

```python
async for chunk in llm.stream(messages=[
    {"role": "user", "content": "Write a poem"}
]):
    print(chunk, end="", flush=True)
```

### 4. Tool Use (Future)

```python
response = await llm.call(
    messages=[...],
    tools=[
        {
            "name": "web:search",
            "description": "Search the web",
            "parameters": {...}
        }
    ]
)

if response.tool_calls:
    for call in response.tool_calls:
        result = await ctx.tool(call.name, payload=call.arguments)
```

## Configuration from loom.toml

Load provider config from project file:

**loom.toml:**
```toml
[llm.deepseek]
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"
api_key_env = "DEEPSEEK_API_KEY"
temperature = 0.7
max_tokens = 8192

[llm.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4-turbo"
api_key_env = "OPENAI_API_KEY"
```

**Python:**
```python
from loom.runtime import load_project_config

config = load_project_config("loom.toml")
llm = LLMProvider.from_config(ctx, "deepseek", config)
```

## Error Handling

### 1. API Errors

```python
try:
    response = await llm.call(messages=[...])
except httpx.HTTPStatusError as e:
    if e.response.status_code == 429:
        # Rate limit - backoff and retry
        await asyncio.sleep(5)
    elif e.response.status_code == 401:
        # Invalid API key
        print("Check your API key")
```

### 2. Timeouts

```python
from loom.llm import LLMConfig

config = LLMConfig(
    base_url="...",
    model="...",
    timeout_ms=60000,  # 60s
)

llm = LLMProvider(ctx, config)

try:
    response = await llm.call(messages=[...])
except httpx.TimeoutException:
    print("LLM call timed out")
```

### 3. Network Errors

```python
import httpx

try:
    response = await llm.call(messages=[...])
except httpx.ConnectError:
    print("Failed to connect to LLM API")
    # Fallback to different provider
    llm = LLMProvider.from_name(ctx, "local")
```

## Integration with Cognitive Module

`CognitiveAgent` uses `LLMProvider` for reasoning:

```python
from loom.cognitive import CognitiveAgent, CognitiveConfig
from loom.llm import LLMProvider

cognitive = CognitiveAgent(
    ctx=agent.ctx,
    llm=LLMProvider.from_name(agent.ctx, "deepseek"),
    config=CognitiveConfig(
        system_prompt="You are a research assistant...",
        max_iterations=5,
    )
)

result = await cognitive.run("Research AI frameworks")
```

## OpenTelemetry Tracing

All LLM calls are traced:

```python
from opentelemetry import trace

# LLM calls create spans automatically
response = await llm.call(messages=[...])

# Span attributes:
# - llm.provider: "deepseek"
# - llm.model: "deepseek-chat"
# - llm.tokens.prompt: 150
# - llm.tokens.completion: 200
# - llm.latency.ms: 1523
```

View traces in Jaeger or Grafana.

## Cost Tracking

Track token usage:

```python
total_prompt_tokens = 0
total_completion_tokens = 0

response = await llm.call(messages=[...])
total_prompt_tokens += response.usage["prompt_tokens"]
total_completion_tokens += response.usage["completion_tokens"]

print(f"Total tokens: {total_prompt_tokens + total_completion_tokens}")
print(f"Estimated cost: ${estimate_cost(total_prompt_tokens, total_completion_tokens)}")
```

## Testing

### Mock LLM for Tests

```python
class MockLLM:
    async def call(self, messages, **kwargs):
        return LLMResponse(
            content="Mock response",
            finish_reason="stop",
            usage={"prompt_tokens": 10, "completion_tokens": 5}
        )

# Use in tests
cognitive = CognitiveAgent(ctx=agent.ctx, llm=MockLLM())
```

### Run LLM Tests

```bash
# Unit tests (mocked)
pytest tests/unit/test_llm.py -v

# Integration tests (requires API key)
export DEEPSEEK_API_KEY="sk-..."
pytest tests/integration/test_llm.py -v
```

## Performance Tips

### 1. Connection Pooling

`LLMProvider` reuses `httpx.AsyncClient`:

```python
llm = LLMProvider(ctx, config)
# Multiple calls reuse connection
for i in range(100):
    await llm.call(messages=[...])  # Fast!
```

### 2. Streaming for Long Responses

```python
# Non-streaming: waits for full response
response = await llm.call(messages=[...])  # 10s

# Streaming: starts immediately
async for chunk in llm.stream(messages=[...]):
    print(chunk, end="")  # 0.1s first token
```

### 3. Parallel Calls

```python
import asyncio

# Call multiple LLMs in parallel
tasks = [
    llm1.call(messages=[...]),
    llm2.call(messages=[...]),
    llm3.call(messages=[...]),
]
responses = await asyncio.gather(*tasks)
```

## Supported Providers

| Provider | Base URL | Model | API Key Env |
|----------|----------|-------|-------------|
| DeepSeek | api.deepseek.com/v1 | deepseek-chat | DEEPSEEK_API_KEY |
| OpenAI | api.openai.com/v1 | gpt-4o-mini | OPENAI_API_KEY |
| Local | localhost:8000/v1 | qwen2.5-0.5b | (none) |
| Custom | (custom) | (custom) | (custom) |

## Custom Provider

```python
from loom.llm import LLMConfig, LLMProvider

config = LLMConfig(
    base_url="https://my-llm.example.com/v1",
    model="my-model-v1",
    api_key="custom-key",
    temperature=0.8,
    max_tokens=4096,
)

llm = LLMProvider(ctx, config)
```

## Related Documentation

- **[Cognitive Module](../cognitive/README.md)**: How CognitiveAgent uses LLMProvider
- **[Context Engineering](../context/README.md)**: Token management for prompts
- **[OpenTelemetry](../../../docs/observability/)**: Tracing LLM calls

---

**Key Insight**: Direct LLM access in Python enables fast experimentation. This is the "thinking" part of the Brain, while Rust Core provides the "doing" (tool execution).
