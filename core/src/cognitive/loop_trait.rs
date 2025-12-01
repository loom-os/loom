//! Core cognitive loop trait definition.

use async_trait::async_trait;

use crate::proto::{Action, AgentState, Event};
use crate::Result;

use super::memory_buffer::MemoryBuffer;
use super::thought::Plan;

/// Perception result from the perceive phase
#[derive(Debug, Clone)]
pub struct Perception {
    /// The original event that triggered this cognitive cycle
    pub event: Event,

    /// Extracted goal or intent from the event
    pub goal: Option<String>,

    /// Relevant context retrieved from memory
    pub context: Vec<String>,

    /// Available tools/capabilities for this cycle
    pub available_tools: Vec<String>,

    /// Priority level (inherited from event)
    pub priority: i32,
}

impl Perception {
    /// Create a new perception from an event
    pub fn from_event(event: Event) -> Self {
        let goal = Self::extract_goal(&event);
        Self {
            priority: event.priority,
            event,
            goal,
            context: vec![],
            available_tools: vec![],
        }
    }

    /// Extract goal from event payload or metadata
    fn extract_goal(event: &Event) -> Option<String> {
        // Try to get goal from metadata first
        if let Some(goal) = event.metadata.get("goal") {
            return Some(goal.clone());
        }

        // Try to get instruction from metadata
        if let Some(instruction) = event.metadata.get("instruction") {
            return Some(instruction.clone());
        }

        // Try to parse payload as UTF-8 text
        if !event.payload.is_empty() {
            if let Ok(text) = std::str::from_utf8(&event.payload) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }

        None
    }

    /// Add context items
    pub fn with_context(mut self, context: Vec<String>) -> Self {
        self.context = context;
        self
    }

    /// Add available tools
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.available_tools = tools;
        self
    }
}

/// Execution result from the act phase
#[derive(Debug, Clone, Default)]
pub struct ExecutionResult {
    /// Actions to be executed by the agent runtime
    pub actions: Vec<Action>,

    /// Text response (if any)
    pub response: Option<String>,

    /// Whether the goal was achieved
    pub goal_achieved: bool,

    /// Error message (if any)
    pub error: Option<String>,
}

impl ExecutionResult {
    /// Create an empty result
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a result with actions
    pub fn with_actions(actions: Vec<Action>) -> Self {
        Self {
            actions,
            goal_achieved: true,
            ..Default::default()
        }
    }

    /// Create a result with a text response
    pub fn with_response(response: impl Into<String>) -> Self {
        Self {
            response: Some(response.into()),
            goal_achieved: true,
            ..Default::default()
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            error: Some(message.into()),
            goal_achieved: false,
            ..Default::default()
        }
    }

    /// Convert to actions list for AgentBehavior
    pub fn into_actions(self) -> Vec<Action> {
        self.actions
    }
}

/// High-level cognitive loop: perceive events, think, then act.
///
/// This trait defines the core cognitive architecture for intelligent agents.
/// Implementations can choose different strategies for each phase while
/// maintaining a consistent interface.
///
/// # Example
///
/// ```rust,ignore
/// use loom_core::agent::cognitive::{CognitiveLoop, Perception, Plan, ExecutionResult};
///
/// struct MyCognitiveLoop { /* ... */ }
///
/// #[async_trait]
/// impl CognitiveLoop for MyCognitiveLoop {
///     async fn perceive(&mut self, event: Event, state: &AgentState) -> Result<Perception> {
///         // Build perception from event and memory
///         Ok(Perception::from_event(event))
///     }
///
///     async fn think(&mut self, perception: &Perception) -> Result<Plan> {
///         // Use LLM to generate a plan
///         Ok(Plan::empty())
///     }
///
///     async fn act(&mut self, plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult> {
///         // Execute the plan
///         Ok(ExecutionResult::empty())
///     }
/// }
/// ```
#[async_trait]
pub trait CognitiveLoop: Send + Sync {
    /// Perceive phase: process incoming event and build context.
    ///
    /// This phase is responsible for:
    /// - Parsing the incoming event
    /// - Retrieving relevant context from memory
    /// - Identifying the goal or intent
    /// - Discovering available tools
    async fn perceive(&mut self, event: Event, state: &AgentState) -> Result<Perception>;

    /// Think phase: reason about the perception and create a plan.
    ///
    /// This phase is responsible for:
    /// - Generating reasoning steps (if using ReAct/CoT)
    /// - Deciding which tools to use
    /// - Creating a structured plan
    async fn think(&mut self, perception: &Perception) -> Result<Plan>;

    /// Act phase: execute the plan and produce results.
    ///
    /// This phase is responsible for:
    /// - Invoking tools via ActionBroker
    /// - Generating final response
    /// - Updating agent state
    async fn act(&mut self, plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult>;

    /// Optional: Reflect on the execution and learn.
    ///
    /// This phase can:
    /// - Evaluate if the goal was achieved
    /// - Generate self-critique
    /// - Suggest corrections for future attempts
    async fn reflect(
        &mut self,
        _perception: &Perception,
        _plan: &Plan,
        _result: &ExecutionResult,
    ) -> Result<Option<String>> {
        // Default: no reflection
        Ok(None)
    }

    /// Access the memory buffer
    fn memory_buffer(&self) -> &MemoryBuffer;

    /// Mutable access to memory buffer
    fn memory_buffer_mut(&mut self) -> &mut MemoryBuffer;

    /// Run the complete cognitive cycle
    async fn run_cycle(&mut self, event: Event, state: &mut AgentState) -> Result<ExecutionResult> {
        // 1. Perceive
        let perception = self.perceive(event.clone(), state).await?;

        // 2. Think
        let plan = self.think(&perception).await?;

        // 3. Act
        let result = self.act(&plan, state).await?;

        // 4. Reflect (optional)
        if let Ok(Some(reflection)) = self.reflect(&perception, &plan, &result).await {
            tracing::debug!(
                target = "cognitive",
                reflection = %reflection,
                "Reflection complete"
            );
        }

        // 5. Update memory buffer
        self.memory_buffer_mut().add_event_summary(&event);

        Ok(result)
    }
}
