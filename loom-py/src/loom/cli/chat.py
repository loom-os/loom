"""Loom Chat CLI - Interactive chat with cognitive agents.

This module provides terminal UI for chatting with cognitive agents.
It uses ChatClient from loom.streaming to connect to backend agents
and Rich for beautiful terminal output.

Architecture:
    CLI (this module) → ChatClient → Bridge → Backend Agent (CognitiveAgent)

The CLI is a lightweight client that:
1. Connects to a running backend agent via Bridge
2. Sends user messages and receives streaming responses
3. Renders output using Rich components

Usage:
    loom chat              # Connect to running backend (requires loom run)
    loom chat --help       # Show help
"""

from __future__ import annotations

import time
from typing import Optional

from opentelemetry import trace

from ..streaming.types import StreamStatus
from .ui import (
    ReactStreamRenderer,
    console,
    create_spinner,
    print_assistant_message,
    print_connection_status,
    print_error,
    print_header,
    print_help,
    print_history,
    print_stats,
    print_success,
    print_warning,
    print_welcome,
)

# Tracer for CLI spans
tracer = trace.get_tracer(__name__)

# ============================================================================
# Chat Session
# ============================================================================


class ChatSession:
    """Chat session that connects to a backend agent via Bridge.

    This connects to a running backend agent (started via `loom run`)
    and communicates via the event bus with streaming support.
    """

    def __init__(
        self,
        backend_agent_id: str = "chat-assistant",
        backend_topic: str = "chat.input",
        bridge_addr: Optional[str] = None,
        verbose: bool = True,
        streaming: bool = True,
    ):
        self.backend_agent_id = backend_agent_id
        self.backend_topic = backend_topic
        self.bridge_addr = bridge_addr
        self.verbose = verbose
        self.streaming = streaming

        self._client = None
        self._connected = False
        self._last_error: Optional[str] = None

    @property
    def connected(self) -> bool:
        return self._connected and self._client is not None

    async def connect(self) -> bool:
        """Connect to the backend agent via Bridge."""
        from ..streaming import ChatClient

        try:
            self._client = ChatClient(
                backend_topic=self.backend_topic,
                bridge_addr=self.bridge_addr,
            )
            await self._client.connect()
            self._connected = True
            return True
        except Exception as e:
            self._last_error = str(e)
            self._connected = False
            return False

    async def disconnect(self):
        """Disconnect from Bridge."""
        if self._client:
            await self._client.disconnect()
        self._connected = False

    async def chat(self, message: str) -> dict:
        """Send a message and get streaming response.

        Args:
            message: User message

        Returns:
            Response dict with 'content', 'stats', etc.
        """
        if not self._client:
            raise RuntimeError("Not connected")

        from ..streaming import StreamContentType

        # Create tracing span for CLI chat interaction
        with tracer.start_as_current_span(
            "cli.chat",
            attributes={
                "cli.backend_agent_id": self.backend_agent_id,
                "cli.message_length": len(message),
                "cli.streaming": self.streaming,
                "cli.verbose": self.verbose,
            },
        ) as span:
            start_time = time.time()

            # Collect response parts
            thinking_steps: list[str] = []
            tool_calls: list[str] = []
            content_parts: list[str] = []

            async def chunk_handler(content: str, content_type: StreamContentType):
                if content_type == StreamContentType.TEXT:
                    content_parts.append(content)
                elif content_type == StreamContentType.THINKING:
                    thinking_steps.append(content)
                elif content_type == StreamContentType.TOOL_CALL:
                    tool_calls.append(content)

            try:
                response = await self._client.chat(
                    message,
                    on_chunk=chunk_handler if self.streaming else None,
                    include_thinking=self.verbose,
                    include_tool_calls=True,
                )

                # Record metrics
                duration_ms = int((time.time() - start_time) * 1000)
                span.set_attribute("cli.response_length", len(response.content or ""))
                span.set_attribute("cli.thinking_steps", len(thinking_steps))
                span.set_attribute("cli.tool_calls", len(tool_calls))
                span.set_attribute("cli.duration_ms", duration_ms)
                if response.stats:
                    span.set_attribute("cli.total_tokens", response.stats.total_tokens)

            except Exception as e:
                span.record_exception(e)
                raise

            return {
                "content": response.content or "".join(content_parts),
                "thinking": thinking_steps,
                "tool_calls": tool_calls,
                "stats": response.stats,
                "status": response.status,
            }

    def get_history(self) -> list[dict]:
        """Get conversation history."""
        if self._client:
            return [
                {"role": msg.role, "content": msg.content}
                for msg in self._client.get_thread_history()
            ]
        return []

    def clear_history(self):
        """Clear conversation history."""
        if self._client:
            self._client.clear_thread()


# ============================================================================
# CLI Runner
# ============================================================================


