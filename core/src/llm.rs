use crate::context::{PromptBundle, TokenBudget};
use crate::{LoomError, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Configuration for LlmClient loaded from environment variables
#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    pub base_url: String, // e.g., http://localhost:8000/v1
    pub model: String,    // e.g., qwen2.5-0.5b-instruct
    pub api_key: Option<String>,
    pub request_timeout_ms: u64,
    pub temperature: f32,
}

impl Default for LlmClientConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("VLLM_BASE_URL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "http://localhost:8000/v1".to_string()),
            model: std::env::var("VLLM_MODEL")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "qwen2.5-0.5b-instruct".to_string()),
            api_key: std::env::var("VLLM_API_KEY").ok().filter(|s| !s.is_empty()),
            request_timeout_ms: std::env::var("REQUEST_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(30_000),
            temperature: std::env::var("VLLM_TEMPERATURE")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.7),
        }
    }
}

/// Minimal response containing the assistant text
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmResponse {
    pub text: String,
    pub model: Option<String>,
    pub provider: Option<String>, // "responses" or "chat.completions"
    pub usage: Option<serde_json::Value>,
    pub raw: Option<serde_json::Value>,
}

/// HTTP client that prefers the OpenAI Responses API and falls back to Chat Completions
#[derive(Clone)]
pub struct LlmClient {
    http: Client,
    cfg: LlmClientConfig,
}

impl LlmClient {
    pub fn new(cfg: LlmClientConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_millis(cfg.request_timeout_ms))
            .build()
            .map_err(|e| LoomError::AgentError(format!("Failed to build HTTP client: {e}")))?;
        Ok(Self { http, cfg })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(LlmClientConfig::default())
    }

    /// Generate a completion for the given prompt bundle
    /// Contract:
    /// - Input: PromptBundle + optional budget
    /// - Output: LlmResponse with assistant text
    /// - Error: network/parse; safe fallbacks are attempted before erroring
    pub async fn generate(
        &self,
        bundle: &PromptBundle,
        budget: Option<TokenBudget>,
    ) -> Result<LlmResponse> {
        // Prepare payloads
        let budget = budget.unwrap_or_default();
        let (messages, input_text) = promptbundle_to_messages_and_text(bundle, budget);

        // Try Responses API first
        let responses_url = format!("{}/responses", self.cfg.base_url.trim_end_matches('/'));
        debug!(
            target = "llm_client",
            "POST {} via Responses API", responses_url
        );

        let mut req = self
            .http
            .post(&responses_url)
            .header("content-type", "application/json");
        if let Some(key) = &self.cfg.api_key {
            req = req.bearer_auth(key);
        }

        // Build Responses API body (prefer the unified input field)
        let body = json!({
            "model": self.cfg.model,
            "input": input_text,
            // The Responses API uses max_output_tokens
            "max_output_tokens": budget.max_output_tokens as u32,
            "temperature": self.cfg.temperature,
        });

        match req.json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let val: serde_json::Value = resp.json().await.map_err(|e| {
                        LoomError::AgentError(format!("Failed to parse Responses API JSON: {e}"))
                    })?;
                    if let Some(text) = extract_text_from_responses(&val) {
                        return Ok(LlmResponse {
                            text,
                            model: val
                                .get("model")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            provider: Some("responses".to_string()),
                            usage: val.get("usage").cloned(),
                            raw: Some(val),
                        });
                    } else {
                        warn!(target = "llm_client", "Responses API returned unexpected schema; will try chat.completions fallback");
                    }
                } else {
                    warn!(target = "llm_client", status = %resp.status(), "Responses API non-success; will try chat.completions fallback");
                }
            }
            Err(err) => {
                // Fallback on network error
                warn!(target = "llm_client", error = %err, "Responses API request failed; trying chat.completions fallback");
            }
        }

        // Fallback to Chat Completions
        let chat_url = format!(
            "{}/chat/completions",
            self.cfg.base_url.trim_end_matches('/')
        );
        debug!(
            target = "llm_client",
            "POST {} via Chat Completions", chat_url
        );

        let mut req = self
            .http
            .post(&chat_url)
            .header("content-type", "application/json");
        if let Some(key) = &self.cfg.api_key {
            req = req.bearer_auth(key);
        }

        let body = json!({
            "model": self.cfg.model,
            "messages": messages,
            "max_tokens": budget.max_output_tokens as u32,
            "temperature": self.cfg.temperature,
        });

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| LoomError::AgentError(format!("Chat Completions HTTP error: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!(target = "llm_client", %status, body = %text, "Chat Completions error");
            return Err(LoomError::AgentError(format!(
                "Chat Completions error: status={} body={}",
                status, text
            )));
        }

        let val: serde_json::Value = resp.json().await.map_err(|e| {
            LoomError::AgentError(format!("Failed to parse Chat Completions JSON: {e}"))
        })?;
        let text = extract_text_from_chat_completions(&val).ok_or_else(|| {
            LoomError::AgentError("Missing choices[0].message.content in chat completions".into())
        })?;
        Ok(LlmResponse {
            text,
            model: val
                .get("model")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            provider: Some("chat.completions".to_string()),
            usage: val.get("usage").cloned(),
            raw: Some(val),
        })
    }
}

