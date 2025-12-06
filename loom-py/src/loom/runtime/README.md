# Runtime Module

Loom Runtime management - orchestration, embedded binaries, and project configuration.

## Overview

The runtime module provides tools for managing the Loom runtime lifecycle:

- **Orchestrator**: Full project lifecycle management (runtime + agents)
- **Embedded**: Binary download, caching, and execution
- **Config**: Project configuration from `loom.toml`

## Key Components

### 1. Orchestrator (`orchestrator.py`)

Manages runtime and agent processes for a project.

```python
from loom.runtime import Orchestrator, OrchestratorConfig

config = OrchestratorConfig(
    project_dir=Path("/path/to/project"),
    runtime_mode="full",  # or "bridge-only"
    agent_scripts=[Path("agents/researcher.py")],
)

# Run orchestrator (blocks until shutdown)
await run_orchestrator(config)
```

**Features:**

- Automatic runtime startup (Core + Bridge + Dashboard)
- Agent process management with restart on failure
- Graceful shutdown with SIGTERM → SIGKILL escalation
- Log file management
- Health checking

**Runtime Modes:**

- `full`: Core + Bridge + Dashboard (default)
- `bridge-only`: Bridge only (for agent-only deployments)

### 2. Embedded Runtime (`embedded.py`)

Downloads and manages pre-built binaries.

```python
from loom.runtime import start_bridge, start_core, get_binary

# Start bridge server
proc = await start_bridge(port=50051)

# Start core runtime
proc = await start_core(dashboard_port=3030)

# Download specific binary
binary_path = await get_binary(
    name="loom-bridge",
    version="v0.1.0",
    force_download=False,
)
```

**Binary Management:**

1. **Local builds**: Check `target/release/` and `target/debug/` first
2. **Cache**: Use `~/.cache/loom/bin/` if available
3. **Download**: Fetch from GitHub releases if needed

**Functions:**

- `start_bridge()`: Start Bridge server
- `start_core()`: Start Core runtime
- `get_binary()`: Download/cache binary
- `binary_path()`: Get path to cached binary
- `cache_dir()`: Get cache directory
- `platform_tag()`: Get platform identifier (e.g., `linux-x86_64`)

### 3. Project Configuration (`config.py`)

Load and parse `loom.toml` configuration.

```python
from loom.runtime import ProjectConfig, load_project_config

# Load from directory
config = load_project_config(Path("/project"))

# Access configuration
config.bridge.address  # Bridge address
config.llm["deepseek"].base_url  # LLM config
config.mcp_servers["brave"].command  # MCP server config
config.dashboard.enabled  # Dashboard settings
```

**Configuration Sections:**

```toml
# loom.toml

[bridge]
address = "localhost:50051"

[dashboard]
enabled = true
port = 3030

[llm.deepseek]
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"
api_key = "${DEEPSEEK_API_KEY}"

[mcp.brave]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]
env = { BRAVE_API_KEY = "${BRAVE_API_KEY}" }
```

**Types:**

- `ProjectConfig`: Root configuration
- `BridgeConfig`: Bridge connection settings
- `LLMProviderConfig`: LLM provider configuration
- `MCPServerConfig`: MCP server configuration
- `DashboardConfig`: Dashboard settings

## Usage Examples

### Start Runtime Programmatically

```python
import asyncio
from pathlib import Path
from loom.runtime import start_bridge, start_core

async def main():
    # Start bridge
    bridge_proc = await start_bridge(port=50051)

    # Start core with dashboard
    core_proc = await start_core(dashboard_port=3030)

    print("Runtime started!")
    print("Bridge: localhost:50051")
    print("Dashboard: http://localhost:3030")

    # Wait for shutdown signal
    await asyncio.Event().wait()

asyncio.run(main())
```

### Orchestrate Full Project

```python
import asyncio
from pathlib import Path
from loom.runtime import OrchestratorConfig, run_orchestrator

async def main():
    config = OrchestratorConfig(
        project_dir=Path.cwd(),
        runtime_mode="full",
        agent_scripts=[
            Path("agents/researcher.py"),
            Path("agents/analyst.py"),
        ],
        logs_dir=Path("logs"),
    )

    await run_orchestrator(config)

asyncio.run(main())
```

### Custom Binary Download

```python
from loom.runtime import get_binary, cache_dir

# Download specific version
binary = await get_binary(
    name="loom-bridge",
    version="v0.1.0-alpha.1",
    force_download=True,  # Skip cache
)

print(f"Binary location: {binary}")
print(f"Cache directory: {cache_dir()}")
```

## CLI Integration

The runtime module powers the `loom` CLI commands:

```bash
# Start runtime and agents
loom up                    # Full mode (default)
loom up --mode bridge-only # Bridge only
loom up --force-download   # Force re-download

# Stop all Loom processes
loom down

# Run project
loom run                   # Auto-discover agents
loom run agents/main.py    # Specific script
```

## Environment Variables

```bash
# Override bridge address
export LOOM_BRIDGE_ADDR="bridge.example.com:50051"

# Force debug builds
export LOOM_PREFER_DEBUG=1

# Custom cache directory
export XDG_CACHE_HOME="/custom/cache"
```

## Binary Caching

Binaries are cached in `~/.cache/loom/bin/`:

```
~/.cache/loom/bin/
├── loom-bridge-v0.1.0-linux-x86_64
├── loom-core-v0.1.0-linux-x86_64
└── checksums.txt
```

Cache management:

- Automatic version validation
- SHA256 checksum verification
- Platform-specific storage
- Shared across projects

## Architecture

```
┌─────────────────────────────────────┐
│         Orchestrator                │
│  (Lifecycle Management)             │
└────────┬────────────┬───────────────┘
         │            │
         ▼            ▼
┌─────────────┐  ┌──────────────┐
│   Embedded  │  │   Config     │
│  (Binaries) │  │ (loom.toml)  │
└─────────────┘  └──────────────┘
         │
         ▼
┌─────────────────────────────────────┐
│  Rust Core Runtime                  │
│  ┌──────┐  ┌────────┐  ┌─────────┐ │
│  │Bridge│  │  Core  │  │Dashboard│ │
│  └──────┘  └────────┘  └─────────┘ │
└─────────────────────────────────────┘
```

## Related Modules

- **agent**: Use EventContext to connect to runtime
- **bridge**: gRPC client for Bridge communication
- **cli**: Command-line interface (`loom up`, `loom run`)
- **telemetry**: Observability integration

## See Also

- [Orchestrator Design](../../../docs/ORCHESTRATOR.md)
- [Binary Distribution](../../../docs/DISTRIBUTION.md)
- [Project Configuration](../../../docs/CONFIGURATION.md)
