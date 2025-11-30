//! Simple implementation of the CognitiveLoop trait.
//!
//! This provides a ready-to-use cognitive loop that integrates with
//! the LLM client and ActionBroker for tool use.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::{debug, info, warn};

use crate::context::PromptBundle;
use crate::llm::LlmClient;
use crate::proto::{AgentState, Event};
use crate::tools::ToolRegistry;
use crate::Result;

use super::config::{CognitiveConfig, ThinkingStrategy};
use super::loop_trait::{CognitiveLoop, ExecutionResult, Perception};
use super::thought::{Observation, Plan, ThoughtStep, ToolCall};
use super::working_memory::WorkingMemory;

/// A simple implementation of the CognitiveLoop trait.
///
/// This implementation provides:
/// - Context building from working memory
/// - LLM-based thinking with optional tool use
/// - Tool execution via ActionBroker
/// - Support for SingleShot and ReAct strategies
///
/// # Example
///
/// ```rust,ignore
/// use loom_core::agent::cognitive::{SimpleCognitiveLoop, CognitiveConfig, CognitiveAgent};
/// use loom_core::llm::LlmClient;
/// use loom_core::ActionBroker;
/// use std::sync::Arc;
///
/// let config = CognitiveConfig::react();
/// let llm = Arc::new(LlmClient::from_env()?);
/// let broker = Arc::new(ActionBroker::new());
///
/// let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);
/// let behavior = CognitiveAgent::new(loop_impl);
/// ```
pub struct SimpleCognitiveLoop {
    /// Configuration
    config: CognitiveConfig,

    /// LLM client for generating thoughts
    llm: Arc<LlmClient>,

    /// ToolRegistry for tool execution
    tools: Arc<ToolRegistry>,

    /// Working memory for this agent
    memory: WorkingMemory,

    /// Correlation ID for tracing
    correlation_id: Option<String>,
}

impl SimpleCognitiveLoop {
    /// Create a new SimpleCognitiveLoop
    pub fn new(config: CognitiveConfig, llm: Arc<LlmClient>, tools: Arc<ToolRegistry>) -> Self {
        let memory = WorkingMemory::new(config.memory_window_size);
        Self {
            config,
            llm,
            tools,
            memory,
            correlation_id: None,
        }
    }

