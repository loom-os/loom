use crate::context::{PromptBundle, TokenBudget};
use crate::{LoomError, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error, warn};

use super::adapter::promptbundle_to_messages_and_text;

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
    pub(crate) http: Client,
    pub(crate) cfg: LlmClientConfig,
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
                        LoomError::AgentError(format!("Failed to parse Responses JSON: {e}"))
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
                    }
                    // fallthrough to chat if we couldn't parse
                } else if resp.status() == StatusCode::NOT_FOUND {
                    // Endpoint missing; try chat fallback
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    warn!(target = "llm_client", %status, body = %body, "Responses API error; trying chat.completions fallback");
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
                    // Collect text segments from various shapes
                    if let Some(t) = c
                        .get("text")
                        .and_then(|t| t.get("value"))
                        .and_then(|v| v.as_str())
                    {
                        acc.push_str(t);
                    } else if let Some(t) = c.get("text").and_then(|v| v.as_str()) {
                        acc.push_str(t);
                    } else if let Some(t) = c.get("content").and_then(|v| v.as_str()) {
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
