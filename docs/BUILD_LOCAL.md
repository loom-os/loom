# Building Loom Core Locally

This guide explains how to build Loom Core and Bridge binaries from source for local development and testing.

## Prerequisites

### Required Tools

- **Rust Toolchain** (1.75 or later)

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```

- **Protocol Buffers Compiler**

  ```bash
  # Ubuntu/Debian
  sudo apt install protobuf-compiler

  # macOS
  brew install protobuf

  # Or download from https://github.com/protocolbuffers/protobuf/releases
  ```

- **Python 3.8+** (for SDK testing)
  ```bash
  python3 --version
  ```

### Optional Tools

- **Node.js & npm** (for Dashboard frontend)
  ```bash
  # Only needed if you want to rebuild the Dashboard
  node --version
  npm --version
  ```

## Building the Core Runtime

### 1. Clone the Repository

```bash
git clone https://github.com/loom-os/loom.git
cd loom
```

### 2. Build All Binaries

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (optimized for performance)
cargo build --release
```

**Build Outputs**:

- `target/debug/loom-core` (or `target/release/loom-core`)
- `target/debug/loom-bridge-server` (or `target/release/loom-bridge-server`)

### 3. Build Specific Components

```bash
# Build only Core
cargo build -p loom-core

# Build only Bridge
cargo build -p loom-bridge

# Build with Dashboard frontend
cd core/src/dashboard/frontend
npm install
npm run build
cd ../../../..
cargo build -p loom-core --features dashboard
```

## Using Local Builds with Python SDK

The Python SDK (`loom-py`) automatically detects and uses local builds before downloading from GitHub Releases.

### How It Works

When you run `loom up` or `loom run`, the SDK searches for binaries in this order:

1. **Cached** (`~/.cache/loom/bin/{version}/{platform}/`)
2. **Local Build** (`target/debug/` or `target/release/`)
3. **GitHub Release** (downloads if not found)

### Verification

To see which binary is being used:

```bash
# Start runtime
loom up

# Check the process
ps aux | grep loom-core
# Should show path to binary (e.g., /path/to/loom/target/debug/loom-core)
```

### Force Local Build Usage

If you're developing on Core and want to ensure your local build is used:

```bash
# Option 1: Run from repo root
cd /path/to/loom
loom up

# Option 2: Clear cache
rm -rf ~/.cache/loom/bin/
loom up  # Will use local build

# Option 3: Use cargo directly
export LOOM_BRIDGE_ADDR="127.0.0.1:50051"
export LOOM_DASHBOARD_PORT="3030"
cargo run -p loom-core
```

## Development Workflow

### Iterative Development

```bash
# Terminal 1: Watch and rebuild on changes
cargo watch -x 'build -p loom-core'

# Terminal 2: Run your Python agents
cd demo/market-analyst
loom run
```

### Testing Core Changes

```bash
# Run Core unit tests
cargo test -p loom-core

# Run Bridge integration tests
cargo test -p loom-bridge

# Run end-to-end tests
cargo test --workspace

# Run specific test
cargo test -p loom-core test_event_bus
```

### Dashboard Development

If you're working on the Dashboard frontend:

```bash
# Terminal 1: Frontend dev server (hot reload)
cd core/src/dashboard/frontend
npm run dev

# Terminal 2: Run Core with external frontend
export LOOM_DASHBOARD_DEV=true
cargo run -p loom-core

# Access at http://localhost:5173 (Vite dev server)
```

## Build Configurations

### Debug vs Release

- **Debug** (`cargo build`):

  - Fast compilation (~2 minutes)
  - Larger binary (~100MB)
  - Includes debug symbols
  - Slower runtime performance
  - **Use for**: Development, testing, debugging

- **Release** (`cargo build --release`):
  - Slow compilation (~5 minutes)
  - Smaller binary (~20MB)
  - Optimized for performance
  - No debug symbols (use `--profile release-with-debug` if needed)
  - **Use for**: Production, benchmarking, demos

### Cross-Compilation

To build for a different target:

```bash
# List available targets
rustc --print target-list

# Add target
rustup target add x86_64-unknown-linux-musl

# Build for target
cargo build --release --target x86_64-unknown-linux-musl
```

### Feature Flags

```bash
# Build without Dashboard
cargo build --no-default-features

# Build with OpenTelemetry
cargo build --features otlp

# Build with all features
cargo build --all-features
```

## Platform-Specific Notes

### Linux

```bash
# Install dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev protobuf-compiler

# Build
cargo build --release
```

### macOS

```bash
# Install dependencies
brew install protobuf

# Build for current architecture
cargo build --release

# Universal binary (Intel + Apple Silicon)
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
lipo -create \
  target/x86_64-apple-darwin/release/loom-core \
  target/aarch64-apple-darwin/release/loom-core \
  -output loom-core-universal
```

### Windows

```bash
# Install Rust and Visual Studio Build Tools
# https://visualstudio.microsoft.com/downloads/ (Build Tools)

# Build
cargo build --release

# Output: target\release\loom-core.exe
```

## Troubleshooting

### "protoc not found"

```bash
# Install Protocol Buffers compiler
# See Prerequisites section above
```

### "linker `cc` not found"

```bash
# Install build essentials
sudo apt install build-essential  # Ubuntu/Debian
xcode-select --install            # macOS
```

### "cannot find -lssl"

```bash
# Install OpenSSL development libraries
sudo apt install libssl-dev  # Ubuntu/Debian
brew install openssl         # macOS
```

### Local Build Not Detected

The SDK looks for binaries in:

- `target/debug/loom-core`
- `target/release/loom-core`
- `target/debug/loom-bridge-server`
- `target/release/loom-bridge-server`

Ensure you're running `loom up` from the repo root, or the binary exists in one of these paths.

### Binary Version Mismatch

If you see protocol errors between SDK and Core:

```bash
# Rebuild both
cd /path/to/loom
cargo build --release

# Reinstall SDK (if you've made proto changes)
cd loom-py
pip install -e .
```

## Performance Profiling

### Using `perf` (Linux)

```bash
# Build with debug symbols
cargo build --profile release-with-debug

# Record performance data
perf record -g target/release-with-debug/loom-core

# View report
perf report
```

### Using Instruments (macOS)

```bash
# Build with debug symbols
cargo build --profile release-with-debug

# Open in Instruments
open -a Instruments target/release-with-debug/loom-core
```

### Using `cargo flamegraph`

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin loom-core

# Opens flamegraph.svg in browser
```

## Continuous Integration

Our CI builds and tests on:

- Ubuntu 22.04 (x86_64)
- macOS 13 (x86_64, aarch64)
- Windows Server 2022 (x86_64)

See `.github/workflows/ci.yml` for the full CI configuration.

## Creating Release Binaries

For maintainers creating official releases:

```bash
# Build release binaries for all platforms
./scripts/build-releases.sh

# This creates:
# - loom-core-{version}-linux-x86_64.tar.gz
# - loom-core-{version}-macos-x86_64.tar.gz
# - loom-core-{version}-macos-aarch64.tar.gz
# - loom-core-{version}-windows-x86_64.zip
# - SHA256SUMS.txt
```

## Next Steps

- **Run Examples**: `cargo run --example dashboard_demo`
- **Read Architecture**: [ARCHITECTURE.md](ARCHITECTURE.md)
- **Contribute**: [CONTRIBUTING.md](../CONTRIBUTING.md)
- **Join Community**: [Discord](https://discord.gg/loom-os)

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Protocol Buffers](https://protobuf.dev/)
- [Loom Architecture](ARCHITECTURE.md)
