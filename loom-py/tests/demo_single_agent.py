#!/usr/bin/env python3
"""Demo: Single Agent Research with CognitiveAgent

This demo shows a complete cognitive loop:
1. Agent receives a task (get weather info)
2. Uses ReAct pattern to reason and call tools
3. Synthesizes findings into a response

Requirements:
    - DEEPSEEK_API_KEY environment variable
    - loom-bridge-server binary built

Usage:
    export DEEPSEEK_API_KEY=your_key
    python demo_single_agent.py
"""

import asyncio
import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from loom import Agent, CognitiveAgent, CognitiveConfig, ThinkingStrategy
from loom.llm import LLMProvider


def start_server(bridge_addr: str) -> subprocess.Popen:
    """Start loom-bridge-server."""
    binary = Path(__file__).parent.parent.parent / "target" / "release" / "loom-bridge-server"

    if not binary.exists():
        print(f"❌ Binary not found: {binary}")
        print(
            "   Please build first: cargo build -p loom-bridge --bin loom-bridge-server --release"
        )
        sys.exit(1)

    env = os.environ.copy()
    env["LOOM_BRIDGE_ADDR"] = bridge_addr
    env["RUST_LOG"] = "warn"  # Quiet logs

    # Configure LLM provider for Core (uses DeepSeek if DEEPSEEK_API_KEY is set)
    if os.environ.get("DEEPSEEK_API_KEY"):
        env["VLLM_BASE_URL"] = "https://api.deepseek.com/v1"
        env["VLLM_MODEL"] = "deepseek-chat"
        env["VLLM_API_KEY"] = os.environ["DEEPSEEK_API_KEY"]

    print(f"🚀 Starting Loom Bridge on {bridge_addr}...")

    proc = subprocess.Popen(
        [str(binary)],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    time.sleep(2)
    return proc


async def demo_weather_assistant():
    """Demo: Weather assistant using CognitiveAgent."""
    print("\n" + "=" * 60)
    print("🌤️  Weather Assistant Demo")
    print("=" * 60)

    bridge_addr = "127.0.0.1:50060"
    proc = start_server(bridge_addr)

    try:
        # Create agent
        agent = Agent(
            agent_id="weather-assistant",
            topics=["weather.queries"],
            address=bridge_addr,
        )
        await agent.start()
        print("✅ Agent connected to Loom Bridge")

        # Create LLM provider
        llm = LLMProvider.from_name(agent._ctx, "deepseek")
        print("✅ LLM provider configured (DeepSeek)")

        # Create cognitive agent
        cognitive = CognitiveAgent(
            ctx=agent._ctx,
            llm=llm,
            config=CognitiveConfig(
                system_prompt="""You are a helpful weather assistant.
When asked about weather, use the weather:get tool to fetch current conditions.
Provide a friendly, concise summary of the weather.""",
                thinking_strategy=ThinkingStrategy.REACT,
                max_iterations=5,
                temperature=0.7,
            ),
            available_tools=["weather:get"],
        )
        print("✅ CognitiveAgent initialized (ReAct mode)")

        # Execute task
        print("\n📝 Task: What's the weather like in Tokyo and Paris?")
        print("-" * 60)

        result = await cognitive.run(
            "What's the current weather in Tokyo and Paris? Compare them briefly."
        )

        # Display results
        print("\n🤔 Reasoning Steps:")
        for step in result.steps:
            print(f"\n  Step {step.step}:")
            if step.reasoning:
                print(f"    💭 Thought: {step.reasoning[:100]}...")
            if step.tool_call:
                print(f"    🔧 Action: {step.tool_call.name}({step.tool_call.arguments})")
            if step.observation:
                status = "✅" if step.observation.success else "❌"
                output = (
                    step.observation.output[:80]
                    if step.observation.success
                    else step.observation.error
                )
                print(f"    {status} Observation: {output}...")

        print("\n" + "-" * 60)
        print("📊 Final Answer:")
        print(result.answer)
        print("-" * 60)

        print("\n📈 Stats:")
        print(f"   Iterations: {result.iterations}")
        print(f"   Total time: {result.total_latency_ms}ms")
        print(f"   Success: {'✅' if result.success else '❌'}")

        await agent.stop()
        return result.success

    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback

        traceback.print_exc()
        return False

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()


async def demo_simple_question():
    """Demo: Simple question without tools (SingleShot)."""
    print("\n" + "=" * 60)
    print("💬 Simple Q&A Demo (SingleShot)")
    print("=" * 60)

    bridge_addr = "127.0.0.1:50061"
    proc = start_server(bridge_addr)

    try:
        agent = Agent(
            agent_id="qa-assistant",
            topics=["qa.queries"],
            address=bridge_addr,
        )
        await agent.start()

        llm = LLMProvider.from_name(agent._ctx, "deepseek")

        cognitive = CognitiveAgent(
            ctx=agent._ctx,
            llm=llm,
            config=CognitiveConfig(
                system_prompt="You are a helpful assistant. Be concise and clear.",
                thinking_strategy=ThinkingStrategy.SINGLE_SHOT,
            ),
        )

        print("\n📝 Task: Explain what a cognitive loop is in AI agents.")
        print("-" * 60)

        result = await cognitive.run(
            "Explain what a cognitive loop is in AI agents, in 2-3 sentences."
        )

        print("\n📊 Answer:")
        print(result.answer)
        print("-" * 60)
        print(f"   Time: {result.total_latency_ms}ms")

        await agent.stop()
        return result.success

    except Exception as e:
        print(f"❌ Error: {e}")
        return False

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()


async def main():
    print("=" * 60)
    print("🧠 Loom CognitiveAgent Demo")
    print("=" * 60)

    # Check for API key
    if not os.environ.get("DEEPSEEK_API_KEY"):
        print("\n❌ DEEPSEEK_API_KEY not set!")
        print("   Please set it: export DEEPSEEK_API_KEY=your_api_key")
        sys.exit(1)

    results = []

    # Run demos
    results.append(("Simple Q&A", await demo_simple_question()))
    results.append(("Weather Assistant", await demo_weather_assistant()))

    # Summary
    print("\n" + "=" * 60)
    print("📋 Summary")
    print("=" * 60)

    passed = sum(1 for _, ok in results if ok)
    for name, ok in results:
        status = "✅" if ok else "❌"
        print(f"  {status} {name}")

    print(f"\n{passed}/{len(results)} demos completed successfully")

    return 0 if passed == len(results) else 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
