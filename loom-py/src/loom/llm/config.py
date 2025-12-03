"""LLM Configuration - Settings for LLM providers.

This module defines configuration for LLM API connections.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass
class LLMConfig:
    """Configuration for an LLM provider.

    Attributes:
        base_url: API base URL (e.g., "https://api.openai.com/v1")
        model: Model name to use
        api_key: API key for authentication (optional for local models)
        temperature: Sampling temperature (0.0-2.0)
        max_tokens: Maximum tokens to generate
        timeout_ms: Request timeout in milliseconds
    """

    base_url: str
    model: str
    api_key: Optional[str] = None
    temperature: float = 0.7
    max_tokens: int = 4096
    timeout_ms: int = 30000


__all__ = ["LLMConfig"]