    /// Set the correlation ID for tracing
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Build a PromptBundle from the current context
    fn build_prompt(&self, perception: &Perception, plan: &Plan) -> PromptBundle {
        let system = self.config.system_prompt.clone().unwrap_or_else(|| {
            "You are a helpful AI assistant. Think step by step and use available tools when needed.".to_string()
        });

        // Build context from memory and perception
        let mut context_docs = Vec::new();

        // Add memory context
        let memory_context = self.memory.to_context_string();
        if !memory_context.is_empty() {
            context_docs.push(format!("Recent conversation:\n{}", memory_context));
        }

        // Add perception context
        if !perception.context.is_empty() {
            context_docs.push(format!("Context:\n{}", perception.context.join("\n")));
        }

        // Add previous reasoning steps if in ReAct mode
        if !plan.steps.is_empty() {
            let steps_text = plan
                .steps
                .iter()
                .map(|s| {
                    let mut text = format!("Thought {}: {}", s.step, s.reasoning);
                    if let Some(ref tc) = s.tool_call {
                        text.push_str(&format!("\nAction: {} with {}", tc.name, tc.arguments));
                    }
                    if let Some(ref obs) = s.observation {
                        if obs.success {
                            text.push_str(&format!("\nObservation: {}", obs.output));
                        } else {
                            text.push_str(&format!(
                                "\nObservation: Error - {}",
                                obs.error.as_deref().unwrap_or("unknown")
                            ));
                        }
                    }
                    text
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            context_docs.push(format!("Previous reasoning:\n{}", steps_text));
        }

        // Build instructions based on strategy
        let instructions = match self.config.thinking_strategy {
            ThinkingStrategy::SingleShot => {
                format!(
                    "User request: {}\n\nProvide a helpful response.",
                    perception
                        .goal
                        .as_deref()
                        .unwrap_or("(no specific request)")
                )
            }
            ThinkingStrategy::ReAct => {
                let tools_list = perception.available_tools.join(", ");
                format!(
                    "User request: {}\n\n\
                    Available tools: {}\n\n\
                    Think step by step. For each step:\n\
                    1. Thought: Explain your reasoning\n\
                    2. Action: If needed, specify a tool to call as JSON: {{\"tool\": \"name\", \"args\": {{}}}}\n\
                    3. When you have the final answer, respond with: FINAL ANSWER: <your answer>\n\n\
                    Begin:",
                    perception.goal.as_deref().unwrap_or("(no specific request)"),
                    if tools_list.is_empty() { "none" } else { &tools_list }
                )
            }
            ThinkingStrategy::ChainOfThought => {
                format!(
                    "User request: {}\n\n\
                    Let's think through this step by step:\n\
                    1. First, I'll identify what we need to do\n\
                    2. Then, I'll work through the logic\n\
                    3. Finally, I'll provide the answer\n\n\
                    Begin:",
                    perception
                        .goal
                        .as_deref()
                        .unwrap_or("(no specific request)")
                )
            }
        };

        // Build tools JSON schema if tools are available
        let tools_schema = if !perception.available_tools.is_empty() {
            let tools: Vec<Value> = perception
                .available_tools
                .iter()
                .take(self.config.max_tools_exposed)
                .map(|name| {
                    json!({
                        "name": name,
                        "description": format!("Tool: {}", name),
                        "parameters": {"type": "object"}
                    })
                })
                .collect();
            Some(serde_json::to_string(&tools).unwrap_or_default())
        } else {
            None
        };

        PromptBundle {
            system,
            instructions,
            tools_json_schema: tools_schema,
            context_docs,
            history: vec![],
        }
    }

    /// Parse LLM response to extract thought, tool call, or final answer
    pub(crate) fn parse_llm_response(&self, text: &str) -> ParsedResponse {
        let text = text.trim();

        // Check for final answer
        if let Some(idx) = text.to_uppercase().find("FINAL ANSWER:") {
            let answer = text[idx + 13..].trim().to_string();
            return ParsedResponse::FinalAnswer(answer);
        }

        // Try to find tool call JSON
        if let Some(tool_call) = self.extract_tool_call(text) {
            // Extract reasoning before the tool call
            let reasoning = text
                .split('{')
                .next()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            return ParsedResponse::ToolCall {
                reasoning,
                tool_call,
            };
        }

        // Default: treat as reasoning
        ParsedResponse::Reasoning(text.to_string())
    }

    /// Extract a tool call from text
    pub(crate) fn extract_tool_call(&self, text: &str) -> Option<ToolCall> {
        // Look for JSON-like tool call patterns
        // Pattern 1: {"tool": "name", "args": {...}}
        // Pattern 2: {"action": "name", "arguments": {...}}

        // Find JSON object boundaries
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        if end <= start {
            return None;
        }

        let json_str = &text[start..=end];
        let parsed: Value = serde_json::from_str(json_str).ok()?;

        // Try different patterns
        let (name, args) = if let Some(tool) = parsed.get("tool") {
            let name = tool.as_str()?.to_string();
            let args = parsed.get("args").cloned().unwrap_or(json!({}));
            (name, args)
        } else if let Some(action) = parsed.get("action") {
            let name = action.as_str()?.to_string();
            let args = parsed
                .get("arguments")
                .or_else(|| parsed.get("input"))
                .cloned()
                .unwrap_or(json!({}));
            (name, args)
        } else if let Some(name) = parsed.get("name") {
            let name = name.as_str()?.to_string();
            let args = parsed
                .get("arguments")
                .or_else(|| parsed.get("input"))
                .or_else(|| parsed.get("parameters"))
                .cloned()
                .unwrap_or(json!({}));
            (name, args)
        } else {
            return None;
        };

        Some(ToolCall::new(name, args))
    }

    /// Execute a single tool call via ToolRegistry
    async fn execute_tool(&self, tool_call: &ToolCall) -> Observation {
        let started = Instant::now();

        match self
            .tools
            .call(&tool_call.name, tool_call.arguments.clone())
            .await
        {
            Ok(result) => {
                let latency_ms = started.elapsed().as_millis() as u64;
                let output =
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());
                Observation::success(&tool_call.name, output, latency_ms)
            }
            Err(e) => {
                let latency_ms = started.elapsed().as_millis() as u64;
                Observation::error(&tool_call.name, e.to_string(), latency_ms)
            }
        }
    }

    /// Get available tools from ActionBroker
    fn get_available_tools(&self) -> Vec<String> {
        self.tools
            .list_tools()
            .into_iter()
            .take(self.config.max_tools_exposed)
            .map(|tool| tool.name())
            .collect()
    }
}

/// Parsed response from LLM
#[derive(Debug)]
pub(crate) enum ParsedResponse {
    FinalAnswer(String),
    ToolCall {
        reasoning: String,
        tool_call: ToolCall,
    },
    Reasoning(String),
}

#[async_trait]
impl CognitiveLoop for SimpleCognitiveLoop {
    async fn perceive(&mut self, event: Event, _state: &AgentState) -> Result<Perception> {
        debug!(
            target = "cognitive.perceive",
            event_id = %event.id,
            event_type = %event.r#type,
            "Perceiving event"
        );

        // Build base perception from event
        let mut perception = Perception::from_event(event);

        // Add available tools
        perception.available_tools = self.get_available_tools();

        // Add context from working memory
        let recent = self.memory.recent(5);
        perception.context = recent
            .into_iter()
            .map(|item| item.content.clone())
            .collect();

        debug!(
            target = "cognitive.perceive",
            goal = ?perception.goal,
            context_items = perception.context.len(),
            tools = perception.available_tools.len(),
            "Perception complete"
        );

        Ok(perception)
    }

