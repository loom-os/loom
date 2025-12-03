"""LLM module - Direct HTTP calls to LLM APIs.

This module provides LLM integration for Python agents:
- LLMProvider: Main class for LLM API calls
- LLMConfig: Configuration for API connections
- Message/LLMResponse: Types for LLM interactions

Part of the Brain/Hand separation - Python makes LLM calls directly
for fast iteration on prompt engineering.
"""

from .config import LLMConfig
from .provider import LLMProvider
from .types import LLMResponse, Message

__all__ = [
    "LLMProvider",
    "LLMConfig",
    "Message",
    "LLMResponse",
]
