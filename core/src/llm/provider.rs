use crate::context::PromptBundle;
use crate::context::TokenBudget;
use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use super::client::LlmClient;

/// Native Action provider that wraps LlmClient
pub struct LlmGenerateProvider {
    client: Arc<LlmClient>,
}

impl LlmGenerateProvider {
    pub fn new(client: Option<LlmClient>) -> crate::Result<Self> {
        let c = match client {
            Some(c) => c,
            None => LlmClient::from_env()?,
        };
        Ok(Self {
            client: Arc::new(c),
        })
    }
}

#[derive(Debug, Deserialize)]
struct GeneratePayload {
    input: String,
    bundle: Option<PromptBundle>,
    budget: Option<TokenBudget>,
}

#[async_trait]
impl Tool for LlmGenerateProvider {
    fn name(&self) -> String {
        "llm:generate".to_string()
    }

    fn description(&self) -> String {
        "Generate text using an LLM".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input text or instructions"
                },
                "bundle": {
                    "type": "object",
                    "description": "Full prompt bundle (optional)"
                },
                "budget": {
                    "type": "object",
                    "description": "Token budget (optional)"
                }
            },
            "required": ["input"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let payload: GeneratePayload = serde_json::from_value(arguments)
            .map_err(|e| ToolError::InvalidArguments(format!("Invalid arguments: {}", e)))?;

        let bundle = if let Some(b) = payload.bundle {
            b
        } else {
            // Build a minimal bundle from input
            PromptBundle {
                system: String::new(),
                instructions: payload.input,
                tools_json_schema: None,
                context_docs: Vec::new(),
                history: Vec::new(),
            }
        };

        let res = self
            .client
            .generate(&bundle, payload.budget)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("LLM generation failed: {}", e)))?;

        Ok(json!({
            "text": res.text,
            "model": res.model,
            "provider": res.provider,
            "usage": res.usage
        }))
    }
}
