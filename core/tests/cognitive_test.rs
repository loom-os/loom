//! Tests for the cognitive loop module.

use async_trait::async_trait;
use loom_core::agent::cognitive::{
    CognitiveAgent, CognitiveConfig, CognitiveLoop, ExecutionResult, MemoryItem, MemoryItemType,
    Observation, Perception, Plan, ThinkingStrategy, Thought, ThoughtStep, ToolCall, WorkingMemory,
};
use loom_core::agent::AgentBehavior;
use loom_core::proto::{AgentConfig, AgentState, Event};
use loom_core::Result;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// ============================================================================
// Test Helpers
// ============================================================================

fn make_test_event(id: &str, payload: &str) -> Event {
    Event {
        id: id.to_string(),
        r#type: "test.message".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
        source: "test".to_string(),
        metadata: Default::default(),
        payload: payload.as_bytes().to_vec(),
        confidence: 1.0,
        tags: vec![],
        priority: 50,
    }
}

fn make_test_state(agent_id: &str) -> AgentState {
    AgentState {
        agent_id: agent_id.to_string(),
        persistent_state: vec![],
        ephemeral_context: vec![],
        last_update_ms: 0,
        metadata: Default::default(),
    }
}

fn make_test_config(agent_id: &str) -> AgentConfig {
    AgentConfig {
        agent_id: agent_id.to_string(),
        agent_type: "cognitive_test".to_string(),
        subscribed_topics: vec![],
        capabilities: vec![],
        parameters: Default::default(),
    }
}

// ============================================================================
// Mock Cognitive Loop for Testing
// ============================================================================

/// A mock cognitive loop that tracks calls and returns configurable responses
struct MockCognitiveLoop {
    memory: WorkingMemory,
    perceive_count: Arc<AtomicUsize>,
    think_count: Arc<AtomicUsize>,
    act_count: Arc<AtomicUsize>,
    reflect_count: Arc<AtomicUsize>,
    should_reflect: bool,
    final_answer: String,
}

impl MockCognitiveLoop {
    fn new() -> Self {
        Self {
            memory: WorkingMemory::new(10),
            perceive_count: Arc::new(AtomicUsize::new(0)),
            think_count: Arc::new(AtomicUsize::new(0)),
            act_count: Arc::new(AtomicUsize::new(0)),
            reflect_count: Arc::new(AtomicUsize::new(0)),
            should_reflect: false,
            final_answer: "Test answer".to_string(),
        }
    }

    fn with_reflection(mut self) -> Self {
        self.should_reflect = true;
        self
    }

    fn with_answer(mut self, answer: &str) -> Self {
        self.final_answer = answer.to_string();
        self
    }
}

#[async_trait]
impl CognitiveLoop for MockCognitiveLoop {
    async fn perceive(&mut self, event: Event, _state: &AgentState) -> Result<Perception> {
        self.perceive_count.fetch_add(1, Ordering::SeqCst);
        Ok(Perception::from_event(event))
    }

    async fn think(&mut self, perception: &Perception) -> Result<Plan> {
        self.think_count.fetch_add(1, Ordering::SeqCst);
        Ok(Plan::final_answer(
            perception.goal.clone().unwrap_or_default(),
            &self.final_answer,
        ))
    }

    async fn act(&mut self, _plan: &Plan, state: &mut AgentState) -> Result<ExecutionResult> {
        self.act_count.fetch_add(1, Ordering::SeqCst);
        state.last_update_ms = chrono::Utc::now().timestamp_millis();
        Ok(ExecutionResult::with_response(&self.final_answer))
    }

