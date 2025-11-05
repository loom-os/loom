//! LLM module root
//!
//! This file delegates to submodules in `llm/` and re-exports public types.

mod adapter;
mod client;
mod provider;

pub use adapter::promptbundle_to_messages_and_text;
pub use client::{LlmClient, LlmClientConfig, LlmResponse};
pub use provider::LlmGenerateProvider;
