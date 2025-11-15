# Loom Runtime Management

This document describes how Loom manages runtime binaries and configuration.

## Overview

Loom provides a unified Python SDK that automatically manages the underlying Rust runtime:

- **No Rust required**: Users only need `pip install loom`
- **Automatic downloads**: Pre-built binaries downloaded on first use
- **Version management**: Multiple runtime versions can coexist
- **Cross-platform**: Supports Linux, macOS, and Windows

## Runtime Modes

### Full Mode (default)

Starts complete Loom Core with:
- Event Bus and Agent Runtime
- gRPC Bridge for Python agents
- Dashboard UI (http://localhost:3030)
- LLM and MCP integration

```bash
loom up
# or explicitly
loom up --mode full --dashboard-port 3030
```

### Bridge-Only Mode

Starts only the gRPC bridge server:
- Minimal runtime for simple agent communication
- No dashboard overhead
- Useful for production deployments where dashboard runs separately

```bash
loom up --mode bridge-only
```

**Note**: Both modes use the same `loom-bridge-server` binary. The `full` mode enables the Dashboard, while `bridge-only` mode disables it. `loom-core` is a Rust library crate, not a standalone executable.

## Binary Management

### Download and Caching

On first use, `loom up` will:

1. Check if binary exists in cache (`~/.cache/loom/bin/{version}/{platform}/`)
2. If not found, download from GitHub Releases
3. Verify SHA256 checksum (if available)
4. Extract and cache for future use

### Platform Detection

Loom automatically detects your platform:

- **Linux**: `linux-x86_64`, `linux-aarch64`
- **macOS**: `macos-x86_64`, `macos-aarch64`
- **Windows**: `windows-x86_64`

### Version Management

```bash
# Use latest version (default)
loom up

# Use specific version
loom up --version 0.2.0

# Multiple versions can coexist in cache
ls ~/.cache/loom/bin/
# latest/
# 0.2.0/
# 0.1.0/
```

### Development Mode

In a development environment, Loom will automatically use local builds:

```bash
# If you have cargo builds in target/, loom will use them
cd /path/to/loom
cargo build -p loom-bridge --bin loom-bridge-server
cd /path/to/my-project
loom up  # Will use your local build
```

## Configuration (`loom.toml`)

Projects can include a `loom.toml` file for configuration:

### Basic Example

```toml
name = "my-agent-project"
version = "0.1.0"
description = "My first Loom project"

[bridge]
address = "127.0.0.1:50051"
mode = "full"
version = "latest"

[dashboard]
enabled = true
port = 3030
host = "127.0.0.1"
```

### LLM Configuration

```toml
[llm.deepseek]
type = "http"
api_key = "sk-xxxxx"  # Or use env var DEEPSEEK_API_KEY
api_base = "https://api.deepseek.com"
model = "deepseek-chat"
max_tokens = 4096
temperature = 0.7

[llm.local]
type = "http"
api_base = "http://localhost:8000"
model = "qwen2.5"
```

### MCP Server Configuration

```toml
[mcp.web-search]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]

[mcp.web-search.env]
BRAVE_API_KEY = "YOUR_KEY_HERE"

[mcp.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

### Agent-Specific Config

```toml
[agents.data-agent]
topics = ["market.price.BTC", "market.price.ETH"]
refresh_interval_sec = 1

[agents.planner]
topics = ["analysis.trend", "analysis.risk", "analysis.sentiment"]
llm_provider = "deepseek"
timeout_sec = 30
```

## Environment Variables

Configuration can also be set via environment variables:

```bash
# Bridge
export LOOM_BRIDGE_ADDR="127.0.0.1:50051"

# Dashboard
export LOOM_DASHBOARD=true
export LOOM_DASHBOARD_PORT=3030

# LLM (for default provider)
export LOOM_LLM_API_KEY="sk-xxxxx"
export LOOM_LLM_API_BASE="https://api.deepseek.com"
export LOOM_LLM_MODEL="deepseek-chat"
```

Priority: Environment variables > `loom.toml` > defaults

## Python API

### Loading Configuration

```python
from loom import ProjectConfig, load_project_config

# Load from current directory
config = load_project_config()

# Access settings
print(config.bridge.address)
print(config.dashboard.port)
print(config.llm_providers["deepseek"].model)
```

### Starting Runtime Programmatically

```python
from loom import embedded

# Start bridge only
proc = embedded.start_bridge("127.0.0.1:50051", version="latest")

# Start full core with dashboard
proc = embedded.start_core(
    bridge_addr="127.0.0.1:50051",
    dashboard_port=3030,
    version="latest",
)

# Keep alive
proc.wait()
```

## Troubleshooting

### Download Failures

If binary download fails:

1. Check internet connection
2. Verify GitHub Releases page has artifacts for your platform
3. Try forcing a local build:
   ```bash
   git clone https://github.com/loom-os/loom
   cd loom
   cargo build -p loom-bridge --bin loom-bridge-server
   cargo build --example dashboard_demo  # For full mode
   ```

### Checksum Verification Failed

If checksum verification fails:

1. Remove cached binary: `rm -rf ~/.cache/loom/bin/{version}`
2. Re-download: `loom up`
3. If persists, report an issue

### Port Already in Use

```bash
# Use different ports
loom up --bridge-port 9999 --dashboard-port 8080
```

### Permission Denied (Unix)

Binaries should be auto-marked executable. If not:

```bash
chmod +x ~/.cache/loom/bin/latest/*/loom-*
```

## CI/CD Integration

For automated testing:

```yaml
# GitHub Actions example
- name: Start Loom runtime
  run: |
    pip install loom
    loom up --mode bridge-only &
    sleep 3  # Wait for startup

- name: Run tests
  run: pytest tests/
```

## Security Considerations

- **Checksums**: Always verify SHA256 checksums in production
- **API Keys**: Use environment variables, never commit to `loom.toml`
- **Network**: Binaries downloaded over HTTPS from GitHub
- **Isolation**: Consider running runtime in containers for production

## Release Process (for maintainers)

When releasing a new version:

1. Build binaries for all platforms
2. Package as:
   - `{binary_name}-{version}-{platform}.tar.gz` (Unix)
   - `{binary_name}-{version}-{platform}.zip` (Windows)
3. Generate SHA256 checksums:
   ```bash
   sha256sum {binary_name}-{version}-{platform}.tar.gz > {asset}.sha256
   ```
4. Upload to GitHub Releases
5. Users can now `loom up --version {version}`

## Future Enhancements

- [ ] HTTPS download with progress bars
- [ ] Signature verification (GPG/cosign)
- [ ] Docker image distribution
- [ ] Cloud-hosted managed runtimes
- [ ] Auto-update mechanism
