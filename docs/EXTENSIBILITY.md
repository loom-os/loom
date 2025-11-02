# Extensibility and SDKs

Plugin tiers (choose by trust/perf/isolation):

1. Native Rust — best performance, weakest isolation
2. WASM — portable and sandboxed (capabilities), recommended default for third‑party
3. Out‑of‑process (gRPC/UDS) — strongest isolation, language‑agnostic (Python/Node/Java)

Protocol

- Protobuf‑defined plugin API (`core/proto/plugin.proto`): init, handle event, health, shutdown
- Capability declaration during init; runtime issues scoped tokens

Security

- Principle of least privilege (topics/actions/storage/network)
- Resource limits; audit logs; optional payload masking

SDK roadmap

- Rust: native and WASM templates (macros/traits)
- Python/Node: gRPC templates; 10‑minute onboarding
- Scaffolding: `plugins/` templates; future CLI `loom new plugin --lang <lang>`
