"""Provider abstraction for LLM and related AI services.

This module exposes a simple pluggable interface so that agent capability
functions can remain provider-agnostic. The design goals:

- Unified request/response envelopes for chat, completion, embedding.
- Streaming support (async generator) where underlying provider supports it.
- Minimal dependency surface: core SDK avoids heavy ML libs unless user opts in.
- Late binding of credentials (env vars, config file, or secret manager stub).
- Easy extension: user supplies a class implementing the abstract methods and
  registers it in loom.toml or via code (Agent(..., provider=MyProvider())).

Future: integrate tool calling, function calling, reasoning, multi-turn state.
"""
from .base import Provider, ChatMessage, ChatRequest, ChatResponse, Usage
from .openai import OpenAIProvider

__all__ = [
    "Provider",
    "ChatMessage",
    "ChatRequest",
    "ChatResponse",
    "Usage",
    "OpenAIProvider",
]
