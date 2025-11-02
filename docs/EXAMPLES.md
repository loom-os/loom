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

Locations

- Minimal examples: `core/examples/`
- End‑to‑end demos: `examples/`
