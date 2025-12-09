"""Loom Chat CLI - Interactive chat with running agents.

This module provides terminal UI for chatting with cognitive agents.
It uses ChatClient from loom.streaming to connect to backend agents
and Rich for beautiful terminal output.

Architecture:
    CLI (this module) → ChatClient → Bridge → Backend Agent (CognitiveAgent)

The CLI is a lightweight client that:
1. Connects to a running backend agent via Bridge
2. Sends user messages and receives streaming responses
3. Renders output using Rich components

For standalone mode (no backend required), use StandaloneChatSession
which creates its own CognitiveAgent locally.
"""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Optional

from .ui import (
    ReactStreamRenderer,
    console,
    create_spinner,
    print_assistant_message,
    print_connection_status,
    print_divider,
    print_error,
    print_header,
    print_help,
    print_history,
    print_permission_request,
    print_stats,
    print_success,
    print_thinking_step,
    print_warning,
    print_welcome,
)

if TYPE_CHECKING:
    from ..cognitive import CognitiveAgent
    from ..streaming import StreamContentType


# ============================================================================
# Chat Session Modes
# ============================================================================


class ConnectedChatSession:
    """Chat session that connects to a backend agent via Bridge.

    This is the recommended mode when a backend agent is already running.
    The CLI acts as a thin client, sending messages and receiving streams.
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

    async def chat(self, message: str, on_chunk=None) -> dict:
        """Send a message and get response.

        Args:
            message: User message
            on_chunk: Optional callback for streaming chunks

        Returns:
            Response dict with 'content', 'stats', etc.
        """
        if not self._client:
            raise RuntimeError("Not connected")

        from ..streaming import StreamContentType

        # Collect thinking steps for verbose mode
        thinking_steps = []
        tool_calls = []
        content_parts = []

        async def chunk_handler(content: str, content_type: "StreamContentType"):
            if on_chunk:
                await on_chunk(content, content_type)

            if content_type == StreamContentType.TEXT:
                content_parts.append(content)
            elif content_type == StreamContentType.THINKING:
                thinking_steps.append(content)
            elif content_type == StreamContentType.TOOL_CALL:
                tool_calls.append(content)

        response = await self._client.chat(
            message,
            on_chunk=chunk_handler if self.streaming else None,
            include_thinking=self.verbose,
            include_tool_calls=True,
        )

        return {
            "content": response.content,
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


class StandaloneChatSession:
    """Standalone chat session with local CognitiveAgent.

    This mode creates its own CognitiveAgent locally, without requiring
    a separate backend agent process. Useful for development and testing.
    """

    def __init__(
        self,
        agent_id: str = "chat-assistant",
        bridge_addr: Optional[str] = None,
        verbose: bool = True,
        streaming: bool = True,
    ):
        self.agent_id = agent_id
        self.bridge_addr = bridge_addr
        self.verbose = verbose
        self.streaming = streaming

        self._agent = None
        self._cognitive: Optional[CognitiveAgent] = None
        self._conversation_history: list[dict] = []
        self._last_error: Optional[str] = None

    @property
    def connected(self) -> bool:
        return self._agent is not None and self._cognitive is not None

    async def connect(self) -> bool:
        """Initialize local agent and cognitive loop."""
        from .. import Agent, CognitiveAgent, CognitiveConfig, ThinkingStrategy
        from ..llm import LLMProvider
        from ..runtime.config import load_project_config

        try:
            # Load project config
            project_config = load_project_config(Path.cwd())

            # Use provided address or from config
            addr = self.bridge_addr or project_config.bridge.address

            # Create base agent
            self._agent = Agent(
                agent_id=self.agent_id,
                topics=["chat.input", "chat.replies"],
                address=addr,
            )
            await self._agent.start()

            # Create LLM provider
            llm = LLMProvider.from_config(
                self._agent._ctx,
                project_config.agents.get(self.agent_id, {}).get("llm_provider", "deepseek"),
                project_config,
            )

            # Determine thinking strategy
            strategy_name = project_config.agents.get(self.agent_id, {}).get(
                "thinking_strategy", "react"
            )
            strategy = {
                "react": ThinkingStrategy.REACT,
                "single_shot": ThinkingStrategy.SINGLE_SHOT,
                "chain_of_thought": ThinkingStrategy.CHAIN_OF_THOUGHT,
            }.get(strategy_name, ThinkingStrategy.REACT)

            max_iterations = project_config.agents.get(self.agent_id, {}).get("max_iterations", 10)

            # Create cognitive agent
            self._cognitive = CognitiveAgent(
                ctx=self._agent._ctx,
                llm=llm,
                config=CognitiveConfig(
                    system_prompt=self._get_system_prompt(),
                    thinking_strategy=strategy,
                    max_iterations=max_iterations,
                    temperature=0.7,
                ),
                available_tools=[
                    "weather:get",
                    "system:shell",
                    "fs:read_file",
                    "fs:write_file",
                    "fs:list_dir",
                    "fs:delete",
                    "web:search",
                ],
                permission_callback=self._request_permission,
            )

            return True
        except Exception as e:
            self._last_error = str(e)
            return False

    def _get_system_prompt(self) -> str:
        return """You are a helpful AI assistant with access to tools.

