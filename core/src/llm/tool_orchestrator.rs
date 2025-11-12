use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, info, warn, Span};

use crate::action_broker::ActionBroker;
use crate::context::{PromptBundle, TokenBudget};
use crate::proto::{ActionCall, ActionResult, ActionStatus, CapabilityDescriptor, QoSLevel};
use crate::{LoomError, Result};

use super::adapter::promptbundle_to_messages_and_text;
use super::client::LlmClient;

// OpenTelemetry imports
use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
    KeyValue,
};

/// How the model should use tools
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ToolChoice {
    /// Let the model decide whether to call a tool
    #[default]
    Auto,
    /// Require the model to call a tool at least once
    Required,
    /// Do not expose tools to the model
    None,
}

/// Orchestrator options controlling tool exposure and refinement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorOptions {
    pub tool_choice: ToolChoice,
    pub per_tool_timeout_ms: u64,
    pub refine_on_tool_result: bool,
    pub max_tools_exposed: usize,
}

impl Default for OrchestratorOptions {
    fn default() -> Self {
        Self {
            tool_choice: ToolChoice::Auto,
            per_tool_timeout_ms: 30_000,
            refine_on_tool_result: true,
            max_tools_exposed: 64,
        }
    }
}

/// Normalized tool call parsed from model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedToolCall {
    pub id: Option<String>,
    pub name: String,
    pub arguments: Value,
}

/// Final answer surfaced to the caller
#[derive(Debug, Clone)]
pub struct FinalAnswer {
    pub text: String,
    pub tool_calls: Vec<NormalizedToolCall>,
    pub tool_results: Vec<ActionResult>,
    pub raw_model: Option<Value>,
}

impl FinalAnswer {
    pub fn from_text(text: String) -> Self {
        Self {
            text,
            tool_calls: vec![],
            tool_results: vec![],
            raw_model: None,
        }
    }
}

/// Lightweight in-orchestrator counters for observability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolOrchestratorStats {
    pub total_invocations: u64,
    pub total_tool_calls: u64,
    pub total_tool_errors: u64,
    pub avg_tool_latency_ms: f64,
}

/// Orchestrates LLM tool use with the ActionBroker
pub struct ToolOrchestrator {
    llm: Arc<LlmClient>,
    broker: Arc<ActionBroker>,
    pub stats: ToolOrchestratorStats,

    // OpenTelemetry metrics
    runs_counter: Counter<u64>,
    tool_calls_counter: Counter<u64>,
    tool_errors_counter: Counter<u64>,
    refine_cycles_counter: Counter<u64>,
    tool_latency: Histogram<f64>,
    discovery_latency: Histogram<f64>,
    llm_latency: Histogram<f64>,
}

impl ToolOrchestrator {
    pub fn new(llm: Arc<LlmClient>, broker: Arc<ActionBroker>) -> Self {
        // Initialize OpenTelemetry metrics
        let meter = global::meter("loom.tool_orchestrator");

        let runs_counter = meter
            .u64_counter("loom.tool_orch.runs_total")
            .with_description("Total number of orchestrator runs")
            .init();

        let tool_calls_counter = meter
            .u64_counter("loom.tool_orch.tool_calls_total")
            .with_description("Total number of tool calls")
            .init();

        let tool_errors_counter = meter
            .u64_counter("loom.tool_orch.tool_errors_total")
            .with_description("Total number of tool errors")
            .init();

        let refine_cycles_counter = meter
            .u64_counter("loom.tool_orch.refine_cycles_total")
            .with_description("Total number of refine cycles")
            .init();

        let tool_latency = meter
            .f64_histogram("loom.tool_orch.tool_latency_ms")
            .with_description("Tool invocation latency in milliseconds")
            .init();

        let discovery_latency = meter
            .f64_histogram("loom.tool_orch.discovery_latency_ms")
            .with_description("Tool discovery latency in milliseconds")
            .init();

        let llm_latency = meter
            .f64_histogram("loom.tool_orch.llm_latency_ms")
            .with_description("LLM API latency in milliseconds")
            .init();

        Self {
            llm,
            broker,
            stats: ToolOrchestratorStats::default(),
            runs_counter,
            tool_calls_counter,
            tool_errors_counter,
            refine_cycles_counter,
            tool_latency,
            discovery_latency,
            llm_latency,
        }
    }

