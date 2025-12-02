#!/usr/bin/env python3
"""DeepResearch CLI - Send queries to the Lead Agent.

Usage:
    python query.py "What are the latest developments in AI agents?"
    python query.py --interactive
"""

import argparse
import asyncio
import json
import sys
import os

# Add parent to path for loom import
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "..", "loom-py", "src"))

from loom import Agent, load_project_config


async def send_query(query: str) -> None:
    """Send a research query to the Lead Agent.

    Args:
        query: Research query to send
    """
    # Create a minimal agent just to emit events
    agent = Agent(
        agent_id="query-cli",
        topics=["research.complete"],  # Listen for completion
        on_event=None,
    )

    print(f"[query] Connecting to Loom...")
    await agent.start()

    print(f"[query] Sending query: {query}")
    await agent._ctx.emit(
        "user.query",
        type="user.query",
        payload=json.dumps({"query": query}).encode("utf-8"),
    )

    print(f"[query] Query sent! Watch lead-agent output for progress.")
    print(f"[query] Report will be saved to workspace/reports/")

    # Give it a moment then exit
    await asyncio.sleep(1)
    await agent.stop()


async def interactive_mode() -> None:
    """Run in interactive mode - continuous query input."""
    print("═" * 60)
    print("DeepResearch Interactive Mode")
    print("═" * 60)
    print("Type your research queries. Type 'quit' to exit.\n")

    agent = Agent(
        agent_id="query-cli",
        topics=["research.complete"],
        on_event=None,
    )

    await agent.start()

    while True:
        try:
            query = input("\n[You] > ").strip()

            if not query:
                continue

            if query.lower() in ("quit", "exit", "q"):
                print("\n[query] Goodbye!")
                break

            print(f"[query] Sending: {query[:50]}...")
            await agent._ctx.emit(
                "user.query",
                type="user.query",
                payload=json.dumps({"query": query}).encode("utf-8"),
            )

            print("[query] Query sent! Check lead-agent for progress.")

        except KeyboardInterrupt:
            print("\n[query] Interrupted.")
            break
        except EOFError:
            print("\n[query] EOF received.")
            break

    await agent.stop()


def main():
    parser = argparse.ArgumentParser(
        description="Send research queries to DeepResearch Lead Agent"
    )
    parser.add_argument(
        "query",
        nargs="?",
        help="Research query to send",
    )
    parser.add_argument(
        "-i", "--interactive",
        action="store_true",
        help="Run in interactive mode",
    )

    args = parser.parse_args()

    if args.interactive:
        asyncio.run(interactive_mode())
    elif args.query:
        asyncio.run(send_query(args.query))
    else:
        parser.print_help()
        print("\nExamples:")
        print('  python query.py "What are the latest developments in AI agents?"')
        print('  python query.py --interactive')


if __name__ == "__main__":
    main()