    async fn think(&mut self, perception: &Perception) -> Result<Plan> {
        info!(
            target = "cognitive.think",
            strategy = ?self.config.thinking_strategy,
            goal = ?perception.goal,
            "Starting think phase"
        );

        let mut plan = Plan::with_goal(perception.goal.clone().unwrap_or_default());

        match self.config.thinking_strategy {
            ThinkingStrategy::SingleShot => {
                // Single LLM call, no tool use
                let bundle = self.build_prompt(perception, &plan);
                let response = self.llm.generate(&bundle, None).await?;

                plan.complete_with_answer(&response.text);
            }

            ThinkingStrategy::ReAct | ThinkingStrategy::ChainOfThought => {
                // Iterative reasoning with potential tool use
                for iteration in 0..self.config.max_iterations {
                    debug!(
                        target = "cognitive.think",
                        iteration = iteration,
                        "ReAct iteration"
                    );

                    let bundle = self.build_prompt(perception, &plan);
                    let response = self.llm.generate(&bundle, None).await?;

                    match self.parse_llm_response(&response.text) {
                        ParsedResponse::FinalAnswer(answer) => {
                            plan.complete_with_answer(answer);
                            break;
                        }
                        ParsedResponse::ToolCall {
                            reasoning,
                            tool_call,
                        } => {
                            // Add thought step with tool call
                            plan.add_step(ThoughtStep::with_tool(
                                iteration + 1,
                                reasoning,
                                tool_call,
                            ));
                            // Don't execute here - that's for the act phase
                        }
                        ParsedResponse::Reasoning(text) => {
                            // If no tool call and this is the last iteration, use as answer
                            if iteration == self.config.max_iterations - 1 {
                                plan.complete_with_answer(&text);
                            } else {
                                // Just reasoning, no tool call
                                plan.add_step(ThoughtStep::reasoning(iteration + 1, text));
                            }
                        }
                    }

                    // If we have pending tool calls, break to let act phase handle them
                    if plan.has_pending_tools() {
                        break;
                    }
                }
            }
        }

        info!(
            target = "cognitive.think",
            steps = plan.steps.len(),
            complete = plan.complete,
            has_pending_tools = plan.has_pending_tools(),
            "Think phase complete"
        );

        Ok(plan)
    }

