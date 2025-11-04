# Out‑of‑the‑box Examples

Zero‑background path (≈10 minutes):

1. Build core (release)
2. Run basic pub/sub example
3. Switch router policy (local vs cloud) and observe latency/privacy differences
4. Add a Python gRPC plugin without writing Rust

Example set

- Voice assistant: Mic → wake‑word (WASM) → Router → cloud LLM → TTS action
- Camera pipeline: Camera → local detector (WASM/local ML) → annotated events → UI/TTS
- Workflow bridge: Loom topic ↔ n8n → email/calendar task
- Memory agent: dialog events → short‑term context + long‑term memory → action selection
- Desktop automation: system events → rules/LLM tools → safe actions
- Hybrid routing demo: local small model + cloud LLM, policy‑driven switching
- Crypto Advisor (planned): market + sentiment streams → indicators → signals → paper broker

Locations

- Minimal examples: `core/examples/`
- End‑to‑end demos: `examples/`
  - Planned: `examples/crypto_advisor/` (spec + backlog)

## Minimal: ActionBroker + Echo TTS

A tiny example that registers a native capability `tts.echo` and invokes it through the ActionBroker.

Run:

```bash
cd core
cargo run --example echo_tts
```

Expected output:

```
[EchoTts] speaking: Hello Loom!
ActionResult: status=0, error=None, output={"spoken":"Hello Loom!"}
```

If you see a build error mentioning libclang (bindgen), install the system packages (Debian/Ubuntu):

```bash
sudo apt-get update
sudo apt-get install -y clang libclang-dev pkg-config build-essential
```

Then re-run the example command.

## Minimal: ContextBuilder + InMemoryMemory + Echo TTS

This example shows how to build a small prompt context from recent session events and then invoke a capability.

Run:

```bash
cd core
cargo run --example echo_tts_with_context
```

It will print the minimal instructions from ContextBuilder and then call `tts.echo` through the broker.

## E2E: Mock LLM tool-use → ActionBroker → EventBus

A minimal end-to-end loop where a mock LLM decides to call a tool, the broker executes it, and the result is published as an event.

Run:

```bash
cd core
cargo run --example e2e_tool_use
```

This will:

- Seed a session with a couple of events
- Build a minimal context
- Mock LLM selects `tts.echo` with arguments
- ActionBroker executes the tool call
- Publish a final `action_done` event on the bus

## Minimal: Mic → audio_chunk events (cpal)

Capture microphone audio and publish `audio_chunk` events at a fixed chunk size. This demo requires enabling the `mic` feature and having system audio development libraries installed (on Debian/Ubuntu: `sudo apt-get install -y libasound2-dev pkg-config`).

Run:

```bash
cd core
# Run for ~5 seconds by default
cargo run --example mic_capture --features mic
# Or run until Ctrl-C
MIC_DEMO_SECONDS=0 cargo run --example mic_capture --features mic
```

Environment variables (optional):

- `MIC_DEVICE` — substring to select the input device by name (e.g., "USB").
- `MIC_CHUNK_MS` — chunk size in milliseconds (default: 20).
- `MIC_TOPIC` — event topic to publish to (default: `audio.mic`).
- `MIC_SOURCE` — event source string (default: `mic.primary`).
- `MIC_DEMO_SECONDS` — demo duration in seconds; set to `0` or `inf` to run until Ctrl-C (default: `5`).

Each event includes metadata:

- `sample_rate`, `channels`, `device`, `encoding` (pcm_s16le), `chunk_ms`, and `frame_samples`.

Subscribe in code with `QoSRealtime` for best latency and drop behavior under load.
