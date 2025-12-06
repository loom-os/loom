"""Loom Python SDK - Brain 🧠 for AI agents.

This is the Python SDK for Loom, an event-driven AI agent runtime.
It provides the "Brain" in Loom's Brain/Hand separation:

- **Brain (Python)**: LLM calls, reasoning, context engineering
- **Hands (Rust Core)**: Tool execution, event bus, persistence

Quick Start:
    ```python
    from loom import Agent, tool

    @tool("hello.echo", description="Echo a message")
    def echo(text: str):
        return {"echo": text}

    async def on_event(ctx, topic, event):
        print(f"Received: {event.type}")

    agent = Agent(
        agent_id="my-agent",
        topics=["my.topic"],
        tools=[echo],
        on_event=on_event,
    )

    agent.run()
    ```

For cognitive agents with LLM reasoning:
    ```python
    from loom import Agent, CognitiveAgent, CognitiveConfig, LLMProvider

    agent = Agent(agent_id="researcher", topics=["research.tasks"])
    await agent.start()

    cognitive = CognitiveAgent(
        ctx=agent.ctx,
        llm=LLMProvider.from_config(agent.ctx, "deepseek", config),
        config=CognitiveConfig(
            system_prompt="You are a research assistant...",
            max_iterations=5,
        ),
    )

    result = await cognitive.run("Research the latest AI trends")
    ```

Module structure:
    - agent/: Agent, Context, Envelope
    - cognitive/: CognitiveAgent, reasoning loops
    - context/: Context engineering (memory, ranking, window)
    - llm/: LLM providers (direct HTTP)
    - tools/: Tool decorator and types
    - bridge/: gRPC communication with Rust Core
    - runtime/: Config, orchestrator, embedded runtime
    - telemetry/: OpenTelemetry tracing
    - cli/: Command line interface
"""

# Agent
from .agent import Agent, Envelope, EventContext

# Cognitive
from .cognitive import (
    CognitiveAgent,
    CognitiveConfig,
    CognitiveResult,
    ThinkingStrategy,
    WorkingMemory,
)

# LLM
from .llm import LLMConfig, LLMProvider

# Config
from .runtime.config import ProjectConfig, load_project_config

# Telemetry
from .telemetry import init_telemetry, shutdown_telemetry

# Tools
from .tools import Capability, Tool, capability, tool

# Backward compatibility
Context = EventContext

__all__ = [
    # Agent
    "Agent",
    "Context",
    "Envelope",
    # Cognitive
    "CognitiveAgent",
    "CognitiveConfig",
    "CognitiveResult",
    "ThinkingStrategy",
    "WorkingMemory",
    # LLM
    "LLMProvider",
    "LLMConfig",
    # Tools
    "tool",
    "Tool",
    # Config
    "ProjectConfig",
    "load_project_config",
    # Telemetry
    "init_telemetry",
    "shutdown_telemetry",
    # Deprecated aliases
    "capability",
    "Capability",
]
