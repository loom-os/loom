#!/usr/bin/env python3.12
"""Test script to diagnose agent connection issues."""

import asyncio
import sys

async def test_connection():
    """Test agent connection to bridge."""
    from loom import Agent

    print("=" * 60)
    print("Testing Loom Agent Connection")
    print("=" * 60)

    agent = Agent(
        agent_id="test-connection-agent",
        topics=["test.topic"],
        on_event=None,
    )

    print("\n[1/3] Connecting to bridge at 127.0.0.1:50051...")
    try:
        await agent.client.connect()
        print("✓ Connected successfully!")
    except Exception as e:
        print(f"✗ Connection failed: {e}")
        return False

    print("\n[2/3] Registering agent...")
    try:
        await agent.client.register_agent(agent.agent_id, agent.topics, [])
        print("✓ Registration successful!")
    except Exception as e:
        print(f"✗ Registration failed: {e}")
        await agent.client.close()
        return False

    print("\n[3/3] Starting event stream...")
    try:
        async def outbound_iter():
            while True:
                msg = await agent._outbound_queue.get()
                yield msg

        stream = agent.client.event_stream(agent.agent_id, outbound_iter())
        print(f"✓ Stream created: {type(stream)}")

        # Try to receive one message with timeout
        print("  Waiting for first message from stream (5s timeout)...")
        try:
            first_msg = await asyncio.wait_for(stream.__anext__(), timeout=5.0)
            print(f"✓ Received first message: {type(first_msg)}")
        except asyncio.TimeoutError:
            print("✗ Timeout waiting for first message")
            await agent.client.close()
            return False

    except Exception as e:
        print(f"✗ Stream creation failed: {e}")
        import traceback
        traceback.print_exc()
        await agent.client.close()
        return False

    print("\n" + "=" * 60)
    print("SUCCESS: All connection tests passed!")
    print("=" * 60)

    await agent.client.close()
    return True

if __name__ == "__main__":
    result = asyncio.run(test_connection())
    sys.exit(0 if result else 1)