async def run_chat_cli(
    bridge_addr: Optional[str] = None,
    agent_id: str = "chat-assistant",
):
    """Run interactive CLI chat.

    Args:
        bridge_addr: Bridge address (default from config/env)
        agent_id: Agent ID to connect to
    """
    # Print header
    print_header()
    print_welcome()

    # Create session
    session = ChatSession(
        backend_agent_id=agent_id,
        bridge_addr=bridge_addr,
    )

    # Connect
    with create_spinner("Connecting to Loom...") as progress:
        progress.add_task("Connecting...", total=None)
        connected = await session.connect()
        progress.stop()

    if not connected:
        error_msg = session._last_error or "Unknown error"
        print_error(
            f"Failed to connect to Loom runtime.\n\n"
            f"Error: {error_msg}\n\n"
            f"Make sure Loom runtime is running:\n"
            f"  cd apps/chat-assistant && loom run"
        )
        return 1

    print_success(f"Connected to backend agent '{agent_id}'")
    console.print()

    try:
        while True:
            # Get user input
            try:
                user_input = console.input("[user]You ▶[/user] ").strip()
            except EOFError:
                break
            except KeyboardInterrupt:
                print_warning("Use /quit to exit")
                continue

            if not user_input:
                continue

            # Handle commands
            if user_input.startswith("/"):
                cmd_parts = user_input.lower().split()
                cmd = cmd_parts[0]

                if cmd in ["/quit", "/exit", "/q"]:
                    console.print("\n[cyan]Goodbye! 👋[/cyan]\n")
                    break

                if cmd == "/clear":
                    session.clear_history()
                    print_success("Conversation cleared.")
                    continue

                if cmd == "/help":
                    print_help()
                    continue

                if cmd == "/verbose":
                    session.verbose = not session.verbose
                    state = "ON" if session.verbose else "OFF"
                    print_success(f"Verbose mode: {state}")
                    continue

                if cmd == "/history":
                    print_history(session.get_history())
                    continue

                if cmd == "/stream":
                    session.streaming = not session.streaming
                    state = "ON" if session.streaming else "OFF"
                    print_success(f"Streaming mode: {state}")
                    continue

                if cmd == "/status":
                    print_connection_status(
                        connected=session.connected,
                        agent_id=agent_id,
                        bridge_addr=bridge_addr or "default",
                    )
                    continue

                print_error(f"Unknown command: {cmd}. Type /help for available commands.")
                continue

            # Process message
            try:
                if session.streaming:
                    # Streaming mode with ReactStreamRenderer
                    from ..streaming import StreamContentType

                    with ReactStreamRenderer(show_thinking=session.verbose) as renderer:
                        thinking_steps: list[str] = []
                        tool_calls: list[str] = []

                        async def on_chunk(
                            content: str,
                            content_type: StreamContentType,
                            _thinking_steps: list[str] = thinking_steps,
                            _tool_calls: list[str] = tool_calls,
                        ):
                            if content_type == StreamContentType.TEXT:
                                renderer.feed(content)
                            elif content_type == StreamContentType.THINKING:
                                _thinking_steps.append(content)
                                renderer.feed(f"\nThought: {content}\n")
                            elif content_type == StreamContentType.TOOL_CALL:
                                _tool_calls.append(content)
                                # Parse tool call: "tool_name: {args}"
                                if ": " in content:
                                    tool_name, args = content.split(": ", 1)
                                    renderer.current_action = {"tool": tool_name, "args": args}
                            elif content_type == StreamContentType.TOOL_RESULT:
                                # Tool result received
                                if renderer.current_action:
                                    renderer.add_observation(
                                        tool_name=renderer.current_action.get("tool", "unknown"),
                                        result=content,
                                        success=True,
                                    )

                        response = await session._client.chat(
                            user_input,
                            on_chunk=on_chunk,
                            include_thinking=session.verbose,
                            include_tool_calls=True,
                        )

                        renderer.flush()

                    # Show stats
                    if response.stats:
                        print_stats(
                            iterations=0,
                            latency_ms=response.stats.duration_ms,
                            tool_calls=len(tool_calls),
                            success=response.status == StreamStatus.OK,
                        )
                else:
                    # Non-streaming mode
                    with create_spinner("Processing...") as progress:
                        progress.add_task("Thinking...", total=None)
                        result = await session.chat(user_input)
                        progress.stop()

                    # Show result
                    if result.get("content"):
                        print_assistant_message(result["content"])

                    # Show stats
                    stats = result.get("stats")
                    print_stats(
                        iterations=0,
                        latency_ms=stats.duration_ms if stats else 0,
                        tool_calls=len(result.get("tool_calls", [])),
                        success=True,
                    )

                console.print()

            except Exception as e:
                print_error(str(e))
                import traceback

                console.print(f"[dim]{traceback.format_exc()}[/dim]")

    except KeyboardInterrupt:
        console.print("\n[cyan]Goodbye! 👋[/cyan]\n")
    finally:
        await session.disconnect()

    return 0


# ============================================================================
# Legacy Compatibility
# ============================================================================


def print_stream_step_complete(step) -> None:
    """Print a completed thinking step with context engineering support.

    This function displays tool execution results, handling both normal
    outputs and offloaded data references appropriately.

    Args:
        step: ThoughtStep object with tool_call and observation
    """
    from .ui import console as ui_console

    if not step.tool_call:
        return

    # Build output parts
    parts = []

    # Step header
    parts.append(f"[bold magenta]Step {step.step}[/bold magenta]")

    # Reasoning
    if step.reasoning:
        parts.append(f"  [dim]💭 {step.reasoning}[/dim]")

    # Tool info
    parts.append(f"  [cyan]🔧 Tool: {step.tool_call.name}[/cyan]")

    # Observation/Result
    if step.observation:
        if step.observation.success:
            # Check for offloaded data
            if step.reduced_step and step.reduced_step.outcome_ref:
                parts.append("  [green]✅ Data offloaded[/green]")
                parts.append(f"     Offloaded to: {step.reduced_step.outcome_ref}")
                # Show summary from reduced step
                if step.reduced_step.observation:
                    parts.append(f"     Summary: {step.reduced_step.observation}")
                parts.append(f"     View with: cat {step.reduced_step.outcome_ref}")
            else:
                # Normal output
                output = step.observation.output
                if len(output) > 200:
                    output = output[:197] + "..."
                parts.append("  [green]✅ Result:[/green]")
                for line in output.split("\n"):
                    parts.append(f"     {line}")
        else:
            # Error
            parts.append(f"  [red]❌ Error: {step.observation.error}[/red]")

    # Print all parts
    for part in parts:
        ui_console.print(part)
