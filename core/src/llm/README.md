# LLM Module

HTTP client, prompt adapter, and capability provider for OpenAI-compatible backends (vLLM, OpenAI-style servers, etc.).

## Components

- client.rs — `LlmClient`, `LlmClientConfig`, and `LlmResponse`
  - Prefers `/v1/responses` and falls back to `/v1/chat/completions`
  - Extracts assistant text from multiple compatible shapes
- adapter.rs — `promptbundle_to_messages_and_text`
  - Converts a `PromptBundle` into chat `messages` and a single fused `input` text
  - Character-based budgeting and trimming (UTF‑8 safe)
- provider.rs — `LlmGenerateProvider`
  - Native capability registered as `llm.generate` via the `ActionBroker`

## Prompt adapter details

Input: `PromptBundle { system, instructions, tools_json_schema, context_docs, history }` and `TokenBudget { max_input_tokens, max_output_tokens }`.

Algorithm:

1. Build a `Context:` block from `context_docs` (one item per line)
2. Keep `history` (user turns) oldest→newest and trim the oldest first to fit the budget
3. If still over budget, truncate `instructions` using character counts
4. Emit:
   - Chat messages: `system`, optional `Context:` (as system), each history line as `user`, and `instructions` as final `user`
   - Fused text for `/responses`: `System:`, `Context:`, `History:`, `User:` blocks

Budgeting: we approximate 4 characters per token to compute a conservative character budget from `max_input_tokens`. All slicing uses `chars()` to be UTF‑8 safe.

## HTTP client

Order of operations:

1. Try POST `{BASE_URL}/responses` with body `{ model, input, max_output_tokens, temperature }`
2. If not available or unparsable, POST `{BASE_URL}/chat/completions` with `{ model, messages, max_tokens, temperature }`

Text extraction supports common variants:

- Responses: `output_text` or `output[].content[].text.value`/`text`
- Chat: `choices[0].message.content`

Returned value is `LlmResponse { text, model, provider, usage, raw }`.

## Capability provider: `llm.generate`

The provider wraps `LlmClient::generate` and accepts a JSON payload:

```json
{
  "input": "optional plain text",
  "bundle": {
    "system": "...",
    "instructions": "...",
    "context_docs": ["..."],
    "history": ["..."]
  },
  "budget": { "max_input_tokens": 2048, "max_output_tokens": 512 }
}
```

Headers override (per-call):

- `model`: string
- `base_url`: string
- `temperature`: f32
- `request_timeout_ms`: u64

These map to `LlmClientConfig` for that call only.

## Environment variables

- VLLM_BASE_URL: Base URL (default: http://localhost:8000/v1)
- VLLM_MODEL: Model name (default: qwen2.5-0.5b-instruct)
- VLLM_API_KEY: Optional Bearer token
- REQUEST_TIMEOUT_MS: HTTP timeout in ms (default: 30000)
- VLLM_TEMPERATURE: Sampling temperature (default: 0.7)

## Usage example (sync)

```rust
use loom_core::context::{PromptBundle, TokenBudget};
use loom_core::llm::client::{LlmClient, LlmClientConfig};

let client = LlmClient::new(LlmClientConfig::default())?;
let bundle = PromptBundle {
		system: "You are Loom Agent.".into(),
		instructions: "Say hello.".into(),
		tools_json_schema: None,
		context_docs: vec![],
		history: vec![],
};
let res = client.generate(&bundle, Some(TokenBudget::default())).await?;
println!("{}", res.text);
```

## Notes and roadmap

- Aligns with P0 Voice Agent (see `docs/ROADMAP.md`): local LLM path for the E2E demo
- Streaming (SSE) and tool call plumbing can be added in P1
- A tokenizer-aware budgeter can replace the 4-char/token heuristic if needed
