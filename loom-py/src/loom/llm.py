"""LLM Helper for Loom Agents

Provides convenient wrappers for calling LLM providers via the Core ActionBroker.
Supports multiple providers: DeepSeek, OpenAI, local models, etc.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from typing import Dict, List, Optional

from .context import Context


@dataclass
class LLMConfig:
    """Configuration for an LLM provider."""

    base_url: str
    model: str
    api_key: Optional[str] = None
    temperature: float = 0.7
    max_tokens: int = 4096
    timeout_ms: int = 30000


class LLMProvider:
    """Helper class for calling LLM providers via ActionBroker."""

    # Pre-configured popular providers
    DEEPSEEK = LLMConfig(
        base_url="https://api.deepseek.com/v1",
        model="deepseek-chat",
        api_key=os.getenv("DEEPSEEK_API_KEY"),
        temperature=0.7,
        max_tokens=4096,
        timeout_ms=30000,
    )

    OPENAI = LLMConfig(
        base_url="https://api.openai.com/v1",
        model="gpt-4o-mini",
        api_key=os.getenv("OPENAI_API_KEY"),
        temperature=0.7,
        max_tokens=4096,
        timeout_ms=30000,
    )

    LOCAL = LLMConfig(
        base_url="http://localhost:8000/v1",
        model="qwen2.5-0.5b-instruct",
        temperature=0.8,
        max_tokens=2048,
        timeout_ms=30000,
    )

    def __init__(self, ctx: Context, config: Optional[LLMConfig] = None):
        """Initialize LLM provider.

        Args:
            ctx: Loom agent context
            config: LLM configuration (defaults to LOCAL)
        """
        self.ctx = ctx
        self.config = config or self.LOCAL

    @classmethod
    def from_name(cls, ctx: Context, provider_name: str) -> LLMProvider:
        """Create provider from name.

        Args:
            ctx: Loom agent context
            provider_name: One of "deepseek", "openai", "local" or a provider from loom.toml

        Returns:
            Configured LLMProvider instance
        """
        # First try to load from project config
        from .config import load_project_config

        project_config = load_project_config()

        # Check if provider exists in project config
        if provider_name in project_config.llm_providers:
            provider_config = project_config.llm_providers[provider_name]
            config = LLMConfig(
                base_url=provider_config.api_base or "http://localhost:8000/v1",
                model=provider_config.model or "default",
                api_key=provider_config.api_key,
                temperature=provider_config.temperature,
                max_tokens=provider_config.max_tokens,
                timeout_ms=provider_config.timeout_sec * 1000,
            )
            return cls(ctx, config)

        # Fall back to hardcoded configs
        configs = {
            "deepseek": cls.DEEPSEEK,
            "openai": cls.OPENAI,
            "local": cls.LOCAL,
        }
        config = configs.get(provider_name.lower())
        if not config:
            raise ValueError(
                f"Unknown provider: {provider_name}. Choose from: {list(configs.keys())}"
            )
        return cls(ctx, config)

    async def generate(
        self,
        prompt: str,
        *,
        system: Optional[str] = None,
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        timeout_ms: Optional[int] = None,
    ) -> str:
        """Generate text completion.

        Args:
            prompt: User prompt/input
            system: Optional system prompt
            temperature: Override default temperature
            max_tokens: Override default max tokens
            timeout_ms: Override default timeout

        Returns:
            Generated text

        Raises:
            RuntimeError: If LLM call fails
        """
        # Build prompt bundle
        bundle = {
            "system": system or "",
            "instructions": prompt,
            "context_docs": [],
            "history": [],
        }

        # Build budget
        budget = {
            "max_input_tokens": 8192,
            "max_output_tokens": max_tokens or self.config.max_tokens,
        }

        payload = {
            "bundle": bundle,
            "budget": budget,
        }

        # Build headers with provider config
        headers = {
            "base_url": self.config.base_url,
            "model": self.config.model,
            "temperature": str(temperature if temperature is not None else self.config.temperature),
        }

        if self.config.api_key:
            headers["api_key"] = self.config.api_key

        # Call llm.generate capability
        result = await self.ctx.tool(
            "llm.generate",
            version="0.1.0",
            payload=payload,
            timeout_ms=timeout_ms or self.config.timeout_ms,
            headers=headers,
        )

        # Parse response
        response = json.loads(result.decode("utf-8"))
        return response.get("text", "")

    async def chat(
        self,
        messages: List[Dict[str, str]],
        *,
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        timeout_ms: Optional[int] = None,
    ) -> str:
        """Chat completion with message history.

        Args:
            messages: List of message dicts with "role" and "content" keys
            temperature: Override default temperature
            max_tokens: Override default max tokens
            timeout_ms: Override default timeout

        Returns:
            Assistant's response text

        Raises:
            RuntimeError: If LLM call fails
        """
        # Convert messages to prompt bundle format
        system = ""
        instructions = ""
        history = []

        for msg in messages:
            role = msg.get("role")
            content = msg.get("content", "")

            if role == "system":
                system = content
            elif role == "user":
                if not instructions:
                    instructions = content
                else:
                    history.append({"role": "user", "content": content})
            elif role == "assistant":
                history.append({"role": "assistant", "content": content})

        bundle = {
            "system": system,
            "instructions": instructions,
            "context_docs": [],
            "history": history,
        }

        budget = {
            "max_input_tokens": 8192,
            "max_output_tokens": max_tokens or self.config.max_tokens,
        }

        payload = {
            "bundle": bundle,
            "budget": budget,
        }

        headers = {
            "base_url": self.config.base_url,
            "model": self.config.model,
            "temperature": str(temperature if temperature is not None else self.config.temperature),
        }

        if self.config.api_key:
            headers["api_key"] = self.config.api_key

        result = await self.ctx.tool(
            "llm.generate",
            version="0.1.0",
            payload=payload,
            timeout_ms=timeout_ms or self.config.timeout_ms,
            headers=headers,
        )

        response = json.loads(result.decode("utf-8"))
        return response.get("text", "")


__all__ = ["LLMProvider", "LLMConfig"]
