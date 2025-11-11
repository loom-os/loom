# Loom Python SDK (MVP)

Build event-driven multi-agent systems in Python and talk to Loom Core over gRPC.

Status: MVP for Roadmap P0. Expect breaking changes until 0.1.0.

## Install

From source (until a PyPI release is published):

```bash
pip install -e ./loom-py
# Stubs are not required at runtime: wheels on PyPI will include them.
# When working from the monorepo, you can optionally generate stubs for local edits:
#   pip install 'loom[dev]'
#   loom proto
```

Planned: `pip install loom` once published to PyPI.

## Quickstart

```python
from loom import Agent, capability

@capability("web.search", version="1.0")
def web_search(query: str) -> dict:
    return {"query": query, "results": ["example.com"]}

async def on_event(ctx, topic, event):
    # Echo payload back
    await ctx.emit(topic, type="echo", payload=event.payload)

agent = Agent(
    agent_id="py-agent-1",
    topics=["topic.test"],
    capabilities=[web_search],
    address="127.0.0.1:50051",  # LOOM_BRIDGE_ADDR
)

if __name__ == "__main__":
    agent.run()
```

## Local developer workflow

- Start a local bridge:
  - `loom up` (embedded mode: download/cache or reuse a local build into ~/.cache/loom/bin)
  - or `loom dev` (from source via cargo)
- Scaffold a new agent: `loom new my-agent && cd my-agent && python agent.py`

In production or CI, you can point the SDK at a managed bridge via `LOOM_BRIDGE_ADDR` or a config file; SDK defaults to `127.0.0.1:50051` if unset.

## Features

- Agent: register, stream events, graceful shutdown
- Context API: emit/request/reply/tool/memory/join_thread (MVP: emit/reply/tool; request/join_thread simplified)
- Capability decorator: register Python functions with JSON Schema metadata
- Unified envelope: thread_id/correlation_id/sender/reply_to/ttl_ms encoded in Event.metadata

See `examples/` for a minimal Planner→Researcher→Writer trio.

## Design overview

- Transport: gRPC Bridge (`RegisterAgent`, `EventStream` with Ack handshake, `ForwardAction`, `Heartbeat`).
- Agent: owns connection lifecycle and the streaming loop; dispatches `Delivery` to a user `on_event`, and executes `action_call` against registered capabilities.
- Context: event primitives (`emit`, `reply`, basic `request`), tool invocations, and lightweight in-process memory; unified envelope encodes thread/correlation metadata in `Event.metadata`.
- Capabilities: declared via `@capability`; input schema is derived from function signature using Pydantic and registered in capability metadata.

More details in `FUTURE.md` and inline docstrings.
