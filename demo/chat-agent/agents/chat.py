#!/usr/bin/env python3
"""Chat Agent - Interactive cognitive agent with tool use.

This agent provides an interactive chat interface with:
- ReAct reasoning pattern (Thought -> Action -> Observation)
- Tool calling (weather, shell, file reading)
- Multi-turn conversation with memory
- Real-time display of thinking process

Run with: loom run
Or directly: python agents/chat.py
"""

import asyncio
import json
import os
import sys
from pathlib import Path

# Load .env from parent directory
def _load_dotenv():
    for env_path in [Path(__file__).parent.parent / ".env", Path(".env")]:
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                line = line.strip()
                if line and not line.startswith('#') and '=' in line:
                    key, _, value = line.partition('=')
                    key = key.strip()
                    value = value.strip().strip("'\"")
                    if key and key not in os.environ:
                        os.environ[key] = value
            break

_load_dotenv()

# Add loom-py to path for local development
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent / "loom-py" / "src"))

from loom import Agent, CognitiveAgent, CognitiveConfig, ThinkingStrategy
from loom.config import load_project_config
from loom.llm import LLMProvider


class ChatAgent:
    """Interactive chat agent with cognitive loop."""

    def __init__(self, config_path: str = None):
        self.config_path = Path(config_path) if config_path else None
        self.agent = None
        self.cognitive = None
        self.conversation_history = []

    async def start(self, bridge_addr: str = None):
        """Initialize and start the agent."""
        # Load project config
        start_dir = self.config_path.parent if self.config_path else Path(".")
        project_config = load_project_config(start_dir)

        # Use provided address or from config
        addr = bridge_addr or project_config.bridge.address

        # Create base agent
        self.agent = Agent(
            agent_id="chat-assistant",
            topics=["chat.input", "chat.replies"],
            address=addr,
        )
        await self.agent.start()

        # Create LLM provider from config
        llm = LLMProvider.from_config(
            self.agent._ctx,
            project_config.agents.get("chat-assistant", {}).get("llm_provider", "deepseek"),
            project_config,
        )

        # Determine thinking strategy
        strategy_name = project_config.agents.get("chat-assistant", {}).get(
            "thinking_strategy", "react"
        )
        strategy = {
            "react": ThinkingStrategy.REACT,
            "single_shot": ThinkingStrategy.SINGLE_SHOT,
            "chain_of_thought": ThinkingStrategy.CHAIN_OF_THOUGHT,
        }.get(strategy_name, ThinkingStrategy.REACT)

        max_iterations = project_config.agents.get("chat-assistant", {}).get("max_iterations", 10)

        # Create cognitive agent
        self.cognitive = CognitiveAgent(
            ctx=self.agent._ctx,
            llm=llm,
            config=CognitiveConfig(
                system_prompt="""You are a helpful AI assistant with access to tools.

Available tools:
- weather:get: Get current weather. Args: {"location": "city name"}
- system:shell: Run shell commands. Args: {"command": "cmd"} (limited to: ls, echo, cat, grep)
- fs:read_file: Read file contents. Args: {"path": "file path"}

When you need information, use the appropriate tool.
Think step by step and explain your reasoning.
Be helpful, concise, and friendly.""",
                thinking_strategy=strategy,
                max_iterations=max_iterations,
                temperature=0.7,
            ),
            available_tools=["weather:get", "system:shell", "fs:read_file"],
        )

        return self

    async def chat(self, message: str, stream_callback=None) -> dict:
        """Process a chat message and return response with reasoning steps.

        Args:
            message: User's input message
            stream_callback: Optional callback for streaming updates

        Returns:
            dict with 'answer', 'steps', 'iterations', 'success'
        """
        # Add to conversation history
        self.conversation_history.append({"role": "user", "content": message})

        # Build context from history
        context = []
        if len(self.conversation_history) > 1:
            # Include recent history as context
            for msg in self.conversation_history[-6:-1]:  # Last 5 messages before current
                context.append(f"{msg['role'].capitalize()}: {msg['content']}")

        # Run cognitive loop
        result = await self.cognitive.run(message, context=context if context else None)

        # Add response to history
        self.conversation_history.append({"role": "assistant", "content": result.answer})

        return {
            "answer": result.answer,
            "steps": [
                {
                    "step": s.step,
                    "reasoning": s.reasoning,
                    "tool_call": s.tool_call.to_dict() if s.tool_call else None,
                    "observation": {
                        "success": s.observation.success,
                        "output": s.observation.output,
                        "error": s.observation.error,
                    }
                    if s.observation
                    else None,
                }
                for s in result.steps
            ],
            "iterations": result.iterations,
            "success": result.success,
            "latency_ms": result.total_latency_ms,
        }

    async def stop(self):
        """Stop the agent."""
        if self.agent:
            await self.agent.stop()

    def clear_history(self):
        """Clear conversation history."""
        self.conversation_history = []
        if self.cognitive:
            self.cognitive.memory.clear()


async def run_cli():
    """Run interactive CLI chat."""
    print("=" * 60)
    print("🤖 Loom Chat Agent")
    print("=" * 60)
    print("Commands: /clear (reset), /history, /quit")
    print("-" * 60)

    chat = ChatAgent()
    await chat.start()

    print("✅ Agent ready! Start chatting...\n")

    try:
        while True:
            try:
                user_input = input("You: ").strip()
            except EOFError:
                break

            if not user_input:
                continue

            if user_input.lower() in ["/quit", "/exit", "/q"]:
                print("Goodbye! 👋")
                break

            if user_input.lower() == "/clear":
                chat.clear_history()
                print("💭 Conversation cleared.\n")
                continue

            if user_input.lower() == "/history":
                print("\n📜 Conversation History:")
                for i, msg in enumerate(chat.conversation_history):
                    role = "You" if msg["role"] == "user" else "Assistant"
                    print(f"  [{i+1}] {role}: {msg['content'][:80]}...")
                print()
                continue

            # Process message
            print("\n🤔 Thinking...", end="", flush=True)

            result = await chat.chat(user_input)

            # Clear "Thinking..." line
            print("\r" + " " * 20 + "\r", end="")

            # Show reasoning steps if any
            if result["steps"]:
                print("💭 Reasoning:")
                for step in result["steps"]:
                    print(f"  Step {step['step']}: {step['reasoning'][:60]}...")
                    if step["tool_call"]:
                        print(f"    🔧 Tool: {step['tool_call']['tool']}({step['tool_call']['args']})")
                    if step["observation"]:
                        obs = step["observation"]
                        status = "✅" if obs["success"] else "❌"
                        output = obs["output"][:50] if obs["success"] else obs["error"]
                        print(f"    {status} Result: {output}...")
                print()

            # Show answer
            print(f"🤖 Assistant: {result['answer']}")
            print(f"   ({result['latency_ms']}ms, {result['iterations']} iterations)\n")

    finally:
        await chat.stop()


if __name__ == "__main__":
    asyncio.run(run_cli())
