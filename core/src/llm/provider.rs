use crate::action_broker::CapabilityProvider;
use crate::context::PromptBundle;
use crate::context::TokenBudget;
use crate::proto::{
    ActionCall, ActionError, ActionResult, ActionStatus, CapabilityDescriptor, ProviderKind,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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

    async fn invoke_with_client(
        &self,
        call: ActionCall,
        client: &LlmClient,
    ) -> crate::Result<ActionResult> {
        let payload: GeneratePayload = match serde_json::from_slice(&call.payload) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ActionResult {
                    id: call.id.clone(),
                    status: ActionStatus::ActionError as i32,
                    output: Vec::new(),
                    error: Some(ActionError {
                        code: "DESERIALIZATION_ERROR".to_string(),
                        message: format!("Failed to deserialize payload: {}", e),
                        details: Default::default(),
                    }),
                });
            }
        };

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

        let res = client.generate(&bundle, payload.budget).await;
        match res {
            Ok(r) => Ok(ActionResult {
                id: call.id.clone(),
                status: ActionStatus::ActionOk as i32,
                output: serde_json::to_vec(&serde_json::json!({
                    "text": r.text,
                    "model": r.model,
                    "provider": r.provider,
                    "usage": r.usage,
                }))
                .unwrap_or_default(),
                error: None,
            }),
            Err(e) => Ok(ActionResult {
                id: call.id.clone(),
                status: ActionStatus::ActionError as i32,
                output: Vec::new(),
                error: Some(ActionError {
                    code: "LLM_ERROR".to_string(),
                    message: e.to_string(),
                    details: Default::default(),
                }),
            }),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct GeneratePayload {
    #[serde(default)]
    input: String,
    #[serde(default)]
    bundle: Option<PromptBundle>,
    #[serde(default)]
    budget: Option<TokenBudget>,
}

#[async_trait]
impl CapabilityProvider for LlmGenerateProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        CapabilityDescriptor {
            name: "llm.generate".to_string(),
            version: "0.1.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata: Default::default(),
        }
    }

    async fn invoke(&self, call: ActionCall) -> crate::Result<ActionResult> {
        // Allow headers to override config dynamically
        // Keys: model, base_url, temperature, request_timeout_ms
        if !call.headers.is_empty() {
            let mut cfg = self.client.cfg.clone();
            if let Some(v) = call.headers.get("model") {
                cfg.model = v.clone();
            }
            if let Some(v) = call.headers.get("base_url") {
                cfg.base_url = v.clone();
            }
            if let Some(v) = call
                .headers
                .get("temperature")
                .and_then(|s| s.parse::<f32>().ok())
            {
                cfg.temperature = v;
            }
            if let Some(v) = call
                .headers
                .get("request_timeout_ms")
                .and_then(|s| s.parse::<u64>().ok())
            {
                cfg.request_timeout_ms = v;
            }
            // rebuild client with new cfg for this call
            let temp_client = LlmClient::new(cfg)?;
            return self.invoke_with_client(call, &temp_client).await;
        }
        self.invoke_with_client(call, self.client.as_ref()).await
    }
}