Available tools:
- weather:get: Get current weather. Args: {"location": "city name"}
- system:shell: Run shell commands. Args: {"command": "cmd"}
- fs:read_file: Read file contents. Args: {"path": "relative/path"}
- fs:write_file: Write content to file. Args: {"path": "relative/path", "content": "text"}
- fs:list_dir: List directory. Args: {"path": "relative/path"}
- fs:delete: Delete file or empty directory. Args: {"path": "relative/path"}
- web:search: Search the web. Args: {"query": "search terms", "limit": 5}

When you need information, use the appropriate tool.
Think step by step and explain your reasoning.
Be helpful, concise, and friendly."""

    def _request_permission(self, tool_name: str, args: dict, error_msg: str) -> bool:
        """Request user permission for a denied tool action."""
        return print_permission_request(tool_name, args, error_msg)

    async def disconnect(self):
        """Stop the agent."""
        if self._agent:
            await self._agent.stop()
            self._agent = None
            self._cognitive = None

    async def chat(self, message: str, on_chunk=None) -> dict:
        """Process a chat message."""
        from ..cognitive.types import CognitiveResult, ThoughtStep

        if not self._cognitive:
            raise RuntimeError("Not connected")

        # Add to history
        self._conversation_history.append({"role": "user", "content": message})

        # Build context
        context = []
        if len(self._conversation_history) > 1:
            for msg in self._conversation_history[-6:-1]:
                context.append(f"{msg['role'].capitalize()}: {msg['content']}")

        # Run cognitive loop
        thinking_steps = []
        tool_calls = []

        if self.streaming and on_chunk:
            # Streaming mode
            final_result = None

            async for item in self._cognitive.run_stream(
                message, context=context if context else None
            ):
                if isinstance(item, str):
                    # Import here to avoid circular
                    from ..streaming import StreamContentType

                    await on_chunk(item, StreamContentType.TEXT)
                elif isinstance(item, ThoughtStep):
                    thinking_steps.append(item)
                    if item.tool_call:
                        tool_calls.append(item.tool_call.name)
                elif isinstance(item, CognitiveResult):
                    final_result = item

            if final_result:
                self._conversation_history.append(
                    {"role": "assistant", "content": final_result.answer}
                )
                return {
                    "content": final_result.answer,
                    "thinking": thinking_steps,
                    "tool_calls": tool_calls,
                    "iterations": final_result.iterations,
                    "latency_ms": final_result.total_latency_ms,
                    "success": final_result.success,
                }
        else:
            # Non-streaming mode
            result = await self._cognitive.run(message, context=context if context else None)
            self._conversation_history.append({"role": "assistant", "content": result.answer})

            return {
                "content": result.answer,
                "thinking": result.steps,
                "tool_calls": [s.tool_call.name for s in result.steps if s.tool_call],
                "iterations": result.iterations,
                "latency_ms": result.total_latency_ms,
                "success": result.success,
            }

        return {"content": "", "success": False}

    def get_history(self) -> list[dict]:
        return self._conversation_history

    def clear_history(self):
        self._conversation_history = []
        if self._cognitive:
            self._cognitive.memory.clear()


# ============================================================================
# Unified Chat Session Factory
# ============================================================================


def create_chat_session(
    mode: str = "auto",
    agent_id: str = "chat-assistant",
    bridge_addr: Optional[str] = None,
    verbose: bool = True,
    streaming: bool = True,
) -> ConnectedChatSession | StandaloneChatSession:
    """Create a chat session based on mode.

    Args:
        mode: "connected" (use backend agent), "standalone" (local agent),
              or "auto" (try connected first, fallback to standalone)
        agent_id: Agent ID to connect to or create
        bridge_addr: Bridge address
        verbose: Show thinking steps
        streaming: Enable streaming output

    Returns:
        ChatSession instance
    """
    if mode == "connected":
        return ConnectedChatSession(
            backend_agent_id=agent_id,
            bridge_addr=bridge_addr,
            verbose=verbose,
            streaming=streaming,
        )
    elif mode == "standalone":
        return StandaloneChatSession(
            agent_id=agent_id,
            bridge_addr=bridge_addr,
            verbose=verbose,
            streaming=streaming,
        )
    else:  # auto
        # Try to detect if a backend is running by checking the bridge
        # For now, default to standalone which is more reliable
        # TODO: Probe bridge to detect running backend
        return StandaloneChatSession(
            agent_id=agent_id,
            bridge_addr=bridge_addr,
            verbose=verbose,
            streaming=streaming,
        )


# ============================================================================
# CLI Runner
# ============================================================================


async def run_chat_cli(
    bridge_addr: Optional[str] = None,
    agent_id: str = "chat-assistant",
    mode: str = "auto",
):
    """Run interactive CLI chat.

    Args:
        bridge_addr: Bridge address (default from config/env)
        agent_id: Agent ID to connect to
        mode: "connected", "standalone", or "auto"
    """
    # Print header
    print_header()
    print_welcome()

    # Create session
    session = create_chat_session(
        mode=mode,
        agent_id=agent_id,
        bridge_addr=bridge_addr,
    )

    # Connect
    with create_spinner("Connecting to Loom...") as progress:
        progress.add_task("Connecting...", total=None)
        connected = await session.connect()
        progress.stop()

    if not connected:
        # Show specific error if available
        if hasattr(session, "_last_error") and session._last_error:
            print_error(f"Connection failed: {session._last_error}")
        print_error(
            "Failed to connect to Loom runtime.\n"
            "Make sure Loom runtime is running (loom run or loom up)"
        )
        return 1

    print_success("Connected! Agent ready.")
    print_divider()

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
                    from ..cognitive.types import CognitiveResult, ThoughtStep

                    with ReactStreamRenderer(show_thinking=session.verbose) as renderer:
                        final_result = None
                        thinking_steps = []
                        tool_calls = []

                        async for item in session._cognitive.run_stream(
                            user_input,
                            context=(
                                [
                                    f"{m['role'].capitalize()}: {m['content']}"
                                    for m in session._conversation_history[-6:-1]
                                ]
                                if len(session._conversation_history) > 1
                                else None
                            ),
                        ):
                            if isinstance(item, str):
                                # Raw LLM chunk - parse and render
                                renderer.feed(item)

                            elif isinstance(item, ThoughtStep):
                                # Step completed (after tool execution)
                                thinking_steps.append(item)
                                if item.tool_call:
                                    tool_calls.append(item.tool_call.name)
                                    # Add observation to renderer
                                    if item.observation:
                                        renderer.add_observation(
                                            tool_name=item.tool_call.name,
                                            result=(
                                                item.observation.output
                                                if item.observation.success
                                                else item.observation.error or "Error"
                                            ),
                                            success=item.observation.success,
                                            offloaded=(
                                                item.reduced_step.outcome_ref
                                                if item.reduced_step
                                                else None
                                            ),
                                        )

                            elif isinstance(item, CognitiveResult):
                                final_result = item

                        # Flush any remaining content
                        renderer.flush()

                    # Update conversation history
                    if final_result:
                        session._conversation_history.append(
                            {"role": "assistant", "content": final_result.answer}
                        )

                    # Show stats
                    print_stats(
                        iterations=final_result.iterations if final_result else 0,
                        latency_ms=final_result.total_latency_ms if final_result else 0,
                        tool_calls=len(tool_calls),
                        success=final_result.success if final_result else False,
                    )
                else:
                    # Non-streaming mode
                    with create_spinner("Processing...") as progress:
                        progress.add_task("Thinking...", total=None)
                        result = await session.chat(user_input)
                        progress.stop()

                    # Show thinking steps if verbose
                    if session.verbose and result.get("thinking"):
                        for i, step in enumerate(result["thinking"], 1):
                            if hasattr(step, "reasoning"):
                                print_thinking_step(
                                    step_num=i,
                                    reasoning=step.reasoning,
                                    tool_name=step.tool_call.name if step.tool_call else None,
                                    tool_args=(
                                        step.tool_call.arguments if step.tool_call else None
                                    ),
                                    result=(
                                        step.observation.output
                                        if step.observation and step.observation.success
                                        else None
                                    ),
                                    error=(
                                        step.observation.error
                                        if step.observation and not step.observation.success
                                        else None
                                    ),
                                )

                    # Show result
                    print_assistant_message(result.get("content", ""))

                    # Show stats
                    print_stats(
                        iterations=result.get("iterations", 0),
                        latency_ms=result.get("latency_ms", 0),
                        tool_calls=len(result.get("tool_calls", [])),
                        success=result.get("success", True),
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

# Keep old ChatSession name for backward compatibility
ChatSession = StandaloneChatSession


def print_stream_step_complete(step) -> None:
    """Print a completed thinking step with context engineering support.

    This function displays tool execution results, handling both normal
    outputs and offloaded data references appropriately.

    Args:
        step: ThoughtStep object with tool_call and observation
    """
    from .ui import console

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
        console.print(part)
