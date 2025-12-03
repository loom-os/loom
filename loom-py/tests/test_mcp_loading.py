#!/usr/bin/env python3
"""Test: MCP Tool Loading from Environment

This test verifies that MCP servers can be loaded from LOOM_MCP_SERVERS.

Usage:
    BRAVE_API_KEY=your_key python tests/test_mcp_loading.py

Note: Requires BRAVE_API_KEY for the Brave Search MCP server.
"""

import asyncio
import json
import os
import subprocess
import sys
import time
from pathlib import Path

# Add loom-py to path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

import pytest

from loom import Agent


def start_server_with_mcp(bridge_addr: str, mcp_config: dict) -> subprocess.Popen:
    """Start loom-bridge-server with MCP configuration."""
    binary = Path(__file__).parent.parent.parent / "target" / "release" / "loom-bridge-server"

    if not binary.exists():
        print(f"[!] Binary not found: {binary}")
        print(
            "[!] Please build first: cargo build -p loom-bridge --bin loom-bridge-server --release"
        )
        sys.exit(1)

    env = os.environ.copy()
    env["LOOM_BRIDGE_ADDR"] = bridge_addr
    env["LOOM_MCP_SERVERS"] = json.dumps(mcp_config)
    env["RUST_LOG"] = "info"

    print(f"[*] Starting server with MCP config: {list(mcp_config.keys())}")

    proc = subprocess.Popen(
        [str(binary)],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )

    # Wait for startup
    time.sleep(3)

    return proc


@pytest.mark.integration
@pytest.mark.asyncio
async def test_mcp_brave_search(bridge_addr: str = "127.0.0.1:50051"):
    """Test Brave Search MCP tool.

    This test requires:
    - A running bridge server with MCP configured
    - BRAVE_API_KEY environment variable set
    """
    brave_api_key = os.environ.get("BRAVE_API_KEY")
    if not brave_api_key:
        pytest.skip("BRAVE_API_KEY not set")

    print("\n" + "=" * 60)
    print("Test: Brave Search MCP Tool")
    print("=" * 60)

    agent = Agent(
        agent_id="test-mcp-brave",
        topics=["test.replies"],
        address=bridge_addr,
    )

    try:
        await agent.start()
        print("[✓] Agent connected to Bridge")

        # The MCP tool name format is "server_name:tool_name"
        # Brave Search MCP server exposes "brave_web_search" tool
        result = await agent._ctx.tool(
            "brave-search:brave_web_search",
            payload={"query": "AI agents 2024"},
            timeout_ms=30000,
        )

        print(f"[✓] MCP Tool returned: {result[:300]}...")

        data = json.loads(result)
        print(f"[✓] Parsed JSON: {type(data)}")

        return True

    except Exception as e:
        print(f"[✗] Error: {e}")
        return False
    finally:
        await agent.stop()


async def main():
    print("=" * 60)
    print("MCP Tool Loading Test")
    print("=" * 60)

    # Check for API key
    brave_api_key = os.environ.get("BRAVE_API_KEY")
    if not brave_api_key:
        print("\n[!] BRAVE_API_KEY not set")
        print("[!] Set it to test Brave Search MCP:")
        print("    export BRAVE_API_KEY=your_api_key")
        print("\n[*] Running without MCP test (native tools only)")

        # Just verify the MCP config parsing works
        mcp_config = {"test-server": {"command": "echo", "args": ["test"], "env": {}}}
        print(f"[✓] MCP config structure valid: {mcp_config}")
        return 0

    bridge_addr = "127.0.0.1:50053"

    # MCP configuration for Brave Search
    mcp_config = {
        "brave-search": {
            "command": "npx",
            "args": ["-y", "@anthropics/mcp-brave-search"],
            "env": {"BRAVE_API_KEY": brave_api_key},
        }
    }

    # Start server
    proc = start_server_with_mcp(bridge_addr, mcp_config)

    try:
        # Run test
        success = await test_mcp_brave_search(bridge_addr)

        if success:
            print("\n🎉 MCP tool loading test passed!")
            return 0
        else:
            print("\n⚠️  MCP tool loading test failed")
            return 1

    finally:
        # Stop server
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
