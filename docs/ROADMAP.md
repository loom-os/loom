# 6–8 Week Roadmap (Alpha)

Weeks 1–2 — Core loop

- Event bus QoS/backpressure/retry stabilized
- Router abstraction + minimal privacy/latency/cost policy
- Examples: voice & camera running
- iOS/Android packaging POC (.xcframework/AAR + basic wrappers)

Weeks 3–4 — Extensibility & ecosystem

- WASM runtime: Wasmtime (desktop), WAMR AOT (mobile)
- gRPC plugin templates: Python/Node
- vLLM adapter (local/remote backend)
- n8n node (minimal)

Weeks 5–6 — Mobile depth & observability

- iOS AOT path validated; Android NDK tuning & permissions
- Minimal tracing/metrics; simple visualization
- Examples 3–5 completed; Quickstart expanded

Weeks 7–8 — Perf, stability, release

- Footprint/latency optimizations and measurements
- Router policy with semantic signals
- Docs/site polish; Alpha release

Acceptance criteria

- 2 examples runnable in ≤10 minutes (incl. one Python plugin)
- Mobile core <8 MB; cold start <200 ms
- Event bus reliable under load with backpressure
- vLLM backend routable; n8n node working
