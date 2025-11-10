## LLM Client and Adapters

Responsibility

- Adapt external LLM providers to the internal request/response model.
- Provide timeouts, retries, and multiple response-mode support (batch API requests, streaming/SSE where supported).

Key files

- `core/src/llm/adapter.rs` — provider-specific adapters.
- `core/src/llm/client.rs` — high-level client used by other core components.
- `core/src/llm/provider.rs` — provider interface and implementations.

Supported paths and behaviors

- `/responses` vs `/chat/completions`: adapters normalize provider responses into a common internal shape.
- SSE / streaming: supported when a provider offers streaming; adapters expose a streaming option to callers.

Common error paths and test cases

- Timeout handling: client timeouts should cancel inflight requests and return a clear timeout error.
- Partial responses: streaming interruptions should produce a partial result with an associated error tag.
- Retry semantics: test idempotent retries where provider semantics permit.

Tuning and configuration

- Per-provider timeout and retry counts.
- Backoff strategy for retries.

Testing tips

- Provide a mock LLM provider that can simulate delays, partial streams, and errors to validate caller resilience.