    async fn reflect(
        &mut self,
        _perception: &Perception,
        _plan: &Plan,
        _result: &ExecutionResult,
    ) -> Result<Option<String>> {
        self.reflect_count.fetch_add(1, Ordering::SeqCst);
        if self.should_reflect {
            Ok(Some(
                "Reflection: Goal was achieved successfully.".to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    fn working_memory(&self) -> &WorkingMemory {
        &self.memory
    }

    fn working_memory_mut(&mut self) -> &mut WorkingMemory {
        &mut self.memory
    }
}

// ============================================================================
// CognitiveConfig Tests
// ============================================================================

#[test]
fn test_cognitive_config_default() {
    let config = CognitiveConfig::default();
    assert_eq!(config.max_iterations, 5);
    assert!(!config.enable_reflection);
    assert_eq!(config.thinking_strategy, ThinkingStrategy::SingleShot);
    assert_eq!(config.memory_window_size, 20);
    assert_eq!(config.tool_timeout_ms, 30_000);
    assert!(config.refine_after_tools);
}

#[test]
fn test_cognitive_config_react() {
    let config = CognitiveConfig::react();
    assert_eq!(config.thinking_strategy, ThinkingStrategy::ReAct);
    assert_eq!(config.max_iterations, 5);
}

#[test]
fn test_cognitive_config_single_shot() {
    let config = CognitiveConfig::single_shot();
    assert_eq!(config.thinking_strategy, ThinkingStrategy::SingleShot);
    assert_eq!(config.max_iterations, 1);
}

#[test]
fn test_cognitive_config_chain_of_thought() {
    let config = CognitiveConfig::chain_of_thought();
    assert_eq!(config.thinking_strategy, ThinkingStrategy::ChainOfThought);
    assert_eq!(config.max_iterations, 3);
}

#[test]
fn test_cognitive_config_builder() {
    let config = CognitiveConfig::react()
        .with_system_prompt("You are a helpful assistant")
        .with_max_iterations(10)
        .with_reflection()
        .with_memory_window(50);

    assert!(config.enable_reflection);
    assert_eq!(config.max_iterations, 10);
    assert_eq!(config.memory_window_size, 50);
    assert_eq!(
        config.system_prompt,
        Some("You are a helpful assistant".to_string())
    );
}

// ============================================================================
// WorkingMemory Tests
// ============================================================================

#[test]
fn test_working_memory_capacity() {
    let mut memory = WorkingMemory::new(3);

    memory.add_user_message("Message 1");
    memory.add_user_message("Message 2");
    memory.add_user_message("Message 3");
    assert_eq!(memory.len(), 3);

    // Adding 4th should evict oldest
    memory.add_user_message("Message 4");
    assert_eq!(memory.len(), 3);

    let context = memory.to_context_string();
    assert!(!context.contains("Message 1")); // Evicted
    assert!(context.contains("Message 4")); // Present
}

#[test]
fn test_working_memory_types() {
    let mut memory = WorkingMemory::new(10);

    memory.add_user_message("Hello");
    memory.add_agent_response("Hi there!");
    memory.add_observation("weather", "Sunny");

    let user_msgs = memory.items_by_type(MemoryItemType::UserMessage);
    assert_eq!(user_msgs.len(), 1);

    let agent_msgs = memory.items_by_type(MemoryItemType::AgentResponse);
    assert_eq!(agent_msgs.len(), 1);

    let observations = memory.items_by_type(MemoryItemType::Observation);
    assert_eq!(observations.len(), 1);
}

#[test]
fn test_working_memory_task_state() {
    let mut memory = WorkingMemory::new(10);

    memory.set_state("iteration", "3");
    memory.set_state("current_tool", "search");

    assert_eq!(memory.get_state("iteration"), Some(&"3".to_string()));
    assert_eq!(
        memory.get_state("current_tool"),
        Some(&"search".to_string())
    );
    assert_eq!(memory.get_state("nonexistent"), None);

    memory.remove_state("iteration");
    assert_eq!(memory.get_state("iteration"), None);
}

#[test]
fn test_working_memory_search() {
    let mut memory = WorkingMemory::new(10);

    memory.add_user_message("What's the weather in Tokyo?");
    memory.add_agent_response("The weather in Tokyo is sunny.");
    memory.add_user_message("And in Paris?");

    let tokyo_results = memory.search("tokyo");
    assert_eq!(tokyo_results.len(), 2);

    let paris_results = memory.search("paris");
    assert_eq!(paris_results.len(), 1);

    let weather_results = memory.search("weather");
    assert_eq!(weather_results.len(), 2);
}

#[test]
fn test_working_memory_session() {
    let memory = WorkingMemory::with_session(10, "session-123");
    assert_eq!(memory.session_id(), Some("session-123"));
}

#[test]
fn test_memory_item_from_event() {
    let event = make_test_event("evt1", "Hello world");
    let item = MemoryItem::from_event(&event);

    assert_eq!(item.id, "evt1");
    assert_eq!(item.item_type, MemoryItemType::Event);
    assert!(item.content.contains("Hello world"));
    assert_eq!(item.metadata.get("source"), Some(&"test".to_string()));
}

// ============================================================================
// Thought and Plan Tests
// ============================================================================

#[test]
fn test_thought_step_creation() {
    let step = ThoughtStep::reasoning(1, "I need to analyze the problem");
    assert_eq!(step.step, 1);
    assert!(!step.has_tool_call());
    assert!(step.is_complete());
}

#[test]
fn test_thought_step_with_tool() {
    let tool = ToolCall::new("search", json!({"query": "test"}));
    let step = ThoughtStep::with_tool(1, "I should search for this", tool);

    assert!(step.has_tool_call());
    assert!(!step.is_complete()); // No observation yet

    let step = step.with_observation(Observation::success("search", "Results found", 100));
    assert!(step.is_complete());
}

#[test]
fn test_tool_call_creation() {
    let tool = ToolCall::new("weather.get", json!({"city": "Tokyo"}));
    assert_eq!(tool.name, "weather.get");
    assert!(tool.id.is_none());

    let tool = ToolCall::with_id("call_123", "weather.get", json!({"city": "Tokyo"}));
    assert_eq!(tool.id, Some("call_123".to_string()));
}

#[test]
fn test_observation_creation() {
    let success = Observation::success("tool1", "Result data", 150);
    assert!(success.success);
    assert_eq!(success.output, "Result data");
    assert_eq!(success.latency_ms, 150);

    let error = Observation::error("tool2", "Connection failed", 50);
    assert!(!error.success);
    assert_eq!(error.error, Some("Connection failed".to_string()));
}

#[test]
fn test_thought_variants() {
    let final_answer = Thought::FinalAnswer("The answer is 42".to_string());
    assert!(final_answer.is_final());
    assert!(final_answer.tool_call().is_none());

    let tool_use = Thought::ToolUse {
        reasoning: "I need to calculate".to_string(),
        tool_call: ToolCall::new("calculator", json!({"expr": "21 * 2"})),
    };
    assert!(!tool_use.is_final());
    assert!(tool_use.tool_call().is_some());
    assert_eq!(tool_use.reasoning(), "I need to calculate");

    let reasoning = Thought::Reasoning("Let me think...".to_string());
    assert!(!reasoning.is_final());
    assert!(reasoning.tool_call().is_none());
}

#[test]
fn test_plan_building() {
    let mut plan = Plan::with_goal("Calculate 2 + 2");

    assert_eq!(plan.goal, "Calculate 2 + 2");
    assert!(!plan.complete);
    assert!(plan.final_answer.is_none());

    // Add a reasoning step
    plan.add_step(ThoughtStep::reasoning(1, "I need to add the numbers"));
    assert_eq!(plan.steps.len(), 1);

    // Add a tool step
    let tool = ToolCall::new("calculator", json!({"a": 2, "b": 2}));
    plan.add_step(ThoughtStep::with_tool(2, "Let me calculate", tool));
    assert!(plan.has_pending_tools());
    assert_eq!(plan.pending_tool_calls().len(), 1);

    // Add observation
    if let Some(step) = plan.last_step_mut() {
        step.observation = Some(Observation::success("calculator", "4", 10));
    }
    assert!(!plan.has_pending_tools());
    assert_eq!(plan.observations().len(), 1);

    // Complete the plan
    plan.complete_with_answer("The answer is 4");
    assert!(plan.complete);
    assert_eq!(plan.final_answer, Some("The answer is 4".to_string()));
}

#[test]
fn test_plan_summary() {
    let mut plan = Plan::with_goal("Test goal");
    plan.add_step(ThoughtStep::reasoning(1, "Thinking..."));
    plan.complete_with_answer("Done!");

    let summary = plan.to_summary();
    assert!(summary.contains("Goal: Test goal"));
    assert!(summary.contains("Thinking..."));
    assert!(summary.contains("Answer: Done!"));
}

// ============================================================================
// Perception Tests
// ============================================================================

#[test]
fn test_perception_from_event() {
    let event = make_test_event("evt1", "What's the weather?");
    let perception = Perception::from_event(event);

    assert_eq!(perception.goal, Some("What's the weather?".to_string()));
    assert_eq!(perception.priority, 50);
    assert!(perception.context.is_empty());
    assert!(perception.available_tools.is_empty());
}

#[test]
fn test_perception_goal_from_metadata() {
    let mut event = make_test_event("evt1", "");
    event
        .metadata
        .insert("goal".to_string(), "Custom goal".to_string());

    let perception = Perception::from_event(event);
    assert_eq!(perception.goal, Some("Custom goal".to_string()));
}

#[test]
fn test_perception_with_context_and_tools() {
    let event = make_test_event("evt1", "Query");
    let perception = Perception::from_event(event)
        .with_context(vec![
            "Context item 1".to_string(),
            "Context item 2".to_string(),
        ])
        .with_tools(vec!["tool1".to_string(), "tool2".to_string()]);

    assert_eq!(perception.context.len(), 2);
    assert_eq!(perception.available_tools.len(), 2);
}

// ============================================================================
// ExecutionResult Tests
// ============================================================================

#[test]
fn test_execution_result_empty() {
    let result = ExecutionResult::empty();
    assert!(result.actions.is_empty());
    assert!(!result.goal_achieved);
    assert!(result.response.is_none());
    assert!(result.error.is_none());
}

#[test]
fn test_execution_result_with_response() {
    let result = ExecutionResult::with_response("Hello!");
    assert!(result.goal_achieved);
    assert_eq!(result.response, Some("Hello!".to_string()));
}

#[test]
fn test_execution_result_error() {
    let result = ExecutionResult::error("Something went wrong");
    assert!(!result.goal_achieved);
    assert_eq!(result.error, Some("Something went wrong".to_string()));
}

// ============================================================================
// CognitiveAgent Adapter Tests
// ============================================================================

#[tokio::test]
async fn test_cognitive_agent_on_event() {
    let mock_loop = MockCognitiveLoop::new().with_answer("42");
    let perceive_count = Arc::clone(&mock_loop.perceive_count);
    let think_count = Arc::clone(&mock_loop.think_count);
    let act_count = Arc::clone(&mock_loop.act_count);

    let mut agent = CognitiveAgent::new(mock_loop);

    let event = make_test_event("evt1", "What is 6 * 7?");
    let mut state = make_test_state("agent1");

    let actions = agent.on_event(event, &mut state).await.unwrap();

    // Verify the cognitive cycle ran
    assert_eq!(perceive_count.load(Ordering::SeqCst), 1);
    assert_eq!(think_count.load(Ordering::SeqCst), 1);
    assert_eq!(act_count.load(Ordering::SeqCst), 1);

    // No actions returned by mock
    assert!(actions.is_empty());
}

#[tokio::test]
async fn test_cognitive_agent_with_reflection() {
    let mock_loop = MockCognitiveLoop::new().with_reflection();
    let reflect_count = Arc::clone(&mock_loop.reflect_count);

    let mut agent = CognitiveAgent::new(mock_loop);

    let event = make_test_event("evt1", "Test");
    let mut state = make_test_state("agent1");

    agent.on_event(event, &mut state).await.unwrap();

    // Reflection should have been called
    assert_eq!(reflect_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_cognitive_agent_init() {
    let mock_loop = MockCognitiveLoop::new();
    let mut agent = CognitiveAgent::new(mock_loop);

    let config = make_test_config("cognitive_agent_1");
    agent.on_init(&config).await.unwrap();

    // Session ID should be set from agent_id
    assert_eq!(
        agent.working_memory().session_id(),
        Some("cognitive_agent_1")
    );
}

#[tokio::test]
async fn test_cognitive_agent_init_with_session() {
    let mock_loop = MockCognitiveLoop::new();
    let mut agent = CognitiveAgent::new(mock_loop);

    let mut config = make_test_config("agent1");
    config
        .parameters
        .insert("session_id".to_string(), "custom-session".to_string());

    agent.on_init(&config).await.unwrap();

    assert_eq!(agent.working_memory().session_id(), Some("custom-session"));
}

#[tokio::test]
async fn test_cognitive_agent_shutdown() {
    let mock_loop = MockCognitiveLoop::new();
    let mut agent = CognitiveAgent::new(mock_loop);

    // Add some items to memory
    agent
        .inner_mut()
        .working_memory_mut()
        .add_user_message("Test");
    assert_eq!(agent.working_memory().len(), 1);

    // Shutdown should clear memory
    agent.on_shutdown().await.unwrap();
    assert!(agent.working_memory().is_empty());
}

#[tokio::test]
async fn test_cognitive_agent_multiple_events() {
    let mock_loop = MockCognitiveLoop::new();
    let perceive_count = Arc::clone(&mock_loop.perceive_count);

    let mut agent = CognitiveAgent::new(mock_loop);
    let mut state = make_test_state("agent1");

    // Process multiple events
    for i in 0..3 {
        let event = make_test_event(&format!("evt{}", i), &format!("Message {}", i));
        agent.on_event(event, &mut state).await.unwrap();
    }

    // Each event should trigger one perceive
    assert_eq!(perceive_count.load(Ordering::SeqCst), 3);

    // Memory should have accumulated event summaries
    assert!(agent.working_memory().len() > 0);
}

// ============================================================================
// Integration: CognitiveLoop run_cycle Tests
// ============================================================================

#[tokio::test]
async fn test_cognitive_loop_run_cycle() {
    let mut mock_loop = MockCognitiveLoop::new().with_answer("Success!");

    let event = make_test_event("evt1", "Do something");
    let mut state = make_test_state("agent1");

    let result = mock_loop.run_cycle(event, &mut state).await.unwrap();

    assert!(result.goal_achieved);
    assert_eq!(result.response, Some("Success!".to_string()));

    // Memory should be updated
    assert_eq!(mock_loop.working_memory().len(), 1);
}
