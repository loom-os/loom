## LLM Client and Adapters

Responsibility

- Adapt external LLM providers to the internal request/response model.
- Provide timeouts, retries, and multiple response-mode support (batch API requests, streaming/SSE where supported).

Key files

- `core/src/llm/adapter.rs` — provider-specific adapters.
- `core/src/llm/client.rs` — high-level client used by other core components.
- `core/src/llm/provider.rs` — provider interface and implementations.
- `core/src/llm/tool_orchestrator.rs` — tool discovery, model invocation with tools, parsing, broker integration, and observability.

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

---

## Tool Use Orchestrator

Responsibility

- Discover registered capabilities from the `ActionBroker` and expose them to the model as tools.
- Parse tool calls from provider responses (Responses API preferred, Chat Completions fallback).
- Invoke capabilities via the `ActionBroker` and optionally perform a refinement turn with the tool results.

Key files

- `core/src/llm/tool_orchestrator.rs` — tool discovery, model invocation with tools, parsing, broker integration, and observability.

Provider protocol

- Primary: OpenAI-style Responses API with `tools` and `tool_choice`.
- Fallback: Chat Completions with `tools` and `tool_choice` and `tool_calls` in the response.

Capability metadata → function schema

- Each capability may populate `CapabilityDescriptor.metadata`:
  - `desc`: short description (string)
  - `schema`: JSON Schema string for the `parameters` object

Example metadata for `web.search`:

```
desc = "Search the web for recent information."
schema = '{"type":"object","properties":{"query":{"type":"string"},"top_k":{"type":"integer","minimum":1,"maximum":10,"default":5}},"required":["query"]}'
```

Public API

- `ToolOrchestrator::run(bundle, budget, options, correlation_id)`
  - `options.tool_choice`: Auto | Required | None
  - `options.per_tool_timeout_ms`: timeout for each tool
  - `options.refine_on_tool_result`: whether to perform a second LLM turn with tool results

Observability

- Tracing spans and logs under target `tool_orch`:
  - discovery latency, tool invocation status/latency, refine latency
- In-memory counters via `ToolOrchestratorStats`:
  - `total_invocations`, `total_tool_calls`, `total_tool_errors`, `avg_tool_latency_ms`

Error handling

- No tool calls: return assistant text.
- Malformed/missing arguments: surfaced as `ActionResult` errors from the broker and logged.
- Missing capability: broker returns `CAPABILITY_ERROR` with message.