    /// Run the model with tools exposed; parse tool calls; invoke broker; optionally refine.
    /// Contract:
    /// - Input: PromptBundle + budget + options
    /// - Output: FinalAnswer (text, tool calls, results)
    #[tracing::instrument(name = "tool_orchestrator.run", skip(self, bundle), fields(tool_choice = ?options.tool_choice, tool_count, refine_enabled = options.refine_on_tool_result))]
    pub async fn run(
        &mut self,
        bundle: &PromptBundle,
        budget: Option<TokenBudget>,
        options: OrchestratorOptions,
        correlation_id: Option<String>,
    ) -> Result<FinalAnswer> {
        let budget = budget.unwrap_or_default();
        self.stats.total_invocations += 1;

        // Record run metric
        self.runs_counter.add(
            1,
            &[KeyValue::new(
                "tool_choice",
                format!("{:?}", options.tool_choice),
            )],
        );

        // Build tools array from capabilities
        let discovery_started = Instant::now();
        let caps = self.broker.list_capabilities();
        let tools = self.build_tools_for_llm(&caps, options.max_tools_exposed);
        let discovery_elapsed_ms = discovery_started.elapsed().as_secs_f64() * 1000.0;

        // Record discovery metrics
        self.discovery_latency.record(discovery_elapsed_ms, &[]);
        Span::current().record("tool_count", tools.len());

        debug!(target="tool_orch", count=%tools.len(), latency_ms=%discovery_elapsed_ms, "Tool discovery complete");
        let (messages, input_text) = promptbundle_to_messages_and_text(bundle, budget);

        // Prefer Responses API; fallback to Chat Completions
        let use_tools = !tools.is_empty() && options.tool_choice != ToolChoice::None;
        let resp_val = if use_tools {
            match self
                .post_responses_with_tools(&input_text, &tools, &options, budget)
                .await
            {
                Ok(v) => Some(v),
                Err(e) => {
                    warn!(target="tool_orch", error=%e, "Responses API with tools failed; trying chat.completions");
                    None
                }
            }
        } else {
            None
        };

        let (raw, parsed_calls, provider_tag) = if let Some(v) = resp_val {
            let calls = parse_tool_calls_from_responses(&v);
            (v, calls, "responses")
        } else {
            let chat_val = self
                .post_chat_with_tools(&messages, &tools, &options, budget)
                .await?;
            let calls = parse_tool_calls_from_chat(&chat_val);
            (chat_val, calls, "chat.completions")
        };

        debug!(target="tool_orch", provider=%provider_tag, calls=%parsed_calls.len(), "Parsed tool calls");

        if parsed_calls.is_empty() {
            let text = extract_text_fallback(&raw).ok_or_else(|| {
                LoomError::AgentError("No tool calls and no assistant text in model output".into())
            })?;
            return Ok(FinalAnswer {
                text,
                tool_calls: vec![],
                tool_results: vec![],
                raw_model: Some(raw),
            });
        }

        // Invoke tools sequentially for now
        let mut results: Vec<ActionResult> = Vec::new();
        for call in &parsed_calls {
            let started = Instant::now();
            let action_call =
                build_action_call(call, options.per_tool_timeout_ms, correlation_id.clone());
            let res = self.broker.invoke(action_call).await?;
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;

            // Update counters
            self.stats.total_tool_calls += 1;
            let status_str = if (res.status) == (ActionStatus::ActionOk as i32) {
                "success"
            } else {
                self.stats.total_tool_errors += 1;
                "error"
            };

            // Welford-like avg update
            let n = self.stats.total_tool_calls as f64;
            self.stats.avg_tool_latency_ms =
                ((self.stats.avg_tool_latency_ms * (n - 1.0)) + elapsed) / n;

            // Record metrics
            self.tool_calls_counter.add(
                1,
                &[
                    KeyValue::new("tool_name", call.name.clone()),
                    KeyValue::new("status", status_str),
                ],
            );

            self.tool_latency
                .record(elapsed, &[KeyValue::new("tool_name", call.name.clone())]);

            if status_str == "error" {
                let error_code = res
                    .error
                    .as_ref()
                    .map(|e| e.code.clone())
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                self.tool_errors_counter.add(
                    1,
                    &[
                        KeyValue::new("tool_name", call.name.clone()),
                        KeyValue::new("error_code", error_code),
                    ],
                );
            }

            info!(target="tool_orch", tool=%call.name, status=%res.status, latency_ms=%elapsed, "Tool invocation finished");
            results.push(res);
        }

        // Optional refine with tool results
        if options.refine_on_tool_result {
            let refine_bundle = make_refine_bundle(bundle, &parsed_calls, &results);
            let refine_started = Instant::now();
            let final_resp = self.llm.generate(&refine_bundle, Some(budget)).await?;
            let refine_elapsed_ms = refine_started.elapsed().as_secs_f64() * 1000.0;

            // Record refine cycle metric
            self.refine_cycles_counter.add(1, &[]);
            self.llm_latency
                .record(refine_elapsed_ms, &[KeyValue::new("api_type", "refine")]);

            debug!(target="tool_orch", latency_ms=%refine_elapsed_ms, "Refine turn finished");
            return Ok(FinalAnswer {
                text: final_resp.text,
                tool_calls: parsed_calls,
                tool_results: results,
                raw_model: Some(raw),
            });
        }

        // No refine: summarize results into a concise answer
        let text = summarize_results_for_user(&parsed_calls, &results);
        Ok(FinalAnswer {
            text,
            tool_calls: parsed_calls,
            tool_results: results,
            raw_model: Some(raw),
        })
    }

