"""Test script to diagnose data agent issues."""

import asyncio
import sys
import traceback


async def test_binance_api():
    """Test direct Binance API connection."""
    print("=" * 60)
    print("Test 1: Binance API Connection")
    print("=" * 60)

    try:
        import aiohttp
        print("✓ aiohttp is installed")
    except ImportError:
        print("✗ aiohttp not installed")
        return False

    try:
        async with aiohttp.ClientSession() as session:
            url = "https://api.binance.com/api/v3/ticker/24hr"
            params = {"symbol": "BTCUSDT"}

            print(f"Fetching: {url}?symbol=BTCUSDT")
            async with session.get(url, params=params, timeout=aiohttp.ClientTimeout(total=10)) as resp:
                print(f"Status: {resp.status}")

                if resp.status == 200:
                    data = await resp.json()
                    print(f"✓ Success! BTC Price: ${float(data['lastPrice']):,.2f}")
                    return True
                else:
                    text = await resp.text()
                    print(f"✗ API Error: {resp.status}")
                    print(f"Response: {text[:500]}")
                    return False

    except Exception as e:
        print(f"✗ Exception: {type(e).__name__}: {e}")
        traceback.print_exc()
        return False


async def test_bridge_connection():
    """Test connection to Loom bridge."""
    print("\n" + "=" * 60)
    print("Test 2: Bridge Connection")
    print("=" * 60)

    try:
        from loom.client import BridgeClient
        print("✓ loom.client imported")

        client = BridgeClient(address="127.0.0.1:50051")
        print(f"Connecting to bridge at 127.0.0.1:50051...")

        try:
            await asyncio.wait_for(client.connect(), timeout=5)
            print("✓ Connected to bridge")

            # Try to register a test agent
            print("Registering test agent...")
            await client.register_agent("test-agent", ["test.topic"], [])
            print("✓ Agent registered")

            await client.close()
            print("✓ Connection closed cleanly")
            return True

        except asyncio.TimeoutError:
            print("✗ Connection timeout - is the bridge running?")
            return False
        except Exception as e:
            print(f"✗ Connection failed: {type(e).__name__}: {e}")
            traceback.print_exc()
            return False

    except ImportError as e:
        print(f"✗ Import failed: {e}")
        traceback.print_exc()
        return False


async def test_agent_config():
    """Test loading agent configuration."""
    print("\n" + "=" * 60)
    print("Test 3: Configuration Loading")
    print("=" * 60)

    try:
        from loom import load_project_config
        print("✓ loom.load_project_config imported")

        config = load_project_config()
        print(f"✓ Config loaded")
        print(f"  Project: {config.name}")
        print(f"  Agents: {list(config.agents.keys())}")

        if "data-agent" in config.agents:
            agent_config = config.agents["data-agent"]
            print(f"  data-agent config: {agent_config}")
            return True
        else:
            print("✗ data-agent not found in config")
            return False

    except Exception as e:
        print(f"✗ Config loading failed: {type(e).__name__}: {e}")
        traceback.print_exc()
        return False


async def test_data_agent_startup():
    """Test data agent startup sequence."""
    print("\n" + "=" * 60)
    print("Test 4: Data Agent Startup")
    print("=" * 60)

    try:
        # Import data agent module
        from pathlib import Path
        sys.path.insert(0, str(Path(__file__).parent.resolve()))
        from agents import data
        print("✓ Data agent module imported")

        # Check if required functions exist
        if hasattr(data, 'main'):
            print("✓ main() function exists")
        else:
            print("✗ main() function not found")
            return False

        # Try to start the agent with a timeout
        print("Starting data agent (5 second timeout)...")
        try:
            await asyncio.wait_for(
                asyncio.create_task(data.main()),
                timeout=5
            )
        except asyncio.TimeoutError:
            print("✓ Agent started and ran for 5 seconds (normal behavior)")
            return True
        except Exception as e:
            print(f"✗ Agent failed during startup: {type(e).__name__}: {e}")
            traceback.print_exc()
            return False

    except ImportError as e:
        print(f"✗ Failed to import data agent: {e}")
        traceback.print_exc()
        return False


async def main():
    """Run all diagnostic tests."""
    print("\n" + "=" * 60)
    print("LOOM DATA AGENT DIAGNOSTICS")
    print("=" * 60 + "\n")

    results = []

    # Test 1: Binance API
    results.append(("Binance API", await test_binance_api()))

    # Test 2: Bridge Connection
    results.append(("Bridge Connection", await test_bridge_connection()))

    # Test 3: Configuration
    results.append(("Configuration", await test_agent_config()))

    # Test 4: Agent Startup (only if bridge is available)
    if results[1][1]:  # If bridge connection works
        results.append(("Agent Startup", await test_data_agent_startup()))

    # Summary
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    for name, success in results:
        status = "✓ PASS" if success else "✗ FAIL"
        print(f"{status:8s} - {name}")

    print("\n" + "=" * 60)
    if all(r[1] for r in results):
        print("All tests passed! The issue might be intermittent.")
    else:
        print("Some tests failed. See details above.")
    print("=" * 60)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\nTests interrupted by user")
    except Exception as e:
        print(f"\n\nFATAL ERROR: {e}")
        traceback.print_exc()
        sys.exit(1)
