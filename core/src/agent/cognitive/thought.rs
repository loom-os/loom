//! Thought and planning structures for the cognitive loop.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single thought step in the reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStep {
    /// The reasoning or chain-of-thought text
    pub reasoning: String,

    /// Optional tool call to execute
    pub tool_call: Option<ToolCall>,

    /// Observation from tool execution (filled after act)
    pub observation: Option<Observation>,

    /// Step index in the reasoning chain
    pub step: usize,
}

impl ThoughtStep {
    /// Create a new thought step with just reasoning
    pub fn reasoning(step: usize, text: impl Into<String>) -> Self {
        Self {
            reasoning: text.into(),
            tool_call: None,
            observation: None,
            step,
        }
    }

    /// Create a thought step with a tool call
    pub fn with_tool(step: usize, reasoning: impl Into<String>, tool_call: ToolCall) -> Self {
        Self {
            reasoning: reasoning.into(),
            tool_call: Some(tool_call),
            observation: None,
            step,
        }
    }

    /// Add an observation to this thought step
    pub fn with_observation(mut self, observation: Observation) -> Self {
        self.observation = Some(observation);
        self
    }

    /// Check if this step has a tool call
    pub fn has_tool_call(&self) -> bool {
        self.tool_call.is_some()
    }

    /// Check if this step is complete (has observation if tool was called)
    pub fn is_complete(&self) -> bool {
        match &self.tool_call {
            Some(_) => self.observation.is_some(),
            None => true,
        }
    }
}

/// A tool call to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this call
    pub id: Option<String>,

    /// Name of the tool/capability to invoke
    pub name: String,

    /// Arguments to pass to the tool
    pub arguments: Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(name: impl Into<String>, arguments: Value) -> Self {
        Self {
            id: None,
            name: name.into(),
            arguments,
        }
    }

    /// Create a tool call with an ID
    pub fn with_id(id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self {
        Self {
            id: Some(id.into()),
            name: name.into(),
            arguments,
        }
    }
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// The tool that was called
    pub tool_name: String,

    /// Whether the call succeeded
    pub success: bool,

    /// The output from the tool (as text)
    pub output: String,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub latency_ms: u64,
}

impl Observation {
    /// Create a successful observation
    pub fn success(
        tool_name: impl Into<String>,
        output: impl Into<String>,
        latency_ms: u64,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            success: true,
            output: output.into(),
            error: None,
            latency_ms,
        }
    }

    /// Create a failed observation
    pub fn error(tool_name: impl Into<String>, error: impl Into<String>, latency_ms: u64) -> Self {
        Self {
            tool_name: tool_name.into(),
            success: false,
            output: String::new(),
            error: Some(error.into()),
            latency_ms,
        }
    }
}

/// A thought represents either a final answer or a tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Thought {
    /// A final answer (no more tool calls needed)
    FinalAnswer(String),

    /// A tool call is needed
    ToolUse {
        reasoning: String,
        tool_call: ToolCall,
    },

    /// Just reasoning, no action needed
    Reasoning(String),
}

impl Thought {
    /// Check if this is a final answer
    pub fn is_final(&self) -> bool {
        matches!(self, Thought::FinalAnswer(_))
    }

    /// Get the tool call if this is a tool use thought
    pub fn tool_call(&self) -> Option<&ToolCall> {
        match self {
            Thought::ToolUse { tool_call, .. } => Some(tool_call),
            _ => None,
        }
    }

    /// Get the reasoning text
    pub fn reasoning(&self) -> &str {
        match self {
            Thought::FinalAnswer(text) => text,
            Thought::ToolUse { reasoning, .. } => reasoning,
            Thought::Reasoning(text) => text,
        }
    }
}

/// A plan consisting of multiple thought steps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
    /// The goal this plan is trying to achieve
    pub goal: String,

    /// Ordered list of thought steps
    pub steps: Vec<ThoughtStep>,

    /// Final answer (if reached)
    pub final_answer: Option<String>,

    /// Whether the plan is complete
    pub complete: bool,
}

impl Plan {
    /// Create an empty plan
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a plan with a goal
    pub fn with_goal(goal: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            ..Default::default()
        }
    }

    /// Create a plan with a final answer (single-shot)
    pub fn final_answer(goal: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            steps: vec![],
            final_answer: Some(answer.into()),
            complete: true,
        }
    }

    /// Add a thought step to the plan
    pub fn add_step(&mut self, step: ThoughtStep) {
        self.steps.push(step);
    }

    /// Mark the plan as complete with a final answer
    pub fn complete_with_answer(&mut self, answer: impl Into<String>) {
        self.final_answer = Some(answer.into());
        self.complete = true;
    }

    /// Get the last step (if any)
    pub fn last_step(&self) -> Option<&ThoughtStep> {
        self.steps.last()
    }

    /// Get mutable reference to the last step
    pub fn last_step_mut(&mut self) -> Option<&mut ThoughtStep> {
        self.steps.last_mut()
    }

    /// Get all pending tool calls (steps without observations)
    pub fn pending_tool_calls(&self) -> Vec<&ToolCall> {
        self.steps
            .iter()
            .filter_map(|step| {
                if step.tool_call.is_some() && step.observation.is_none() {
                    step.tool_call.as_ref()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if there are pending tool calls
    pub fn has_pending_tools(&self) -> bool {
        !self.pending_tool_calls().is_empty()
    }

    /// Get all observations from completed steps
    pub fn observations(&self) -> Vec<&Observation> {
        self.steps
            .iter()
            .filter_map(|step| step.observation.as_ref())
            .collect()
    }

    /// Format the plan as a text summary
    pub fn to_summary(&self) -> String {
        let mut lines = vec![format!("Goal: {}", self.goal)];

        for (i, step) in self.steps.iter().enumerate() {
            lines.push(format!("Step {}: {}", i + 1, step.reasoning));
            if let Some(ref tc) = step.tool_call {
                lines.push(format!("  Tool: {} args={}", tc.name, tc.arguments));
            }
            if let Some(ref obs) = step.observation {
                if obs.success {
                    lines.push(format!("  Result: {}", truncate(&obs.output, 100)));
                } else {
                    lines.push(format!(
                        "  Error: {}",
                        obs.error.as_deref().unwrap_or("unknown")
                    ));
                }
            }
        }

        if let Some(ref answer) = self.final_answer {
            lines.push(format!("Answer: {}", answer));
        }

        lines.join("\n")
    }
}

/// Truncate a string to max_len characters, adding ellipsis if needed
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len.saturating_sub(1);
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}