    fn build_tools_for_llm(&self, caps: &[CapabilityDescriptor], limit: usize) -> Vec<Value> {
        let mut tools = Vec::new();
        for cap in caps.iter().take(limit) {
            // Expect JSON schema for parameters in metadata["schema"], description in metadata["desc"]
            let desc = cap
                .metadata
                .get("desc")
                .cloned()
                .unwrap_or_else(|| cap.name.clone());
            let params: Value = cap
                .metadata
                .get("schema")
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or_else(
                    || json!({"type":"object","properties":{},"additionalProperties":true}),
                );
            tools.push(json!({
                "type": "function",
                "name": cap.name,
                "description": desc,
                "parameters": params,
            }));
        }
        tools
    }

    async fn post_responses_with_tools(
        &self,
        input_text: &str,
        tools: &[Value],
        options: &OrchestratorOptions,
        budget: TokenBudget,
    ) -> Result<Value> {
        let url = format!("{}/responses", self.llm.cfg.base_url.trim_end_matches('/'));
        let mut req = self
            .llm
            .http
            .post(&url)
            .header("content-type", "application/json");
        if let Some(key) = &self.llm.cfg.api_key {
            req = req.bearer_auth(key);
        }
        let tool_choice = match options.tool_choice {
            ToolChoice::Auto => json!({"type":"auto"}),
            ToolChoice::Required => json!({"type":"required"}),
            ToolChoice::None => json!({"type":"none"}),
        };
        let body = json!({
            "model": self.llm.cfg.model,
            "input": input_text,
            "tools": tools,
            "tool_choice": tool_choice,
            "max_output_tokens": budget.max_output_tokens as u32,
            "temperature": self.llm.cfg.temperature,
        });
        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| LoomError::AgentError(format!("Responses request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(LoomError::AgentError(format!(
                "Responses error: status={} body={}",
                status, text
            )));
        }
        resp.json::<Value>()
            .await
            .map_err(|e| LoomError::AgentError(format!("Failed to parse Responses JSON: {e}")))
    }

    async fn post_chat_with_tools(
        &self,
        messages: &[Value],
        tools: &[Value],
        options: &OrchestratorOptions,
        budget: TokenBudget,
    ) -> Result<Value> {
        let url = format!(
            "{}/chat/completions",
            self.llm.cfg.base_url.trim_end_matches('/')
        );
        let mut req = self
            .llm
            .http
            .post(&url)
            .header("content-type", "application/json");
        if let Some(key) = &self.llm.cfg.api_key {
            req = req.bearer_auth(key);
        }
        let tool_choice = match options.tool_choice {
            ToolChoice::Auto => json!({"type":"auto"}),
            ToolChoice::Required => json!({"type":"required"}),
            ToolChoice::None => json!(null),
        };
        let mut body = json!({
            "model": self.llm.cfg.model,
            "messages": messages,
            "max_tokens": budget.max_output_tokens as u32,
            "temperature": self.llm.cfg.temperature,
            "tools": tools,
        });
        if options.tool_choice != ToolChoice::None {
            body["tool_choice"] = tool_choice;
        }
        let resp =
            req.json(&body).send().await.map_err(|e| {
                LoomError::AgentError(format!("Chat Completions request failed: {e}"))
            })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(LoomError::AgentError(format!(
                "Chat Completions error: status={} body={}",
                status, text
            )));
        }
        resp.json::<Value>()
            .await
            .map_err(|e| LoomError::AgentError(format!("Failed to parse Chat JSON: {e}")))
    }
}

pub fn build_action_call(
    call: &NormalizedToolCall,
    timeout_ms: u64,
    correlation_id: Option<String>,
) -> ActionCall {
    ActionCall {
        id: new_call_id(),
        capability: call.name.clone(),
        version: String::new(),
        payload: serde_json::to_vec(&call.arguments).unwrap_or_default(),
        headers: {
            let mut m: std::collections::HashMap<String, String> = Default::default();
            if let Some(ref cid) = correlation_id {
                m.insert("correlation_id".into(), cid.clone());
            }
            m
        },
        timeout_ms: timeout_ms as i64,
        correlation_id: correlation_id.unwrap_or_default(),
        qos: QoSLevel::QosBatched as i32,
    }
}

