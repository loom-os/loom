#!/usr/bin/env python3
"""Quick test for native tool calls through Bridge.

This script tests the tool call chain WITHOUT needing MCP setup.
Tests native tools that are registered by Core at startup.

Usage:
    1. Start Core+Bridge in one terminal:
       cd /home/jared/loom && cargo run -p bridge --bin server

    2. Run this test in another terminal:
       cd /home/jared/loom/loom-py && python tests/test_native_tools.py
"""

import asyncio
import json
import os
import sys
from pathlib import Path

# Add loom-py to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from loom import Agent


async def test_weather_tool():
    """Test the weather:get native tool."""
    print("\n" + "=" * 60)
    print("Test: weather:get tool")
    print("=" * 60)

    agent = Agent(
        agent_id="test-weather",
        topics=["test.replies"],
        address=os.environ.get("LOOM_BRIDGE_ADDR", "127.0.0.1:50051"),
    )

    try:
        await agent.start()
        print("[✓] Agent connected to Bridge")

        result = await agent._ctx.tool(
            "weather:get",
            payload={"location": "Tokyo"},
            timeout_ms=10000,
        )

        print(f"[✓] Tool returned: {result[:200]}...")

        data = json.loads(result)
        print(f"[✓] Parsed JSON with keys: {list(data.keys())}")

        return True

    except Exception as e:
        print(f"[✗] Error: {e}")
        return False
    finally:
        await agent.stop()


async def test_web_search_tool():
    """Test the web:search native tool (DuckDuckGo)."""
    print("\n" + "=" * 60)
    print("Test: web:search tool")
    print("=" * 60)

    agent = Agent(
        agent_id="test-websearch",
        topics=["test.replies"],
        address=os.environ.get("LOOM_BRIDGE_ADDR", "127.0.0.1:50051"),
    )

    try:
        await agent.start()
        print("[✓] Agent connected to Bridge")

        result = await agent._ctx.tool(
            "web:search",
            payload={"query": "AI agents"},
            timeout_ms=15000,
        )

        print(f"[✓] Tool returned: {result[:300]}...")

        data = json.loads(result)
        print(f"[✓] Parsed JSON: {type(data)}")

        if isinstance(data, dict):
            print(f"    Keys: {list(data.keys())}")
        elif isinstance(data, list):
            print(f"    List with {len(data)} items")

        return True

    except Exception as e:
        print(f"[✗] Error: {e}")
        return False
    finally:
        await agent.stop()


async def test_shell_tool():
    """Test the system:shell native tool."""
    print("\n" + "=" * 60)
    print("Test: system:shell tool")
    print("=" * 60)

    agent = Agent(
        agent_id="test-shell",
        topics=["test.replies"],
        address=os.environ.get("LOOM_BRIDGE_ADDR", "127.0.0.1:50051"),
    )

    try:
        await agent.start()
        print("[✓] Agent connected to Bridge")

        result = await agent._ctx.tool(
            "system:shell",
            payload={"command": "echo", "args": ["Hello", "from", "Loom"]},
            timeout_ms=5000,
        )

        print(f"[✓] Tool returned: {result}")

        data = json.loads(result)
        print(f"[✓] Parsed JSON with keys: {list(data.keys())}")

        return True

    except Exception as e:
        print(f"[✗] Error: {e}")
        return False
    finally:
        await agent.stop()


async def test_fs_read_tool():
    """Test the fs:read_file native tool."""
    print("\n" + "=" * 60)
    print("Test: fs:read_file tool")
    print("=" * 60)

    agent = Agent(
        agent_id="test-fs",
        topics=["test.replies"],
        address=os.environ.get("LOOM_BRIDGE_ADDR", "127.0.0.1:50051"),
    )

    try:
        await agent.start()
        print("[✓] Agent connected to Bridge")

        # Try to read README.md (should exist in workspace root)
        result = await agent._ctx.tool(
            "fs:read_file",
            payload={"path": "README.md"},
            timeout_ms=5000,
        )

        print(f"[✓] Tool returned: {result[:200]}...")

        data = json.loads(result)
        if "content" in data:
            content = data["content"]
            print(f"[✓] Read {len(content)} characters from README.md")
            print(f"    First line: {content.split(chr(10))[0][:60]}...")

        return True

    except Exception as e:
        print(f"[✗] Error: {e}")
        # This might fail if README.md doesn't exist - that's OK
        if "not found" in str(e).lower():
            print("    (File not found is expected if README.md missing)")
        return False
    finally:
        await agent.stop()


async def test_tool_not_found():
    """Test that non-existent tool returns proper error."""
    print("\n" + "=" * 60)
    print("Test: Non-existent tool error handling")
    print("=" * 60)

    agent = Agent(
        agent_id="test-notfound",
        topics=["test.replies"],
        address=os.environ.get("LOOM_BRIDGE_ADDR", "127.0.0.1:50051"),
    )

    try:
        await agent.start()
        print("[✓] Agent connected to Bridge")

        try:
            await agent._ctx.tool(
                "nonexistent:tool",
                payload={},
                timeout_ms=5000,
            )
            print("[✗] Should have raised an error!")
            return False
        except RuntimeError as e:
            print(f"[✓] Got expected error: {e}")
            if "not found" in str(e).lower():
                print("[✓] Error message contains 'not found'")
                return True
            return False

    except Exception as e:
        print(f"[✗] Unexpected error: {e}")
        return False
    finally:
        await agent.stop()


async def main():
    print("=" * 60)
    print("Native Tool Call Chain Test")
    print("=" * 60)
    print("\nPrerequisites:")
    print("  1. Core+Bridge must be running:")
    print("     cargo run -p bridge --bin server")
    print("\n  Or using loom CLI:")
    print("     cd demo/deep-research && loom run")
    print("=" * 60)

    results = {}

    # Run tests
    results["weather:get"] = await test_weather_tool()
    results["web:search"] = await test_web_search_tool()
    results["system:shell"] = await test_shell_tool()
    results["fs:read_file"] = await test_fs_read_tool()
    results["error_handling"] = await test_tool_not_found()

    # Summary
    print("\n" + "=" * 60)
    print("Test Summary")
    print("=" * 60)

    passed = sum(1 for v in results.values() if v)
    total = len(results)

    for name, passed_test in results.items():
        status = "✓ PASS" if passed_test else "✗ FAIL"
        print(f"  {status}: {name}")

    print(f"\nTotal: {passed}/{total} tests passed")

    if passed == total:
        print("\n🎉 All native tool tests passed!")
        return 0
    else:
        print("\n⚠️  Some tests failed. Check if Core+Bridge is running.")
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
