#!/usr/bin/env python3
"""Chat Agent Backend - Cognitive AI service.

This is the backend service that provides AI capabilities for chat interactions.

Architecture:
    - Backend Service: This file (runs continuously)
    - Frontend Client: loom chat CLI (user interface)

Communication Flow:
    User (loom chat) → chat.input topic → This Agent → CognitiveAgent
        → AI Processing (ReAct) → chat.replies topic → User sees response

Features:
    - Loads configuration from loom.toml
    - Creates CognitiveAgent with LLM reasoning
    - Listens for user messages on chat.input
    - Processes with ReAct loop and tool calling
    - Sends responses to chat.replies

Run with:
    loom run           # Start runtime + this backend agent
    loom chat          # In another terminal, start client UI

Or for development:
    loom up            # Start runtime in one terminal
    python agents/chat.py   # Run this backend
    loom chat          # Start client in another terminal
"""

import asyncio
import os
import sys
from pathlib import Path


# Load .env from parent directory
def _load_dotenv():
    for env_path in [Path(__file__).parent.parent / ".env", Path(".env")]:
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, _, value = line.partition("=")
                    key = key.strip()
                    value = value.strip().strip("'\"")
                    if key and key not in os.environ:
                        os.environ[key] = value
            break


_load_dotenv()

# Add loom-py to path for local development
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent / "loom-py" / "src"))

from loom import Agent, CognitiveAgent, CognitiveConfig, LLMProvider
from loom.runtime.config import load_project_config


