# LLM Provider Guide

This guide explains how Loom Python SDK performs direct HTTP LLM calls (brain), separate from the Rust runtime (hands), and how to configure and use providers.

## Overview

- Python SDK performs LLM requests directly over HTTP.
- Providers are configured via `loom.toml` under `[llm.<name>]`.
- Streaming and non-streaming responses are supported.
- Use providers inside your cognitive loop without routing through the Rust Core.

## Configuration (`loom.toml`)

Example DeepSeek configuration:

```toml
[llm.deepseek]
 type = "http"
 api_key = "${DEEPSEEK_API_KEY}"
 api_base = "https://api.deepseek.com"
 model = "deepseek-chat"
 max_tokens = 4096
```

Access configuration:

```python
from loom.llm.config import LLMProviderConfig
from loom import load_project_config

config = load_project_config()
provider_cfg: LLMProviderConfig = config.llm_providers["deepseek"]
print(provider_cfg.model)
```

## Using the Provider

```python
from loom.llm.provider import LLMProvider
from loom.llm.types import ChatMessage

provider = LLMProvider.from_name("deepseek")

messages = [
    ChatMessage(role="system", content="You are a helpful assistant."),
    ChatMessage(role="user", content="List 3 fruits in Chinese."),
]

result = provider.generate(messages)
print(result.text)  # Non-streaming result
```

## Streaming Responses

```python
for chunk in provider.stream(messages):
    print(chunk.delta, end="", flush=True)
```

## Error Handling

- Set `LOOM_LOG=debug` to get detailed provider logs.
- Network/API errors raise exceptions; catch and handle in the cognitive loop.

```python
try:
    result = provider.generate(messages)
except Exception as e:
    # fallback logic
    print(f"LLM error: {e}")
```

## Best Practices

- Keep prompts short and use system role to set behavior.
- Prefer streaming for chat UIs.
- Parameterize provider/model via config; avoid hardcoding.

## Next Steps

- See `COGNITIVE_GUIDE.md` for integrating providers in the cognitive loop.
