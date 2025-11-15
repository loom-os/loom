"""Output Agent - Receives processed data and displays it."""

import asyncio
import json

from loom import Agent


async def output_handler(ctx, topic: str, event) -> None:
    """Display processed data."""
    data = json.loads(event.payload.decode("utf-8"))

    print(f"[output] ✓ Received processed data #{data['counter']}")
    print(f"[output]   Original: {data['original_value']}")
    print(f"[output]   Processed: {data['processed_value']}")
    print(f"[output]   Trace ID: {event.trace_id or 'N/A'}")
    print()


async def main():
    # Telemetry is automatically initialized by the Agent class
    # Service name is derived from agent_id: "agent-output-agent"
    agent = Agent(
        agent_id="output-agent",
        topics=["processed.data"],
        on_event=output_handler,
    )

    print("[output] Starting output agent")
    print("[output] Subscribed to: processed.data")
    await agent.start()

    # Keep running
    try:
        await asyncio.Event().wait()
    except KeyboardInterrupt:
        print("[output] Shutting down...")
        await agent.stop()


if __name__ == "__main__":
    asyncio.run(main())