    async fn act(&mut self, plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult> {
        info!(
            target = "cognitive.act",
            pending_tools = plan.pending_tool_calls().len(),
            "Starting act phase"
        );

        // Clone plan to allow modifications
        let mut plan = plan.clone();

        // Execute pending tool calls
        for step in &mut plan.steps {
            if let Some(ref tool_call) = step.tool_call {
                if step.observation.is_none() {
                    debug!(
                        target = "cognitive.act",
                        tool = %tool_call.name,
                        "Executing tool"
                    );

                    let observation = self.execute_tool(tool_call).await;

                    // Add to working memory
                    if observation.success {
                        self.memory
                            .add_observation(&tool_call.name, &observation.output);
                    }

                    step.observation = Some(observation);
                }
            }
        }

        // If we executed tools and refinement is enabled, do another think cycle
        if self.config.refine_after_tools && !plan.observations().is_empty() && !plan.complete {
            // Build a refinement prompt with tool results
            let bundle = self.build_prompt(
                &Perception::from_event(Event::default()).with_context(vec![plan.to_summary()]),
                &plan,
            );

            if let Ok(response) = self.llm.generate(&bundle, None).await {
                plan.complete_with_answer(&response.text);
            }
        }

        // Update agent state metadata
        state.last_update_ms = chrono::Utc::now().timestamp_millis();
        state
            .metadata
            .insert("last_goal".to_string(), plan.goal.clone());

        // Build result
        let result = ExecutionResult {
            goal_achieved: plan.complete,
            response: plan.final_answer.clone(),
            ..Default::default()
        };

        // Add response to memory if we have one
        if let Some(ref response) = result.response {
            self.memory.add_agent_response(response);
        }

        info!(
            target = "cognitive.act",
            goal_achieved = result.goal_achieved,
            "Act phase complete"
        );

        Ok(result)
    }

    async fn reflect(
        &mut self,
        perception: &Perception,
        plan: &Plan,
        result: &ExecutionResult,
    ) -> Result<Option<String>> {
        if !self.config.enable_reflection {
            return Ok(None);
        }

        debug!(
            target = "cognitive.reflect",
            goal_achieved = result.goal_achieved,
            "Starting reflection"
        );

        // Simple self-evaluation prompt
        let reflection_prompt = format!(
            "Goal: {}\n\
            Plan summary:\n{}\n\
            Outcome: {}\n\n\
            Briefly evaluate: Was the goal achieved? What could be improved?",
            perception.goal.as_deref().unwrap_or("(none)"),
            plan.to_summary(),
            if result.goal_achieved {
                "Success"
            } else {
                "Not achieved"
            }
        );

        let bundle = PromptBundle {
            system: "You are a reflective AI evaluating your own performance. Be concise."
                .to_string(),
            instructions: reflection_prompt,
            tools_json_schema: None,
            context_docs: vec![],
            history: vec![],
        };

        match self.llm.generate(&bundle, None).await {
            Ok(response) => {
                debug!(
                    target = "cognitive.reflect",
                    reflection = %response.text,
                    "Reflection complete"
                );
                Ok(Some(response.text))
            }
            Err(e) => {
                warn!(
                    target = "cognitive.reflect",
                    error = %e,
                    "Reflection failed"
                );
                Ok(None)
            }
        }
    }

    fn working_memory(&self) -> &WorkingMemory {
        &self.memory
    }

    fn working_memory_mut(&mut self) -> &mut WorkingMemory {
        &mut self.memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require mocking LlmClient and ActionBroker.
    // These unit tests focus on parsing and helper functions.

    #[test]
    fn test_parse_final_answer() {
        let config = CognitiveConfig::default();
        let llm = Arc::new(LlmClient::from_env().unwrap());
        let broker = Arc::new(ActionBroker::new());
        let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);

        let text = "After considering the options, FINAL ANSWER: The capital of France is Paris.";
        match loop_impl.parse_llm_response(text) {
            ParsedResponse::FinalAnswer(answer) => {
                assert!(answer.contains("Paris"));
            }
            _ => panic!("Expected FinalAnswer"),
        }
    }

    #[test]
    fn test_parse_tool_call() {
        let config = CognitiveConfig::default();
        let llm = Arc::new(LlmClient::from_env().unwrap());
        let broker = Arc::new(ActionBroker::new());
        let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);

        let text =
            r#"I need to check the weather. {"tool": "weather.get", "args": {"city": "Tokyo"}}"#;
        match loop_impl.parse_llm_response(text) {
            ParsedResponse::ToolCall {
                reasoning,
                tool_call,
            } => {
                assert!(reasoning.contains("weather"));
                assert_eq!(tool_call.name, "weather.get");
                assert_eq!(tool_call.arguments["city"], "Tokyo");
            }
            _ => panic!("Expected ToolCall"),
        }
    }

    #[test]
    fn test_parse_reasoning() {
        let config = CognitiveConfig::default();
        let llm = Arc::new(LlmClient::from_env().unwrap());
        let broker = Arc::new(ActionBroker::new());
        let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);

        let text = "Let me think about this problem step by step...";
        match loop_impl.parse_llm_response(text) {
            ParsedResponse::Reasoning(text) => {
                assert!(text.contains("step by step"));
            }
            _ => panic!("Expected Reasoning"),
        }
    }

    #[test]
    fn test_extract_tool_call_variants() {
        let config = CognitiveConfig::default();
        let llm = Arc::new(LlmClient::from_env().unwrap());
        let broker = Arc::new(ActionBroker::new());
        let loop_impl = SimpleCognitiveLoop::new(config, llm, broker);

        // Pattern 1: tool/args
        let tc = loop_impl
            .extract_tool_call(r#"{"tool": "search", "args": {"q": "test"}}"#)
            .unwrap();
        assert_eq!(tc.name, "search");

        // Pattern 2: action/arguments
        let tc = loop_impl
            .extract_tool_call(r#"{"action": "calculate", "arguments": {"x": 1}}"#)
            .unwrap();
        assert_eq!(tc.name, "calculate");

        // Pattern 3: name/input
        let tc = loop_impl
            .extract_tool_call(r#"{"name": "translate", "input": {"text": "hello"}}"#)
            .unwrap();
        assert_eq!(tc.name, "translate");
    }
}
