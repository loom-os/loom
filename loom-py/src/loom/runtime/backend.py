"""Backend Agent - Reusable cognitive backend service.

This module provides a reusable backend agent that handles:
- Bridge connection and event loop
- CognitiveAgent initialization from config
- Streaming protocol handling
- Cancellation support

This extracts common patterns from apps/chat-assistant into reusable infra.

Example:
    ```python
    from loom.runtime import BackendAgent

    async def main():
        backend = BackendAgent.from_config("my-agent", project_dir)
        await backend.run()
    ```
"""

from __future__ import annotations

import asyncio
from pathlib import Path
from typing import TYPE_CHECKING, Optional

from ..agent import Agent
from ..cognitive import CognitiveAgent, CognitiveConfig
from ..context import create_default_registry
from ..llm import LLMProvider
from ..streaming import StreamCancelledError, StreamContentType, StreamingHandler

if TYPE_CHECKING:
    from ..agent import EventContext
    from ..agent.envelope import Envelope
    from .config import ProjectConfig


class BackendAgent:
    """A reusable backend agent with cognitive capabilities.

    Handles the boilerplate of:
    - Loading configuration from loom.toml
    - Connecting to Bridge
    - Setting up CognitiveAgent
    - Processing stream requests
    - Handling cancellation

    Attributes:
        agent_id: Unique identifier for this agent
        agent: The underlying Agent for event bus communication
        cognitive: The CognitiveAgent for LLM reasoning
    """

    def __init__(
        self,
        agent_id: str,
        agent: Agent,
        cognitive: CognitiveAgent,
        input_topic: str = "chat.input",
        verbose: bool = True,
    ):
        """Initialize backend agent.

        Args:
            agent_id: Unique agent identifier
            agent: Agent instance for event bus
            cognitive: CognitiveAgent for reasoning
            input_topic: Topic to listen for requests
            verbose: Whether to print status messages
        """
        self.agent_id = agent_id
        self.agent = agent
        self.cognitive = cognitive
        self.input_topic = input_topic
        self.verbose = verbose

        # Track active streams for cancellation
        self._active_streams: dict[str, StreamingHandler] = {}

    @classmethod
    async def from_config(
        cls,
        agent_id: str,
        project_dir: Path,
        config: Optional["ProjectConfig"] = None,
        verbose: bool = True,
    ) -> "BackendAgent":
        """Create a BackendAgent from project configuration.

        Args:
            agent_id: Agent identifier (must match [agents.{id}] in loom.toml)
            project_dir: Path to project directory containing loom.toml
            config: Optional pre-loaded config (will load from project_dir if None)
            verbose: Whether to print status messages

        Returns:
            Configured BackendAgent ready to run

        Raises:
            ValueError: If configuration is invalid
            ConnectionError: If Bridge connection fails
        """
        from .config import load_project_config

        if config is None:
            config = load_project_config(project_dir)

        # Get agent-specific configuration
        agent_config = config.agents.get(agent_id, {})
        if not agent_config:
            raise ValueError(f"No [{agent_id}] configuration found in loom.toml")

        # Create base Agent for event bus
        input_topic = agent_config.get("input_topic", "chat.input")
        agent = Agent(
            agent_id=agent_id,
            topics=[input_topic],
        )

        try:
            await agent.start()
            if verbose:
                print(f"✅ Connected to Bridge at {agent._ctx.client.address}")
        except Exception as e:
            raise ConnectionError(f"Failed to connect to Bridge: {e}") from e

        # Initialize LLM provider
        llm_provider_name = agent_config.get("llm_provider", "deepseek")
        try:
            llm = LLMProvider.from_config(agent.ctx, llm_provider_name, config)
            if verbose:
                print(f"✅ LLM Provider '{llm_provider_name}' initialized")
        except Exception as e:
            await agent.stop()
            raise ValueError(f"Failed to initialize LLM: {e}") from e

        # Get tool registry for better prompts
        tool_registry = create_default_registry()

        # Create CognitiveAgent
        cognitive = CognitiveAgent(
            ctx=agent.ctx,
            llm=llm,
            config=CognitiveConfig(
                system_prompt=agent_config.get("system_prompt"),
                thinking_strategy=agent_config.get("thinking_strategy", "react"),
                max_iterations=agent_config.get("max_iterations", 20),
            ),
            available_tools=agent_config.get("tools", []),
            tool_registry=tool_registry,
        )

        if verbose:
            print("✅ CognitiveAgent initialized")
            print(f"   Strategy: {cognitive.config.thinking_strategy}")
            print(f"   Tools: {len(cognitive.available_tools)}")

        return cls(
            agent_id=agent_id,
            agent=agent,
            cognitive=cognitive,
            input_topic=input_topic,
            verbose=verbose,
        )

    async def run(self) -> None:
        """Run the backend agent event loop.

        This blocks until interrupted (Ctrl+C) or an error occurs.
        """
        # Attach event handler
        self.agent._on_event = self._handle_event

        if self.verbose:
            print(f"\n{'=' * 60}")
            print(f"✨ Backend Agent '{self.agent_id}' Ready!")
            print(f"{'=' * 60}")
            print(f"Listening on: {self.input_topic}")
            print("Press Ctrl+C to stop.")
            print(f"{'=' * 60}\n")

        try:
            while True:
                await asyncio.sleep(3600)
        except asyncio.CancelledError:
            pass
        finally:
            await self.stop()

    async def stop(self) -> None:
        """Stop the backend agent."""
        if self.verbose:
            print(f"\n[{self.agent_id}] Shutting down...")
        await self.agent.stop()
        if self.verbose:
            print(f"[{self.agent_id}] Stopped.")

    async def _handle_event(
        self,
        ctx: "EventContext",
        topic: str,
        event: "Envelope",
    ) -> None:
        """Handle incoming events."""

        # Handle stream cancellation
        if topic == self.input_topic and event.type == "stream.cancel":
            await self._handle_cancel(event)
            return

        # Handle stream requests
        if topic == self.input_topic and event.type == "stream.request":
            await self._handle_stream_request(ctx, event)
            return

        # Handle legacy non-streaming requests
        if topic == self.input_topic and event.type == "user.message":
            await self._handle_legacy_request(ctx, event)

    async def _handle_cancel(self, event: "Envelope") -> None:
        """Handle stream cancellation request."""
        correlation_id = event.correlation_id
        if correlation_id and correlation_id in self._active_streams:
            handler = self._active_streams[correlation_id]
            reason = event.payload.decode("utf-8") if event.payload else "User cancelled"
            handler.cancel(reason)
            if self.verbose:
                print(f"⚠️  Stream cancelled: {reason}")

    async def _handle_stream_request(
        self,
        ctx: "EventContext",
        event: "Envelope",
    ) -> None:
        """Handle streaming request."""
        from ..cognitive.types import CognitiveResult, ThoughtStep

        handler = StreamingHandler(ctx, event)
        self._active_streams[handler.correlation_id] = handler

        try:
            user_input = event.payload.decode("utf-8")
            if self.verbose:
                print(f"📨 [{event.sender}] {user_input[:80]}...")

            async for item in self.cognitive.run_stream(user_input):
                if isinstance(item, str):
                    await handler.send_chunk(item, StreamContentType.TEXT)

                elif isinstance(item, ThoughtStep):
                    if item.reasoning:
                        await handler.send_thinking(item.reasoning)
                    if item.tool_call:
                        await handler.send_tool_call(
                            item.tool_call.name,
                            str(item.tool_call.arguments),
                        )
                        if item.observation:
                            result_text = (
                                item.observation.output
                                if item.observation.success
                                else f"Error: {item.observation.error}"
                            )
                            await handler.send_tool_result(
                                item.tool_call.name,
                                result_text[:500],
                            )

                elif isinstance(item, CognitiveResult):
                    if self.verbose:
                        print(f"✅ Complete: {item.iterations} iterations")
                    await handler.send_complete(
                        final_content=item.answer,
                        tokens=item.total_tokens,
                    )
                    break

        except StreamCancelledError as e:
            if self.verbose:
                print(f"⚠️  Cancelled: {e.reason}")
            await handler.send_cancelled(e.reason)

        except Exception as e:
            if self.verbose:
                print(f"❌ Error: {e}")
            await handler.send_error(str(e))

        finally:
            self._active_streams.pop(handler.correlation_id, None)

    async def _handle_legacy_request(
        self,
        ctx: "EventContext",
        event: "Envelope",
    ) -> None:
        """Handle legacy non-streaming request."""
        try:
            user_input = event.payload.decode("utf-8")
            if self.verbose:
                print(f"📨 [Legacy] {user_input[:80]}...")

            result = await self.cognitive.run(user_input)

            if self.verbose:
                print(f"✅ Complete: {result.iterations} iterations")

            await ctx.reply(
                event,
                type="assistant.message",
                payload=result.answer.encode("utf-8"),
            )

        except Exception as e:
            if self.verbose:
                print(f"❌ Error: {e}")
            error_msg = f"Sorry, I encountered an error: {str(e)}"
            try:
                await ctx.reply(
                    event,
                    type="assistant.error",
                    payload=error_msg.encode("utf-8"),
                )
            except Exception:
                pass


__all__ = ["BackendAgent"]
