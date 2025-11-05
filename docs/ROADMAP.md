# Roadmap (prioritized by issues)

This roadmap is re-centered on the voice_agent E2E demo as our first complete, event-driven loop: Mic → VAD → STT → Wake → Agent → LLM → TTS. Highest priorities are tasks required to make this demo robust and easy to run; lower priorities cover broader OS and ecosystem evolution.

## P0 — Voice Agent E2E (highest priority)

Goal: ship a reliable demo that runs locally end-to-end with minimal setup and clear docs.

Done

- Mic capture EventSource (cpal) — audio_chunk events with metadata
- VAD gating (webrtc-vad) — speech_start/end, audio_voiced branch
- Utterance segmentation + STT (CLI whisper.cpp) — transcript.final events
- Wake word on transcripts — fuzzy match “hey loom/loom”
- LLM HTTP client — prefers /v1/responses, fallback to /v1/chat/completions
- PromptBundle → messages/input adapter with simple budget trim

Remaining

- Local TTS capability provider
  - Detect piper first, fallback to espeak-ng; headers: voice/rate/volume
  - Degrade gracefully if missing; print guidance
- E2E example: `core/examples/e2e_voice_wake_llm_tts.rs`
  - Wire Mic → VAD → STT → Wake → Agent invoking `llm.generate` → TTS
  - Clear logs, robust error handling at each step
- Docs: Voice E2E guide (docs/voice_agent)
  - Install whisper.cpp and models OR choose alternative STT
  - Start a local vLLM (or point to cloud); env config matrix
  - Run the example; troubleshooting (devices, sampling rate, permissions)
- Observability (minimal)
  - Publish assistant.message and action_result events; summarize latency per stage
  - Enable basic tracing in demo path

Acceptance for P0

- Demo runs on Linux/macOS with CPU-only setup in ≤15 minutes
- End-to-end latency per utterance documented; robust failure messages and fallbacks
- Single command to run the example; environment variables documented

## P1 — Quality and developer ergonomics

Improve latency, stability, and app authoring experience while staying within the voice_agent scope.

- Streaming LLM → TTS
  - SSE from Responses or Chat Completions; incremental `assistant.token` events
  - Early TTS playback for perceived latency
- Session management
  - Scoped session_id with turn boundaries (wake → query → response)
  - Time-based expiry; state propagation in events
- PromptBuilder enhancements
  - Retrieval via MemoryReader; episodic summaries via MemoryWriter
  - Role-aware history; tokenizer-based budgeting (optional feature)
- Tool/function calling path
  - Parse tool calls; dispatch through ActionBroker; feed results back to LLM
- Router improvements
  - Quick (local) + refine (cloud) policy for the demo, configurable per agent
  - Basic price/latency estimates surfaced in routing_decision
- Observability
  - Counters and simple dashboards (latency per stage, error rates)
  - Structured logs and sampling in examples

## P2 — Broader OS and ecosystem (medium priority)

Focus on extensibility, integration, and mobile/runtime breadth beyond the voice_agent.

- Capability providers
  - WASM provider (desktop): Wasmtime; AOT ready for mobile later
  - Out-of-process plugin adapter (gRPC/MCP); templates for Python/Node
- Integrations
  - n8n node (minimal) for orchestration
  - Model/router signals exported for external policy engines
- Local ML backends
  - Replace LocalModel stub with TFLite/ONNX RT variants (vision/audio examples)
  - Event schemas for detections; policy hooks for privacy-preserving on-device inference
- Event bus hardening
  - Backpressure/load tests, QoS tuning, bounded memory
  - Benchmark harness and artifacts
- Docs/site
  - Quickstart upgrades; example catalog; troubleshooting matrix
  - Configuration/secrets guidance

## P3 — Mobile and performance (lower priority)

- iOS/Android packaging POC (xcframework/AAR) and minimal wrappers
- AOT-friendly WASM path (WAMR) and on-device model execution tuning
- Footprint/latency optimizations with measurements and targets

---

Notes

- The LLM layer now supports both /responses and /chat/completions, making it compatible with vLLM and Transformers-based backends as well as OpenAI-style clouds.
- We intentionally use capabilities (`llm.generate`, `tts.*`) so apps and agents remain backend-agnostic; routing and headers select the actual engine.
- The voice_agent demo is our proving ground; we’ll feed its learnings back into core APIs, event schemas, and docs.
