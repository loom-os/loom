# LLM Integration Guide

The Loom Python SDK provides easy integration with Large Language Models through the `LLMProvider` helper class.

## Overview

`LLMProvider` is a convenience wrapper around the Core's `llm.generate` capability, making it easy to call LLMs from your agents without dealing with low-level tool invocation details.

## Supported Providers

### Built-in Providers

- **DeepSeek**: Fast and cost-effective LLM with strong reasoning capabilities
- **OpenAI**: GPT models (requires API key)
- **Local**: Self-hosted models via vLLM or compatible servers

### Adding Custom Providers

You can easily add custom providers by creating an `LLMConfig`:

```python
from loom import LLMConfig, LLMProvider

custom_config = LLMConfig(
    base_url="https://api.example.com/v1",
    model="custom-model",
    api_key="your-api-key",
    temperature=0.7,
    max_tokens=2048,
)

llm = LLMProvider(ctx, config=custom_config)
```

## Quick Start

### 1. Import and Initialize

```python
from loom import Agent, LLMProvider

async def my_handler(ctx, topic, event):
    # Initialize LLM provider (do this once at agent startup)
    llm = LLMProvider.from_name(ctx, "deepseek")

    # Generate text
    response = await llm.generate(
        prompt="What is the capital of France?",
        temperature=0.3,
    )

    print(f"LLM response: {response}")
```

### 2. Environment Setup

Set API keys via environment variables:

```bash
export DEEPSEEK_API_KEY="sk-your-key-here"
export OPENAI_API_KEY="sk-your-key-here"
```

### 3. Configure in loom.toml

```toml
[agents.my-agent]
llm_provider = "deepseek"  # or "openai", "local"
```

## Usage Examples

### Simple Text Generation

```python
llm = LLMProvider.from_name(ctx, "deepseek")

response = await llm.generate(
    prompt="Explain quantum computing in simple terms",
    system="You are a helpful science teacher",
    temperature=0.7,
    max_tokens=500,
)
```

### Chat with Message History

```python
llm = LLMProvider.from_name(ctx, "deepseek")

messages = [
    {"role": "system", "content": "You are a helpful assistant"},
    {"role": "user", "content": "What is Python?"},
    {"role": "assistant", "content": "Python is a programming language..."},
    {"role": "user", "content": "What are its main features?"},
]

response = await llm.chat(messages, temperature=0.5)
```

### Structured Output (JSON)

```python
prompt = """
Analyze this market data and provide a JSON response:

Data: BTC price is $45,000, up 5% in 24h

Format:
{
    "trend": "bullish" | "bearish" | "neutral",
    "confidence": 0.0-1.0,
    "reasoning": "brief explanation"
}
"""

response = await llm.generate(
    prompt=prompt,
    system="You are a market analyst. Respond only with valid JSON.",
    temperature=0.3,
)

# Parse JSON
import json
data = json.loads(response)
```

### Error Handling

```python
try:
    llm = LLMProvider.from_name(ctx, "deepseek")
    response = await llm.generate(prompt="Hello!")
except ValueError as e:
    print(f"Invalid provider name: {e}")
except RuntimeError as e:
    print(f"LLM call failed: {e}")
```

## Advanced Configuration

### Provider-Specific Settings

```python
from loom import LLMConfig, LLMProvider

# High-quality, slower responses
high_quality = LLMConfig(
    base_url="https://api.deepseek.com/v1",
    model="deepseek-chat",
    api_key=os.getenv("DEEPSEEK_API_KEY"),
    temperature=0.3,  # More deterministic
    max_tokens=8192,  # Longer responses
    timeout_ms=60000,  # 60 second timeout
)

llm = LLMProvider(ctx, config=high_quality)
```

### Multiple Providers in One Agent

```python
class MultiLLMAgent:
    def __init__(self, ctx):
        self.fast_llm = LLMProvider.from_name(ctx, "local")
        self.smart_llm = LLMProvider.from_name(ctx, "deepseek")

    async def process(self, query):
        # Use fast local model for simple tasks
        if len(query) < 50:
            return await self.fast_llm.generate(query)

        # Use smart cloud model for complex tasks
        return await self.smart_llm.generate(query)
```

