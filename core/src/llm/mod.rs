//! LLM module: HTTP client, prompt adapter, and capability provider
//!
//! This module provides:
//! - `LlmClientConfig`, `LlmClient`, `LlmResponse` for talking to OpenAI-compatible backends
//! - `promptbundle_to_messages_and_text` adapter for turning `PromptBundle` into payloads
//! - `LlmGenerateProvider` capability provider registered as `llm.generate`

mod adapter;
mod client;
mod provider;
mod tool_orchestrator;

pub use adapter::promptbundle_to_messages_and_text;
pub use client::{LlmClient, LlmClientConfig, LlmResponse};
pub use provider::LlmGenerateProvider;
pub use tool_orchestrator::{
    build_action_call, make_refine_bundle, parse_tool_calls_from_chat,
    parse_tool_calls_from_responses, FinalAnswer, NormalizedToolCall, OrchestratorOptions,
    ToolChoice, ToolOrchestrator, ToolOrchestratorStats,
};
