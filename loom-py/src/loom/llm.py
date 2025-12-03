"""LLM Provider for Loom Agents

Provides direct HTTP calls to LLM APIs (OpenAI-compatible).
Supports multiple providers: DeepSeek, OpenAI, local models, etc.

This module makes direct HTTP requests to LLM APIs, bypassing the Rust Core's
llm:generate tool. This gives Python agents full control over LLM configuration
and allows for faster iteration on prompt engineering.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import TYPE_CHECKING, Dict, List, Optional

import httpx
from opentelemetry import trace

from .context import Context

if TYPE_CHECKING:
    from .config import ProjectConfig

# Get tracer for LLM operation spans
tracer = trace.get_tracer(__name__)


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
            provider_name: One of "deepseek", "openai", "local"

        Returns:
            Configured LLMProvider instance
        """
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

    @classmethod
    def from_config(
        cls,
        ctx: Context,
        provider_name: str,
        project_config: ProjectConfig,
    ) -> LLMProvider:
        """Create provider from ProjectConfig.

        Loads LLM configuration from loom.toml [llm.<provider_name>] section.
        Falls back to built-in presets if not found.

        Args:
            ctx: Loom agent context
            provider_name: Name of the provider (e.g., "deepseek", "openai", "local")
            project_config: Loaded ProjectConfig from loom.toml

        Returns:
            Configured LLMProvider instance

        Example loom.toml:
            [llm.deepseek]
            type = "http"
            api_key = "${DEEPSEEK_API_KEY}"
            api_base = "https://api.deepseek.com"
            model = "deepseek-chat"
            max_tokens = 4096
            temperature = 0.7
            timeout_sec = 30
        """
        # Try to load from project config first
        if provider_name in project_config.llm_providers:
            provider_cfg = project_config.llm_providers[provider_name]

            # Convert ProjectConfig LLMProviderConfig to LLMConfig
            llm_config = LLMConfig(
                base_url=provider_cfg.api_base or "http://localhost:8000/v1",
                model=provider_cfg.model or "unknown",
                api_key=provider_cfg.api_key,
                temperature=provider_cfg.temperature,
                max_tokens=provider_cfg.max_tokens,
                timeout_ms=provider_cfg.timeout_sec * 1000,
            )

            print(f"[loom.llm] Loaded provider '{provider_name}' from loom.toml")
            return cls(ctx, llm_config)

        # Fall back to built-in presets
        print(f"[loom.llm] Provider '{provider_name}' not in loom.toml, using built-in preset")
        return cls.from_name(ctx, provider_name)

    async def generate(
        self,
        prompt: str,
        *,
        system: Optional[str] = None,
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        timeout_ms: Optional[int] = None,
    ) -> str:
        """Generate text completion via direct HTTP call.

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
        temp = temperature if temperature is not None else self.config.temperature
        tokens = max_tokens or self.config.max_tokens
        timeout = (timeout_ms or self.config.timeout_ms) / 1000.0  # Convert to seconds

        # Start LLM generation span
        with tracer.start_as_current_span(
            "llm.generate",
            attributes={
                "llm.provider": self.config.base_url,
                "llm.model": self.config.model,
                "llm.temperature": temp,
                "llm.max_tokens": tokens,
                "llm.prompt.length": len(prompt),
                "llm.system.length": len(system) if system else 0,
                "agent.id": self.ctx.agent_id if self.ctx else "unknown",
            },
        ) as span:
            try:
                # Build messages for chat completions API
                messages = []
                if system:
                    messages.append({"role": "system", "content": system})
                messages.append({"role": "user", "content": prompt})

                # Build request payload
                payload = {
                    "model": self.config.model,
                    "messages": messages,
                    "temperature": temp,
                    "max_tokens": tokens,
                }

                # Build headers
                headers = {"Content-Type": "application/json"}
                if self.config.api_key:
                    headers["Authorization"] = f"Bearer {self.config.api_key}"

                # Make direct HTTP call
                url = f"{self.config.base_url.rstrip('/')}/chat/completions"

                async with httpx.AsyncClient(timeout=timeout) as client:
                    response = await client.post(url, json=payload, headers=headers)
                    response.raise_for_status()
                    result = response.json()

                # Extract generated text
                generated_text = result["choices"][0]["message"]["content"]

                # Record success metrics
                span.set_attribute("llm.response.length", len(generated_text))
                span.set_attribute("llm.status", "success")
                if "usage" in result:
                    span.set_attribute(
                        "llm.usage.prompt_tokens", result["usage"].get("prompt_tokens", 0)
                    )
                    span.set_attribute(
                        "llm.usage.completion_tokens", result["usage"].get("completion_tokens", 0)
                    )
                span.set_status(trace.Status(trace.StatusCode.OK))

                return generated_text

            except httpx.HTTPStatusError as e:
                error_msg = f"LLM HTTP error {e.response.status_code}: {e.response.text}"
                span.set_attribute("llm.status", "error")
                span.set_status(trace.Status(trace.StatusCode.ERROR, error_msg))
                span.record_exception(e)
                raise RuntimeError(error_msg) from e
            except Exception as e:
                span.set_attribute("llm.status", "error")
                span.set_status(trace.Status(trace.StatusCode.ERROR, f"LLM generation failed: {e}"))
                span.record_exception(e)
                raise RuntimeError(f"LLM generation failed: {e}") from e

    async def chat(
        self,
        messages: List[Dict[str, str]],
        *,
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        timeout_ms: Optional[int] = None,
    ) -> str:
        """Chat completion with message history via direct HTTP call.

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
        temp = temperature if temperature is not None else self.config.temperature
        tokens = max_tokens or self.config.max_tokens
        timeout = (timeout_ms or self.config.timeout_ms) / 1000.0

        with tracer.start_as_current_span(
            "llm.chat",
            attributes={
                "llm.provider": self.config.base_url,
                "llm.model": self.config.model,
                "llm.temperature": temp,
                "llm.max_tokens": tokens,
                "llm.messages.count": len(messages),
                "agent.id": self.ctx.agent_id if self.ctx else "unknown",
            },
        ) as span:
            try:
                # Build request payload
                payload = {
                    "model": self.config.model,
                    "messages": messages,
                    "temperature": temp,
                    "max_tokens": tokens,
                }

                # Build headers
                headers = {"Content-Type": "application/json"}
                if self.config.api_key:
                    headers["Authorization"] = f"Bearer {self.config.api_key}"

                # Make direct HTTP call
                url = f"{self.config.base_url.rstrip('/')}/chat/completions"

                async with httpx.AsyncClient(timeout=timeout) as client:
                    response = await client.post(url, json=payload, headers=headers)
                    response.raise_for_status()
                    result = response.json()

                # Extract generated text
                generated_text = result["choices"][0]["message"]["content"]

                # Record success metrics
                span.set_attribute("llm.response.length", len(generated_text))
                span.set_attribute("llm.status", "success")
                span.set_status(trace.Status(trace.StatusCode.OK))

                return generated_text

            except httpx.HTTPStatusError as e:
                error_msg = f"LLM HTTP error {e.response.status_code}: {e.response.text}"
                span.set_attribute("llm.status", "error")
                span.set_status(trace.Status(trace.StatusCode.ERROR, error_msg))
                span.record_exception(e)
                raise RuntimeError(error_msg) from e
            except Exception as e:
                span.set_attribute("llm.status", "error")
                span.set_status(trace.Status(trace.StatusCode.ERROR, f"LLM chat failed: {e}"))
                span.record_exception(e)
                raise RuntimeError(f"LLM chat failed: {e}") from e


__all__ = ["LLMProvider", "LLMConfig"]
