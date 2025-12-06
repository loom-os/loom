# CLI Module

Command-line interface for Loom - runtime management, agent orchestration, and interactive chat.

## Overview

The CLI provides comprehensive tools for Loom development and deployment:

- **Runtime Management**: Start/stop Loom runtime (`up`, `down`)
- **Project Orchestration**: Run agents and manage lifecycle (`run`)
- **Interactive Chat**: Terminal-based cognitive agent interface (`chat`)
- **Project Scaffolding**: Initialize new projects (`init`)
- **Development Tools**: Proto generation and local bridge (`proto`, `dev`)

## Commands Overview

| Command | Description | Use Case |
|---------|-------------|----------|
| `loom up` | Start Loom runtime | Development, production deployment |
| `loom down` | Stop all Loom processes | Cleanup, restart |
| `loom run` | Run project with agents | Multi-agent orchestration |
| `loom chat` | Interactive cognitive agent | Testing, debugging, demos |
| `loom init` | Create new project | Project scaffolding |
| `loom proto` | Generate gRPC stubs | Development workflow |
| `loom dev` | Start local bridge | Rust development |

## Quick Start

### 1. Start Runtime

```bash
# Full runtime (Core + Bridge + Dashboard)
loom up

# Bridge-only mode
loom up --mode bridge-only

# Custom ports
loom up --bridge-port 9999 --dashboard-port 8080
```

### 2. Run Project

```bash
# Auto-discover and run agents
loom run

# Run specific script
loom run agents/main.py
```

### 3. Interactive Chat

```bash
# Start chat with cognitive agent
loom chat

# Custom model
loom chat --model deepseek
```

### 4. Stop Everything

```bash
# Gracefully stop all processes
loom down
```

## Commands Reference

### `loom up` - Start Runtime

Start Loom runtime (Bridge + Core + Dashboard).

**Syntax:**
```bash
loom up [OPTIONS]
```

**Options:**
- `--mode MODE`: Runtime mode (`full` or `bridge-only`, default: `full`)
- `--version VERSION`: Runtime version (default: `latest`)
- `--bridge-port PORT`: Bridge gRPC port (default: `50051`)
- `--dashboard-port PORT`: Dashboard HTTP port (default: `3030`)
- `--use-debug`: Prefer debug builds over release
- `--force-download`: Force download from GitHub (skip cache)

**Examples:**
```bash
# Default: full runtime with dashboard
loom up

# Bridge-only for production
loom up --mode bridge-only

# Custom ports
loom up --bridge-port 50052 --dashboard-port 3031

# Development with debug builds
loom up --use-debug
```

---

### `loom down` - Stop Runtime

Stop all running Loom processes (runtime + agents).

**Syntax:**
```bash
loom down
```

**What it does:**
1. Scans for all Loom-related processes
2. Sends SIGTERM for graceful shutdown (3s timeout)
3. Sends SIGKILL if processes don't stop
4. Cleans up residual processes

---

### `loom run` - Run Projects

Run Loom project with automatic agent discovery.

**Syntax:**
```bash
loom run [SCRIPT|DIR] [OPTIONS]
```

**Options:**
- `--mode MODE`: Runtime mode (default: `full`)
- `--bridge-port PORT`: Bridge port (default: `50051`)
- `--dashboard-port PORT`: Dashboard port (default: `3030`)
- `--logs`: Enable log files in `logs/` directory
- `--use-debug`: Prefer debug builds
- `--force-download`: Force download binaries

**Auto-discovery:**
Discovers agents from:
1. `agents/*.py` - Agent scripts
2. `main.py`, `run.py`, `app.py` - Entry points
3. `loom.toml` - Configuration

**Examples:**
```bash
# Auto-discover and run
loom run

# Specific script
loom run agents/researcher.py

# With logging
loom run --logs
```

---

### `loom chat` - Interactive Chat

Interactive terminal chat with cognitive agents.

**Syntax:**
```bash
loom chat [OPTIONS]
```

**Options:**
- `--agent-id ID`: Agent identifier (default: `chat-agent`)
- `--model MODEL`: LLM model (`deepseek`, `openai`, `local`)
- `--workspace PATH`: Workspace directory

**Interactive Commands:**
```
/clear  - Clear conversation history
/stats  - Show session statistics
/exit   - Exit chat
```

**Example Session:**
```
You: analyze the codebase
🤔 Thinking: Let me analyze...
🔧 Tool: fs:list_dir(path=".")
✅ Result: [found 15 files]
💡 Final Answer: The project has...
```

---

### `loom init` - Initialize Project

Create new Loom project with templates.

**Syntax:**
```bash
loom init PATH
```

**Examples:**
```bash
# Create project
loom init my-project

# Creates:
# my-project/
# ├── agent.py      # Template agent
# └── loom.toml     # Configuration
```

---

### `loom proto` - Generate gRPC Stubs

Generate Python gRPC stubs from `.proto` files.

**Syntax:**
```bash
loom proto
```

**Use for:**
- Modifying protobuf definitions
- Updating gRPC services
- Development workflow

---

### `loom dev` - Start Local Bridge

Start Bridge from local Rust source.

**Syntax:**
```bash
loom dev [--port PORT]
```

**Requirements:**
- Rust toolchain
- Loom source code

**Example:**
```bash
loom dev --port 50051
```

---

## Key Components

### Chat Interface (`chat.py`)

Interactive chat with rich formatting:

```python
from loom.cli.chat import start_chat

await start_chat(
    agent_id="researcher",
    model="deepseek",
    workspace_path="/workspace",
)
```

**Features:**
- Multi-line input (Ctrl+D to submit)
- Streaming responses with syntax highlighting
- Tool call visualization
- Context metrics display

### Main CLI (`main.py`)

Command-line entry point using argparse:

```python
from loom.cli import main

# Programmatic use
main(["chat", "--model", "deepseek"])
```

## Display Format

### Streaming Thought Process

```
🤔 Thinking: I need to search...
⚡ Tool Call: web:search
   Arguments: {"query": "Loom"}
⏳ Executing tool...
✅ Tool Result: Found 10 results
📊 Context: 1 offloaded output
```

### Final Answer

```
💡 Final Answer:

Based on my search results...
```

## Environment Variables

```bash
# Override bridge address
export LOOM_BRIDGE_ADDR="localhost:50051"

# LLM API keys
export DEEPSEEK_API_KEY="sk-..."
export OPENAI_API_KEY="sk-..."

# Telemetry
export OTEL_SERVICE_NAME="my-agent"
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"
```

## Related Modules

- **runtime**: Orchestrator and embedded runtime
- **cognitive**: CognitiveAgent for chat interface
- **llm**: LLMProvider for API calls
- **agent**: EventContext for runtime connection

## See Also

- [Runtime Module](../runtime/README.md) - Orchestration details
- [Cognitive Guide](../../docs/COGNITIVE_GUIDE.md) - Agent patterns
- [CLI Guide](../../docs/CLI_GUIDE.md) - Extended documentation
