# Mobile‑friendly Core

Targets (release builds):

- Binary size: < 5–8 MB core
- Memory: 20–40 MB resident
- Cold start: < 200 ms
- Internal hop latency: 10–50 ms typical

Feature gating:

- Core: event‑bus, agent‑runtime, router (abstraction), minimal telemetry
- Plus (opt‑in): WASM, local‑ml (TFLite/ONNX), cloud connectors, RocksDB, advanced metrics

Packaging:

- iOS: static .xcframework + C ABI (cbindgen) + Swift wrapper; AOT WASM (WAMR)
- Android: AAR (JNI) + cargo‑ndk; AudioRecord/CameraX; TextToSpeech; AOT WASM

WASM runtimes:

- Desktop/server: Wasmtime (debuggability)
- Mobile: WAMR AOT (no JIT; smaller footprint)

Build & perf:

- LTO, codegen‑units=1, panic=abort; strip symbols
- Backpressure + batching in the event bus
- Pluggable storage: in‑memory / lightweight KV / RocksDB via features

Security:

- Capability tokens per plugin (event/topic access, actions, storage, network)
- WASI‑style sandboxing for WASM; resource quotas (CPU/memory/time)