async def main():
    """Start the chat backend agent."""
    print("=" * 70)
    print("🧠 Chat Backend Agent Starting...")
    print("=" * 70)

    # Load project configuration
    project_dir = Path(__file__).parent.parent
    print(f"\n📁 Project Directory: {project_dir}")
    print(f"📄 Config File: {project_dir / 'loom.toml'}")

    try:
        config = load_project_config(project_dir)
        print(f"✅ Configuration loaded successfully")
    except Exception as e:
        print(f"❌ Failed to load configuration: {e}")
        print(f"   Make sure {project_dir / 'loom.toml'} exists")
        return

    # Get agent-specific configuration
    agent_config = config.agents.get("chat-assistant", {})
    if not agent_config:
        print("⚠️  No 'chat-assistant' config in loom.toml, using defaults")
        agent_config = {
            "llm_provider": "deepseek",
            "thinking_strategy": "react",
            "max_iterations": 20,
            "tools": [],
        }

    print(f"\n📋 Agent Configuration:")
    print(f"   LLM Provider: {agent_config.get('llm_provider', 'deepseek')}")
    print(f"   Thinking Strategy: {agent_config.get('thinking_strategy', 'react')}")
    print(f"   Max Iterations: {agent_config.get('max_iterations', 20)}")
    tools = agent_config.get('tools', [])
    print(f"   Tools: {len(tools)} configured")
    if tools:
        print(f"          {', '.join(tools[:5])}{'...' if len(tools) > 5 else ''}")

    # Create base Agent for event bus communication
    print(f"\n🔌 Connecting to Bridge...")
    agent = Agent(
        agent_id="chat-assistant",
        topics=["chat.input"],  # Backend only listens to input
    )

    try:
        await agent.start()
        print(f"✅ Connected to Bridge at {agent._ctx.client.address}")
    except Exception as e:
        print(f"❌ Failed to connect to Bridge: {e}")
        print(f"   Make sure runtime is running (loom up or loom run)")
        return

    # Create CognitiveAgent for LLM reasoning
    print(f"\n🤖 Initializing CognitiveAgent...")
    llm_provider = agent_config.get("llm_provider", "deepseek")

    try:
        llm = LLMProvider.from_config(agent.ctx, llm_provider, config)
        print(f"✅ LLM Provider '{llm_provider}' initialized")
    except Exception as e:
        print(f"❌ Failed to initialize LLM: {e}")
        print(f"   Check your loom.toml [llm.{llm_provider}] section")
        print(f"   and ensure API keys are set in .env")
        await agent.stop()
        return

    cognitive = CognitiveAgent(
        ctx=agent.ctx,
        llm=llm,
        config=CognitiveConfig(
            system_prompt=agent_config.get(
                "system_prompt",
                "You are a helpful AI assistant with access to tools. "
                "Use ReAct reasoning to solve problems step by step. "
                "Be helpful, concise, and friendly."
            ),
            thinking_strategy=agent_config.get("thinking_strategy", "react"),
            max_iterations=agent_config.get("max_iterations", 20),
        ),
        available_tools=agent_config.get("tools", []),
    )
    print(f"✅ CognitiveAgent initialized")
    print(f"   Strategy: {cognitive.config.thinking_strategy}")
    print(f"   Max Iterations: {cognitive.config.max_iterations}")

    # Set up event handler - this is where the magic happens!
    print(f"\n📡 Setting up event handler...")

    # Track active streams for cancellation
    active_streams: dict[str, "StreamingHandler"] = {}

    async def on_event(ctx, topic, event):
        """Handle incoming events with cognitive processing.

        This is the core backend logic:
        1. Receive user message from chat.input
        2. Process with CognitiveAgent (ReAct loop) with streaming
        3. Send streaming chunks back to client
        4. Handle cancellation requests
        """
        from loom.cognitive.types import CognitiveResult, ThoughtStep
        from loom.streaming import StreamCancelledError, StreamContentType, StreamingHandler

        # Handle stream cancellation
        if topic == "chat.input" and event.type == "stream.cancel":
            correlation_id = event.correlation_id
            if correlation_id and correlation_id in active_streams:
                handler = active_streams[correlation_id]
                reason = event.payload.decode("utf-8") if event.payload else "User cancelled"
                handler.cancel(reason)
                print(f"⚠️  Stream {correlation_id[:8]}... cancelled: {reason}")
            return

        # Handle stream requests (new streaming protocol)
        if topic == "chat.input" and event.type == "stream.request":
            handler = StreamingHandler(ctx, event)

            # Register handler for cancellation
            active_streams[handler.correlation_id] = handler

            try:
                # Decode user message
                user_input = event.payload.decode("utf-8")
                print(f"\n{'─' * 70}")
                print(f"📨 [Stream] Received from {event.sender}")
                print(f"   Message: {user_input[:100]}{'...' if len(user_input) > 100 else ''}")

                # Stream cognitive processing
                print(f"🧠 Processing with streaming...")

                async for item in cognitive.run_stream(user_input):
                    if isinstance(item, str):
                        # Raw LLM text chunk
                        await handler.send_chunk(item, StreamContentType.TEXT)

                    elif isinstance(item, ThoughtStep):
                        # Completed step with tool call
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
                                    result_text[:500],  # Truncate for streaming
                                )

                    elif isinstance(item, CognitiveResult):
                        # Final result - send complete
                        print(f"✅ Stream complete: {item.iterations} iterations")
                        await handler.send_complete(
                            final_content=item.answer,
                            tokens=item.total_tokens,
                        )
                        break

                print(f"{'─' * 70}\n")

            except StreamCancelledError as e:
                print(f"⚠️  Stream cancelled: {e.reason}")
                await handler.send_cancelled(e.reason)

            except Exception as e:
                print(f"❌ Stream error: {e}")
                import traceback
                traceback.print_exc()
                await handler.send_error(str(e))

            finally:
                # Cleanup handler registration
                active_streams.pop(handler.correlation_id, None)

        # Handle legacy non-streaming requests
        elif topic == "chat.input" and event.type == "user.message":
            try:
                user_input = event.payload.decode("utf-8")
                print(f"\n{'─' * 70}")
                print(f"📨 [Legacy] Received from {event.sender}")
                print(f"   Message: {user_input[:100]}{'...' if len(user_input) > 100 else ''}")

                # Run non-streaming cognitive processing
                result = await cognitive.run(user_input)

                print(f"✅ Complete: {result.iterations} iterations")

                # Send response back
                await ctx.reply(
                    event,
                    type="assistant.message",
                    payload=result.answer.encode("utf-8"),
                )
                print(f"{'─' * 70}\n")

            except Exception as e:
                print(f"❌ Error: {e}")
                import traceback
                traceback.print_exc()
                error_msg = f"Sorry, I encountered an error: {str(e)}"
                try:
                    await ctx.reply(
                        event,
                        type="assistant.error",
                        payload=error_msg.encode("utf-8"),
                    )
                except Exception:
                    pass

    # Attach event handler to agent
    agent._on_event = on_event
    print(f"✅ Event handler attached")

    # Ready!
    print("\n" + "=" * 70)
    print("✨ Chat Backend Agent Ready!")
    print("=" * 70)
    print(f"Backend ID: chat-assistant")
    print(f"Listening on: chat.input")
    print(f"Replying to: chat.replies (via correlation)")
    print()
    print("The backend is now waiting for messages from clients.")
    print("Start a client with: loom chat")
    print()
    print("Press Ctrl+C to stop.")
    print("=" * 70 + "\n")

    # Wait forever (agent handles events via gRPC streaming)
    try:
        while True:
            await asyncio.sleep(3600)  # Sleep for an hour at a time
    except asyncio.CancelledError:
        pass
    finally:
        print("\n[Backend] Shutting down...")
        await agent.stop()
        print("[Backend] Stopped.")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n[Backend] Interrupted by user.")
