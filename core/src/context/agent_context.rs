//! AgentContext - High-level API for agents to manage context
//!
//! Provides a clean interface for agents to record messages, tool calls, events,
//! and retrieve relevant context without dealing with pipeline complexity.

use crate::context::retrieval::RetrievalTrigger;
use crate::context::{
    ContextContent, ContextItem, ContextItemType, ContextMetadata, ContextPipeline, InMemoryStore,
    MemoryStore, MessageRole, RecencyRetrieval, TemporalRanker, TiktokenCounter, WindowConfig,
};
use crate::proto::ActionResult;
use crate::Result;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, instrument};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// High-level context management for agents
///
/// Wraps ContextPipeline with a simple API for recording and retrieving context.
pub struct AgentContext {
    session_id: String,
    agent_id: String,
    store: Arc<dyn MemoryStore>,
    pipeline: Arc<ContextPipeline>,
}

impl AgentContext {
    /// Create a new AgentContext with custom store and pipeline
    pub fn new(
        session_id: impl Into<String>,
        agent_id: impl Into<String>,
        store: Arc<dyn MemoryStore>,
        pipeline: Arc<ContextPipeline>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            agent_id: agent_id.into(),
            store,
            pipeline,
        }
    }

    /// Create AgentContext with default configuration
    ///
    /// Uses InMemoryStore, RecencyRetrieval(100), TemporalRanker, GPT-4 tokenizer
    pub fn with_defaults(session_id: impl Into<String>, agent_id: impl Into<String>) -> Self {
        let store: Arc<dyn MemoryStore> = InMemoryStore::new();
        let retrieval: Arc<dyn crate::context::retrieval::RetrievalStrategy> =
            RecencyRetrieval::new(100);
        let ranker: Arc<dyn crate::context::ranking::ContextRanker> =
            TemporalRanker::newest_first();
        let counter = Arc::new(TiktokenCounter::gpt4());
        let window_config = WindowConfig::default();
        let window_manager = crate::context::window::WindowManager::new(counter, window_config);
        let pipeline_config = crate::context::pipeline::PipelineConfig::default();

        let pipeline = Arc::new(ContextPipeline::new(
            store.clone(),
            retrieval,
            ranker,
            window_manager,
            pipeline_config,
        ));

        Self::new(session_id, agent_id, store, pipeline)
    }

    /// Record a message in the context
    #[instrument(skip(self, content), fields(session = %self.session_id, role = ?role))]
    pub async fn record_message(
        &self,
        role: MessageRole,
        content: impl Into<String>,
    ) -> Result<String> {
        let content_str = content.into();
        debug!(
            "Recording message: role={:?}, len={}",
            role,
            content_str.len()
        );

        let item = ContextItem {
            id: Self::generate_id(),
            item_type: ContextItemType::Message { role },
            content: ContextContent::from_string(content_str),
            metadata: ContextMetadata::new(self.session_id.clone(), self.agent_id.clone())
                .with_current_trace(),
        };

        let id = item.id.clone();
        self.store.store(item).await?;
        Ok(id)
    }

    /// Record a tool call
    #[instrument(skip(self, arguments), fields(session = %self.session_id, tool = %tool_name))]
    pub async fn record_tool_call(&self, tool_name: &str, arguments: Value) -> Result<String> {
        debug!("Recording tool call: {}", tool_name);

        let item = ContextItem {
            id: Self::generate_id(),
            item_type: ContextItemType::ToolCall {
                tool_name: tool_name.to_string(),
            },
            content: ContextContent::from_value(arguments),
            metadata: ContextMetadata::new(self.session_id.clone(), self.agent_id.clone())
                .with_current_trace(),
        };

        let id = item.id.clone();
        self.store.store(item).await?;
        Ok(id)
    }

    /// Record a tool result
    #[instrument(skip(self, output), fields(session = %self.session_id, tool = %tool_name, success = success))]
    pub async fn record_tool_result(
        &self,
        tool_name: &str,
        success: bool,
        output: Value,
        related_call_id: Option<String>,
    ) -> Result<String> {
        debug!("Recording tool result: {}", tool_name);

        let mut metadata = ContextMetadata::new(self.session_id.clone(), self.agent_id.clone())
            .with_current_trace();

        if let Some(call_id) = related_call_id {
            metadata = metadata.with_related_item(call_id);
        }

        let item = ContextItem {
            id: Self::generate_id(),
            item_type: ContextItemType::ToolResult {
                tool_name: tool_name.to_string(),
                success,
            },
            content: ContextContent::from_value(output),
            metadata,
        };

        let id = item.id.clone();
        self.store.store(item).await?;
        Ok(id)
    }

    /// Convenience method to record tool call and result from ActionResult
    ///
    /// Note: ActionResult doesn't include the tool name, so it must be provided
    #[instrument(skip(self, result), fields(session = %self.session_id, action_id = %result.id))]
    pub async fn record_action_result(&self, result: &ActionResult, tool_name: &str) -> Result<()> {
        debug!("Recording action result: {} ({})", tool_name, result.id);

        // Record the call with empty arguments (actual args not in ActionResult)
        let call_id = self
            .record_tool_call(tool_name, Value::Object(serde_json::Map::new()))
            .await?;

        // Then record the result
        let status = result.status();
        let success = status == crate::proto::ActionStatus::ActionOk;
        let error_msg = result
            .error
            .as_ref()
            .map(|e| format!("{}: {}", e.code, e.message));
        let mut output_map = serde_json::Map::new();
        output_map.insert(
            "output".to_string(),
            serde_json::to_value(String::from_utf8_lossy(&result.output)).unwrap(),
        );
        output_map.insert(
            "status".to_string(),
            serde_json::to_value(format!("{:?}", status)).unwrap(),
        );
        output_map.insert(
            "error".to_string(),
            serde_json::to_value(error_msg).unwrap(),
        );
        let output = Value::Object(output_map);

        self.record_tool_result(tool_name, success, output, Some(call_id))
            .await?;
        Ok(())
    }

    /// Record an event from the proto Event type
    #[instrument(skip(self, event), fields(session = %self.session_id, event_type = %event.r#type))]
    pub async fn record_event(&self, event: &crate::proto::Event) -> Result<String> {
        debug!("Recording event: {}", event.r#type);

        // Manually construct event JSON since Event doesn't implement Serialize
        let mut event_map = serde_json::Map::new();
        event_map.insert("id".to_string(), serde_json::to_value(&event.id).unwrap());
        event_map.insert(
            "type".to_string(),
            serde_json::to_value(&event.r#type).unwrap(),
        );
        event_map.insert(
            "timestamp_ms".to_string(),
            serde_json::to_value(event.timestamp_ms).unwrap(),
        );
        event_map.insert(
            "source".to_string(),
            serde_json::to_value(&event.source).unwrap(),
        );
        event_map.insert(
            "confidence".to_string(),
            serde_json::to_value(event.confidence).unwrap(),
        );
        event_map.insert(
            "priority".to_string(),
            serde_json::to_value(event.priority).unwrap(),
        );
        let event_json = Value::Object(event_map);

        let item = ContextItem {
            id: Self::generate_id(),
            item_type: ContextItemType::Event {
                event_type: event.r#type.clone(),
            },
            content: ContextContent::from_value(event_json),
            metadata: ContextMetadata::new(self.session_id.clone(), self.agent_id.clone())
                .with_current_trace(),
        };

        let id = item.id.clone();
        self.store.store(item).await?;
        Ok(id)
    }

    /// Record a generic observation
    #[instrument(skip(self, content), fields(session = %self.session_id, source = %source))]
    pub async fn record_observation(
        &self,
        source: &str,
        content: impl Into<String>,
    ) -> Result<String> {
        debug!("Recording observation from {}", source);

        let item = ContextItem {
            id: Self::generate_id(),
            item_type: ContextItemType::Observation {
                source: source.to_string(),
            },
            content: ContextContent::from_string(content.into()),
            metadata: ContextMetadata::new(self.session_id.clone(), self.agent_id.clone())
                .with_current_trace(),
        };

        let id = item.id.clone();
        self.store.store(item).await?;
        Ok(id)
    }

    /// Retrieve relevant context for a given goal
    #[instrument(skip(self), fields(session = %self.session_id))]
    pub async fn get_context(&self, goal: Option<&str>) -> Result<Vec<ContextItem>> {
        let mut trigger = RetrievalTrigger::new(self.session_id.clone(), self.agent_id.clone())
            .with_max_items(100);

        if let Some(g) = goal {
            trigger = trigger.with_goal(g.to_string());
        }

        let result = self.pipeline.execute(trigger).await?;
        Ok(result.items)
    }

    /// Generate unique ID for context items
    pub fn generate_id() -> String {
        let counter = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("ctx_{}_{}", nanos, counter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::MessageRole;

    #[tokio::test]
    async fn test_record_message() {
        let ctx = AgentContext::with_defaults("test_session", "agent_1");
        let id = ctx
            .record_message(MessageRole::User, "Hello")
            .await
            .unwrap();
        assert!(id.starts_with("ctx_"));
    }

    #[tokio::test]
    async fn test_record_tool_call() {
        let ctx = AgentContext::with_defaults("test_session", "agent_1");
        let mut args_map = serde_json::Map::new();
        args_map.insert("query".to_string(), serde_json::to_value("test").unwrap());
        let args = Value::Object(args_map);
        let id = ctx.record_tool_call("search", args).await.unwrap();
        assert!(id.starts_with("ctx_"));
    }

    #[tokio::test]
    async fn test_record_tool_result() {
        let ctx = AgentContext::with_defaults("test_session", "agent_1");
        let mut output_map = serde_json::Map::new();
        output_map.insert("result".to_string(), serde_json::to_value("found").unwrap());
        let output = Value::Object(output_map);
        let id = ctx
            .record_tool_result("search", true, output, None)
            .await
            .unwrap();
        assert!(id.starts_with("ctx_"));
    }

    #[tokio::test]
    async fn test_record_tool_call_and_result() {
        let ctx = AgentContext::with_defaults("test_session", "agent_1");

        let mut call_map = serde_json::Map::new();
        call_map.insert("q".to_string(), serde_json::to_value("rust").unwrap());
        let call_args = Value::Object(call_map);
        let call_id = ctx.record_tool_call("search", call_args).await.unwrap();

        let mut result_map = serde_json::Map::new();
        result_map.insert("hits".to_string(), serde_json::to_value(10).unwrap());
        let result_output = Value::Object(result_map);
        let result_id = ctx
            .record_tool_result("search", true, result_output, Some(call_id.clone()))
            .await
            .unwrap();

        assert_ne!(call_id, result_id);
    }

    #[tokio::test]
    async fn test_record_event() {
        let ctx = AgentContext::with_defaults("test_session", "agent_1");
        let event = crate::proto::Event {
            id: "evt_123".to_string(),
            r#type: "test.event".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis() as i64,
            source: "test".to_string(),
            metadata: std::collections::HashMap::new(),
            payload: vec![],
            confidence: 1.0,
            tags: vec![],
            priority: 0,
        };
        let id = ctx.record_event(&event).await.unwrap();
        assert!(id.starts_with("ctx_"));
    }

    #[tokio::test]
    async fn test_record_observation() {
        let ctx = AgentContext::with_defaults("test_session", "agent_1");
        let id = ctx
            .record_observation("sensor", "Temperature: 72F")
            .await
            .unwrap();
        assert!(id.starts_with("ctx_"));
    }

    #[tokio::test]
    async fn test_get_context() {
        let ctx = AgentContext::with_defaults("test_session", "agent_1");
        ctx.record_message(MessageRole::User, "What is Rust?")
            .await
            .unwrap();
        ctx.record_message(MessageRole::Assistant, "Rust is a programming language")
            .await
            .unwrap();

        let items = ctx.get_context(Some("Rust programming")).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_id_generation() {
        let id1 = AgentContext::generate_id();
        let id2 = AgentContext::generate_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("ctx_"));
        assert!(id2.starts_with("ctx_"));
    }
}