fn new_call_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("call_{:x}", now)
}

pub fn make_refine_bundle(
    base: &PromptBundle,
    calls: &[NormalizedToolCall],
    results: &[ActionResult],
) -> PromptBundle {
    let mut bundle = base.clone();
    let mut context_block = String::from("Tool Results:\n");
    for (i, (c, r)) in calls.iter().zip(results.iter()).enumerate() {
        let status = r.status;
        if status == (ActionStatus::ActionOk as i32) {
            let preview = safe_snippet(&r.output);
            context_block.push_str(&format!("- {}: OK -> {}\n", c.name, preview));
        } else {
            let code = r.error.as_ref().map(|e| e.code.as_str()).unwrap_or("ERROR");
            context_block.push_str(&format!("- {}: {}\n", c.name, code));
        }
        if i >= 8 {
            break;
        }
    }
    // Prepend as context via system message (reusing existing mechanism)
    if bundle.system.is_empty() {
        bundle.system = context_block;
    } else {
        bundle.system.push_str("\n\n");
        bundle.system.push_str(&context_block);
    }
    bundle
}

fn summarize_results_for_user(calls: &[NormalizedToolCall], results: &[ActionResult]) -> String {
    // Simple human-readable summary
    let mut out = String::new();
    for (c, r) in calls.iter().zip(results.iter()) {
        if r.status == (ActionStatus::ActionOk as i32) {
            let snippet = safe_snippet(&r.output);
            out.push_str(&format!("{} → {}\n", c.name, snippet));
        } else {
            let msg = r
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "unknown error".into());
            out.push_str(&format!("{} → error: {}\n", c.name, msg));
        }
    }
    if out.is_empty() {
        "No tool results.".into()
    } else {
        out
    }
}

fn safe_snippet(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "<empty>".into();
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => {
            let s = s.trim();
            if s.len() > 280 {
                // Find safe UTF-8 boundary at or before 280 bytes
                let mut end = 280;
                while end > 0 && !s.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}…", &s[..end])
            } else {
                s.to_string()
            }
        }
        Err(_) => format!("{} bytes", bytes.len()),
    }
}

// Parsing helpers (public for testing)
pub fn parse_tool_calls_from_responses(v: &Value) -> Vec<NormalizedToolCall> {
    let mut calls = Vec::new();
    if let Some(outputs) = v.get("output").and_then(|x| x.as_array()) {
        for item in outputs {
            if let Some(contents) = item.get("content").and_then(|c| c.as_array()) {
                for c in contents {
                    if c.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        let name = c
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();
                        let id = c.get("id").and_then(|x| x.as_str()).map(|s| s.to_string());
                        let args = c.get("input").cloned().unwrap_or(json!({}));
                        if !name.is_empty() {
                            calls.push(NormalizedToolCall {
                                id,
                                name,
                                arguments: args,
                            });
                        }
                    }
                }
            }
        }
    }
    calls
}

pub fn parse_tool_calls_from_chat(v: &Value) -> Vec<NormalizedToolCall> {
    let mut calls = Vec::new();
    if let Some(arr) = v.get("choices").and_then(|x| x.as_array()) {
        if let Some(first) = arr.first() {
            if let Some(tc_arr) = first
                .get("message")
                .and_then(|m| m.get("tool_calls"))
                .and_then(|x| x.as_array())
            {
                for tc in tc_arr {
                    let id = tc.get("id").and_then(|x| x.as_str()).map(|s| s.to_string());
                    if let Some(func) = tc.get("function") {
                        let name = func
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();
                        let args_val = func.get("arguments");
                        let args = match args_val {
                            Some(Value::String(s)) => {
                                serde_json::from_str::<Value>(s).unwrap_or(json!({}))
                            }
                            Some(v) => v.clone(),
                            None => json!({}),
                        };
                        if !name.is_empty() {
                            calls.push(NormalizedToolCall {
                                id,
                                name,
                                arguments: args,
                            });
                        }
                    }
                }
            }
        }
    }
    calls
}

fn extract_text_fallback(v: &Value) -> Option<String> {
    // Responses style: output_text
    if let Some(s) = v.get("output_text").and_then(|x| x.as_str()) {
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    // Chat style
    if let Some(arr) = v.get("choices").and_then(|x| x.as_array()) {
        if let Some(first) = arr.first() {
            if let Some(s) = first
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
            {
                return Some(s.to_string());
            }
        }
    }
    None
}