## Best Practices

### 1. Initialize Once

Create the `LLMProvider` once at agent startup, not in every event handler:

```python
# Good
llm_provider = None

async def main():
    global llm_provider
    agent = Agent(...)
    await agent.start()
    llm_provider = LLMProvider.from_name(agent._ctx, "deepseek")

async def handler(ctx, topic, event):
    response = await llm_provider.generate(...)
```

### 2. Handle Failures Gracefully

Always provide fallback behavior if LLM calls fail:

```python
async def make_decision(ctx, data):
    try:
        # Try LLM first
        response = await llm.generate(prompt=build_prompt(data))
        return parse_llm_response(response)
    except Exception as e:
        print(f"LLM failed: {e}, using rule-based fallback")
        return rule_based_decision(data)
```

### 3. Use System Prompts

System prompts help control output format and behavior:

```python
response = await llm.generate(
    prompt="Analyze this: ...",
    system="You are a financial analyst. Always provide: 1) Summary, 2) Risk level, 3) Recommendation"
)
```

### 4. Control Temperature

- **Low temperature (0.1-0.3)**: Deterministic, factual responses
- **Medium temperature (0.5-0.7)**: Balanced creativity and consistency
- **High temperature (0.8-1.0)**: Creative, diverse responses

### 5. Set Appropriate Timeouts

```python
# Short timeout for user-facing responses
response = await llm.generate(prompt, timeout_ms=5000)

# Longer timeout for complex analysis
response = await llm.generate(prompt, timeout_ms=30000)
```

## Troubleshooting

### "Unknown provider" Error

Make sure you're using one of the built-in provider names:

```python
# Correct
llm = LLMProvider.from_name(ctx, "deepseek")  # lowercase

# Incorrect
llm = LLMProvider.from_name(ctx, "DeepSeek")  # case-sensitive
```

### "Tool call failed" Error

Check these common issues:

1. **API Key**: Make sure the environment variable is set
2. **Network**: Check internet connectivity for cloud providers
3. **Model Name**: Verify the model name is correct for your provider
4. **Timeout**: Increase timeout for complex queries

### API Key Not Found

```bash
# Check if API key is set
echo $DEEPSEEK_API_KEY

# Set it if missing
export DEEPSEEK_API_KEY="sk-your-key-here"
```

### Local Model Not Responding

Make sure your local model server is running:

```bash
# Check if vLLM is running
curl http://localhost:8000/v1/models

# Start vLLM if needed
python -m vllm.entrypoints.openai.api_server \
    --model qwen2.5-0.5b-instruct \
    --port 8000
```

## Examples

See the Market Analyst demo for a complete example:

- **File**: `demo/market-analyst/agents/planner.py`
- **Features**: LLM-based trading decisions with rule-based fallback
- **Provider**: DeepSeek (configurable)

## API Reference

### LLMProvider

```python
class LLMProvider:
    def __init__(self, ctx: Context, config: Optional[LLMConfig] = None)

    @classmethod
    def from_name(cls, ctx: Context, provider_name: str) -> LLMProvider

    async def generate(
        self,
        prompt: str,
        *,
        system: Optional[str] = None,
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        timeout_ms: Optional[int] = None,
    ) -> str

    async def chat(
        self,
        messages: List[Dict[str, str]],
        *,
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        timeout_ms: Optional[int] = None,
    ) -> str
```

### LLMConfig

```python
@dataclass
class LLMConfig:
    base_url: str
    model: str
    api_key: Optional[str] = None
    temperature: float = 0.7
    max_tokens: int = 4096
    timeout_ms: int = 30000
```

## Learn More

- [Market Analyst Demo](../../demo/market-analyst/README.md)
- [Python SDK Guide](SDK_GUIDE.md)
- [Core LLM Module](../../core/src/llm/README.md)
