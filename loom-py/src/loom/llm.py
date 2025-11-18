"""LLM Helper for Loom Agents

Provides convenient wrappers for calling LLM providers via the Core ActionBroker.
Supports multiple providers: DeepSeek, OpenAI, local models, etc.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from typing import TYPE_CHECKING, Dict, List, Optional

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
        # Start LLM generation span
        with tracer.start_as_current_span(
            "llm.generate",
            attributes={
                "llm.provider": self.config.base_url,
                "llm.model": self.config.model,
                "llm.temperature": (
                    temperature if temperature is not None else self.config.temperature
                ),
                "llm.max_tokens": max_tokens or self.config.max_tokens,
                "llm.prompt.length": len(prompt),
                "llm.system.length": len(system) if system else 0,
                "agent.id": self.ctx.agent_id,
            },
        ) as span:
            try:
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
                    "temperature": str(
                        temperature if temperature is not None else self.config.temperature
                    ),
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
                generated_text = response.get("text", "")

                # Record success metrics
                span.set_attribute("llm.response.length", len(generated_text))
                span.set_attribute("llm.status", "success")
                span.set_status(trace.Status(trace.StatusCode.OK))

                return generated_text

            except Exception as e:
                # Record error
                span.set_attribute("llm.status", "error")
                span.set_status(trace.Status(trace.StatusCode.ERROR, f"LLM generation failed: {e}"))
                span.record_exception(e)
                raise

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
