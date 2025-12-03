#!/usr/bin/env python3
"""Chat Agent - Cognitive agent with tool use.

This agent provides:
- ReAct reasoning pattern (Thought -> Action -> Observation)
- Tool calling (weather, shell, file reading)
- Multi-turn conversation with memory

Run with:
    loom run           # Start runtime + this agent
    loom chat          # In another terminal, start chatting

Or for development:
    loom up            # Start runtime in one terminal
    python agents/chat.py   # Run agent directly
    loom chat          # Chat in another terminal
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

from loom import Agent


async def main():
    """Start the chat agent and wait for events."""
    print("[chat-agent] Starting...")

    # Create agent that listens for chat events
    agent = Agent(
        agent_id="chat-assistant",
        topics=["chat.input", "chat.replies"],
    )

    await agent.start()
    print(f"[chat-agent] Connected to Bridge at {agent._ctx.client.address}")
    print("[chat-agent] Ready. Use 'loom chat' to start chatting.")

    # Wait forever (agent handles events via gRPC streaming)
    try:
        while True:
            await asyncio.sleep(3600)  # Sleep for an hour at a time
    except asyncio.CancelledError:
        pass
    finally:
        await agent.stop()
        print("[chat-agent] Stopped.")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n[chat-agent] Interrupted.")