/// Convert PromptBundle into both chat messages and a single fused text for the Responses API
pub fn promptbundle_to_messages_and_text(
    bundle: &PromptBundle,
    budget: TokenBudget,
) -> (Vec<serde_json::Value>, String) {
    // Approximate token->char ratio; conservative safety factor ~4 chars/token
    let char_budget = budget.max_input_tokens.saturating_mul(4);

    let mut system = bundle.system.clone();
    let mut context_block = String::new();
    if !bundle.context_docs.is_empty() {
        context_block.push_str("Context:\n");
        for d in &bundle.context_docs {
            context_block.push_str("- ");
            context_block.push_str(d);
            context_block.push('\n');
        }
    }
    let mut history_blocks: Vec<String> = bundle.history.clone();
    let mut instructions = bundle.instructions.clone();

    // Trim oldest history first until within char budget
    let mut assemble_len = system.len()
        + context_block.len()
        + instructions.len()
        + history_blocks.iter().map(|s| s.len()).sum::<usize>();
    while assemble_len > char_budget && !history_blocks.is_empty() {
        let removed = history_blocks.remove(0);
        assemble_len -= removed.len();
    }
    // If still too large, truncate instructions
    if assemble_len > char_budget && !instructions.is_empty() {
        let keep = instructions
            .char_indices()
            .take_while(|(i, _)| {
                *i < char_budget.saturating_sub(system.len() + context_block.len())
            })
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        instructions.truncate(keep);
    }

    // Build chat messages (system + optional context + history as user + user instructions)
    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(json!({"role": "system", "content": system}));
    }
    if !context_block.is_empty() {
        messages.push(json!({"role": "system", "content": context_block.clone()}));
    }
    for h in &history_blocks {
        messages.push(json!({"role": "user", "content": h}));
    }
    if !instructions.is_empty() {
        messages.push(json!({"role": "user", "content": instructions.clone()}));
    }

    // Fused text for Responses API input
    let mut fused = String::new();
    if !system.is_empty() {
        fused.push_str("System:\n");
        fused.push_str(&system);
        fused.push_str("\n\n");
    }
    if !context_block.is_empty() {
        fused.push_str(&context_block);
        fused.push('\n');
    }
    if !history_blocks.is_empty() {
        fused.push_str("History:\n");
        for h in &history_blocks {
            fused.push_str("- ");
            fused.push_str(h);
            fused.push('\n');
        }
        fused.push('\n');
    }
    if !instructions.is_empty() {
        fused.push_str("User:\n");
        fused.push_str(&instructions);
        fused.push('\n');
    }

    (messages, fused)
}

fn extract_text_from_chat_completions(v: &serde_json::Value) -> Option<String> {
    v.get("choices")?
        .get(0)?
        .get("message")?
        .get("content")?
        .as_str()
        .map(|s| s.to_string())
}

fn extract_text_from_responses(v: &serde_json::Value) -> Option<String> {
    // Prefer a direct output_text if present
    if let Some(s) = v.get("output_text").and_then(|x| x.as_str()) {
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    // Otherwise, try unified output array schema
    if let Some(arr) = v.get("output").and_then(|x| x.as_array()) {
        let mut acc = String::new();
        for item in arr {
            if let Some(contents) = item.get("content").and_then(|c| c.as_array()) {
                for c in contents {
                    if let Some(t) = c.get("text").and_then(|t| t.as_str()) {
                        if !acc.is_empty() {
                            acc.push_str("\n");
                        }
                        acc.push_str(t);
                    } else if let Some(t) = c.get("output_text").and_then(|t| t.as_str()) {
                        if !acc.is_empty() {
                            acc.push_str("\n");
                        }
                        acc.push_str(t);
                    }
                }
            }
        }
        if !acc.is_empty() {
            return Some(acc);
        }
    }
    // Some implementations may return choices like chat
    if let Some(s) = extract_text_from_chat_completions(v) {
        return Some(s);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_truncates_history() {
        let bundle = PromptBundle {
            system: "sys".into(),
            instructions: "do it".into(),
            tools_json_schema: None,
            context_docs: vec!["doc1".into()],
            history: vec!["a".repeat(2000), "b".repeat(2000), "c".repeat(10)],
        };
        let budget = TokenBudget {
            max_input_tokens: 512,
            max_output_tokens: 32,
        };
        let (_messages, fused) = promptbundle_to_messages_and_text(&bundle, budget);
        // Expect that fused is under char budget (~2048 chars) and contains 'c' but likely not both 'a' and 'b'
        assert!(fused.len() <= budget.max_input_tokens * 4 + 64); // allow small overhead
        assert!(fused.contains(&"c".repeat(10)));
    }
}

// ---- Capability Provider: llm.generate ----

use crate::action_broker::CapabilityProvider;
use crate::proto::{
    ActionCall, ActionError, ActionResult, ActionStatus, CapabilityDescriptor, ProviderKind,
};
use async_trait::async_trait;
use std::sync::Arc;

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
            let mut cfg = (*self.client).cfg.clone();
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

impl LlmGenerateProvider {
    async fn invoke_with_client(
        &self,
        call: ActionCall,
        client: &LlmClient,
    ) -> crate::Result<ActionResult> {
        let payload: GeneratePayload = match serde_json::from_slice(&call.payload) {
            Ok(v) => v,
            Err(_) => GeneratePayload {
                input: String::new(),
                bundle: None,
                budget: None,
            },
        };

        let bundle = if let Some(b) = payload.bundle {
            b
        } else {
            // Build a minimal bundle from input
            PromptBundle {
                system: String::new(),
                instructions: payload.input,
                tools_json_schema: None,
                context_docs: vec![],
                history: vec![],
            }
        };

        let res = client.generate(&bundle, payload.budget).await;
        match res {
            Ok(r) => Ok(ActionResult {
                id: call.id,
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
                id: call.id,
                status: ActionStatus::ActionError as i32,
                output: Vec::new(),
                error: Some(ActionError {
                    code: "LLM_ERROR".into(),
                    message: e.to_string(),
                    details: Default::default(),
                }),
            }),
        }
    }
}
