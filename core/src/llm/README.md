# LLM Module

This module contains the HTTP client, prompt adapter, and capability provider used to communicate with OpenAI-compatible backends.

## Components

- client.rs: `LlmClient`, `LlmClientConfig`, and `LlmResponse`. Prefers the `/v1/responses` API and falls back to `/v1/chat/completions`.
- adapter.rs: `promptbundle_to_messages_and_text`, which converts a `PromptBundle` into both chat `messages` and a fused `input` string for the Responses API, with simple token-budget based trimming.
- provider.rs: `LlmGenerateProvider`, a native capability provider registered as `llm.generate` via the `ActionBroker`.

## Environment variables

- VLLM_BASE_URL: Base URL for the OpenAI-compatible server (default: http://localhost:8000/v1)
- VLLM_MODEL: Model name (default: qwen2.5-0.5b-instruct)
- VLLM_API_KEY: Optional Bearer token (default: empty)
- REQUEST_TIMEOUT_MS: HTTP request timeout in milliseconds (default: 30000)
- VLLM_TEMPERATURE: Sampling temperature (default: 0.7)

## Notes

- This aligns with the P0 Voice Agent milestone in `docs/ROADMAP.md` by enabling a reliable local LLM path for the E2E demo.
- The client favors compatibility: Responses API first, then Chat Completions.
- Keep the adapter small and predictable—token budgeting is conservative and character-based.
