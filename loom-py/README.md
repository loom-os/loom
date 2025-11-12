# Loom Python SDK

[![PyPI](https://img.shields.io/pypi/v/loom.svg)](https://pypi.org/project/loom/)
[![Python Version](https://img.shields.io/pypi/pyversions/loom.svg)](https://pypi.org/project/loom/)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![CI](https://github.com/loom-os/loom/workflows/Loom%20Python%20SDK%20CI/badge.svg)](https://github.com/loom-os/loom/actions)

Build event-driven multi-agent systems in Python and connect to Loom Core over gRPC.

**Status**: Alpha (0.1.0a1) - Expect breaking changes until 0.1.0 stable release.

## Install

### From PyPI (Recommended)

```bash
pip install loom
```

### From Source

```bash
git clone https://github.com/loom-os/loom.git
cd loom/loom-py
pip install -e .
```

### Development Mode

```bash
pip install -e ".[dev]"
```

## Quick Start

### 1. Start Bridge Server

The Bridge server enables communication between Python agents and Loom Core:

```bash
# Option 1: Auto-download and run (coming soon)
loom up

# Option 2: Build from source
git clone https://github.com/loom-os/loom.git
cd loom
cargo run -p loom-bridge --bin loom-bridge-server
```

Or connect to a remote bridge:

```bash
export LOOM_BRIDGE_ADDR="bridge.example.com:50051"
```

### 2. Create Your First Agent

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

### 3. Run Your Agent

```bash
python my_agent.py
```

## What's Included

✅ **Core Features** (v0.1.0a1):

- Agent lifecycle management
- Event pub/sub via Bridge
- Capability system with auto schema generation
- Context API (emit, reply, tool invocation)
- Envelope for correlation and threading
- Request/reply with timeout
🚧 **Coming Soon**:
- Memory backends
- Dynamic subscriptions
- Streaming responses


## Documentation

- [SDK Guide](docs/SDK_GUIDE.md) - Complete API reference and tutorials
- [Examples](examples/) - Working code samples
- [DESIGN.md](docs/DESIGN.md) - Architecture and design decisions
- [FUTURE.md](docs/FUTURE.md) - Roadmap and planned features

## Requirements

- Python 3.9+
- Loom Bridge server (local or remote)

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Generate proto files
python -m loom.proto.generate

# Run tests
pytest

# Format code
black src/ tests/
ruff check src/ tests/
```

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) in the main repository.

## License

Apache License 2.0 - See [LICENSE](LICENSE)

## Links

- [Main Repository](https://github.com/loom-os/loom)
- [Documentation](https://github.com/loom-os/loom/tree/main/docs)
- [PyPI Package](https://pypi.org/project/loom/)
- [Issue Tracker](https://github.com/loom-os/loom/issues)
