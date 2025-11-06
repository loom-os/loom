# Examples and Demos

This repository currently focuses on a complete end‑to‑end Voice Agent demo. Additional demos will follow as the core and capability crates evolve.

## Primary: Voice Agent (E2E)

Mic → VAD → STT → Wake → LLM → TTS. See `demo/voice_agent/README.md` for setup and detailed instructions.

Quick run:

```bash
cargo run -p voice_agent
```

The demo is designed to run locally on Linux/macOS (CPU‑only acceptable). It detects Piper first for TTS and falls back to espeak‑ng when Piper is not available.

## Planned / in-progress

- Camera/vision demo (dependent on a forthcoming `loom-vision` crate)
- Workflow bridge (Loom ↔ n8n)
- Memory agent (episodic + semantic memory)
- Desktop automation
- Crypto Advisor (spec and plan under `demo/crypto_advisor/`)

## Minimal snippets (DIY)

If you want to quickly play with the core runtime without the audio stack, see `docs/QUICKSTART.md` for a tiny pub/sub code sample you can paste into a project. It covers:

- EventBus creation and start
- Subscribing and publishing events
- Tuning QoS for batched vs realtime topics

For audio capture and VAD, add `loom-audio` to your app and enable the appropriate features. Linux packages: `libasound2-dev`, `pkg-config`.
