//! Loom Bridge - gRPC gateway for external agents
//!
//! Provides registration, bidirectional event streaming, tool forwarding, and heartbeat
//! for agents connecting via gRPC (Python, TypeScript, etc.)

use std::net::SocketAddr;
use std::sync::Arc;

pub mod memory_handler;
pub mod trading_memory;

use dashmap::DashMap;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::info;

use loom_core::{AgentDirectory, AgentInfo, AgentStatus, EventBus, ToolRegistry};
use loom_proto::{
    bridge_server::{Bridge, BridgeServer},
    client_event,
    memory_service_server::MemoryServiceServer,
    server_event, AgentRegisterRequest, AgentRegisterResponse, ClientEvent, Delivery,
    HeartbeatRequest, HeartbeatResponse, ServerEvent, ToolCall, ToolDescriptor, ToolResult,
    ToolStatus,
};

#[derive(thiserror::Error, Debug)]
pub enum BridgeError {
    #[error("registration failed: {0}")]
    Registration(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, BridgeError>;

#[derive(Clone)]
pub struct BridgeState {
    pub event_bus: Arc<EventBus>,
    pub tool_registry: Arc<ToolRegistry>,
    pub agent_directory: Arc<AgentDirectory>,
    // Optional dashboard broadcaster for event notifications
    pub dashboard_broadcaster: Option<loom_core::dashboard::EventBroadcaster>,
    // Optional flow tracker for event flow visualization
    pub flow_tracker: Option<Arc<loom_core::dashboard::FlowTracker>>,
    // agent_id -> subscribed topics
    pub subscriptions: Arc<DashMap<String, Vec<String>>>,
    // agent_id -> tools provided by the agent
    pub agent_tools: Arc<DashMap<String, Vec<ToolDescriptor>>>,
    // agent_id -> sender to push ServerEvent into gRPC stream task
    pub streams: Arc<DashMap<String, mpsc::Sender<ServerEvent>>>,
    // tool_call_id -> ToolResult received from agent (server-push correlation)
    pub tool_results: Arc<DashMap<String, ToolResult>>,
    // agent_id -> event bus subscription ids for cleanup
    pub subscription_ids: Arc<DashMap<String, Vec<String>>>,
    // agent_id -> task handles for forwarding loops (abort on disconnect)
    pub forwarding_tasks: Arc<DashMap<String, Vec<JoinHandle<()>>>>,
    // agent_id -> list of tool_result ids for cleanup
    pub tool_result_index: Arc<DashMap<String, Vec<String>>>,
}

impl BridgeState {
    pub fn new(
        event_bus: Arc<EventBus>,
        tool_registry: Arc<ToolRegistry>,
        agent_directory: Arc<AgentDirectory>,
    ) -> Self {
        Self {
            event_bus,
            tool_registry,
            agent_directory,
            dashboard_broadcaster: None,
            flow_tracker: None,
            subscriptions: Arc::new(DashMap::new()),
            agent_tools: Arc::new(DashMap::new()),
            streams: Arc::new(DashMap::new()),
            tool_results: Arc::new(DashMap::new()),
            subscription_ids: Arc::new(DashMap::new()),
            forwarding_tasks: Arc::new(DashMap::new()),
            tool_result_index: Arc::new(DashMap::new()),
        }
    }

    /// Set dashboard broadcaster for event notifications
    pub fn set_dashboard_broadcaster(
        &mut self,
        broadcaster: loom_core::dashboard::EventBroadcaster,
    ) {
        self.dashboard_broadcaster = Some(broadcaster);
    }

    /// Set flow tracker for event flow visualization
    pub fn set_flow_tracker(&mut self, flow_tracker: Arc<loom_core::dashboard::FlowTracker>) {
        self.flow_tracker = Some(flow_tracker);
    }
}

#[derive(Clone)]
pub struct BridgeService {
    state: BridgeState,
}

impl BridgeService {
    pub fn new(state: BridgeState) -> Self {
        Self { state }
    }

    /// Push a ToolCall to an agent's active stream; returns Ok(true) if delivered.
    pub async fn push_tool_call(&self, agent_id: &str, call: ToolCall) -> Result<bool> {
        if let Some(sender) = self.state.streams.get(agent_id) {
            let server_event = ServerEvent {
                msg: Some(server_event::Msg::ToolCall(call)),
            };
            match sender.send(server_event).await {
                Ok(_) => Ok(true),
                Err(e) => Err(BridgeError::Internal(format!(
                    "failed to send tool_call: {}",
                    e
                ))),
            }
        } else {
            Ok(false)
        }
    }

    /// Retrieve stored ToolResult by call id (set when client sends ToolResult on stream)
    pub fn get_tool_result(&self, call_id: &str) -> Option<ToolResult> {
        self.state.tool_results.get(call_id).map(|e| e.clone())
    }
}

#[tonic::async_trait]
impl Bridge for BridgeService {
    async fn register_agent(
        &self,
        request: Request<AgentRegisterRequest>,
    ) -> std::result::Result<Response<AgentRegisterResponse>, Status> {
        let req = request.into_inner();
        let agent_id = req.agent_id.clone();
        if agent_id.is_empty() {
            return Ok(Response::new(AgentRegisterResponse {
                success: false,
                error_message: "agent_id cannot be empty".into(),
            }));
        }
        self.state
            .subscriptions
            .insert(agent_id.clone(), req.subscribed_topics.clone());
        self.state
            .agent_tools
            .insert(agent_id.clone(), req.tools.clone());

        // Register agent in AgentDirectory for Dashboard visibility
        let tool_names: Vec<String> = req.tools.iter().map(|t| t.name.clone()).collect();
        let now = chrono::Utc::now().timestamp_millis();
        self.state.agent_directory.register_agent(AgentInfo {
            agent_id: agent_id.clone(),
            subscribed_topics: req.subscribed_topics.clone(),
            capabilities: tool_names,
            metadata: std::collections::HashMap::new(),
            last_heartbeat: Some(now),
            status: AgentStatus::Active,
        });

        // Broadcast AgentRegistered event to Dashboard
        if let Some(ref broadcaster) = self.state.dashboard_broadcaster {
            broadcaster.broadcast(loom_core::dashboard::DashboardEvent {
                timestamp: chrono::Utc::now().to_rfc3339(),
                event_type: loom_core::dashboard::DashboardEventType::AgentRegistered,
                event_id: format!("register_{}", agent_id),
                topic: "system.agent.register".to_string(),
                sender: Some(agent_id.clone()),
                thread_id: None,
                correlation_id: None,
                payload_preview: format!(
                    "Agent {} registered with {} topics, {} tools",
                    agent_id,
                    req.subscribed_topics.len(),
                    req.tools.len()
                ),
                trace_id: String::new(),
            });
        }

        info!(agent_id=%agent_id, topics=?req.subscribed_topics, tools=req.tools.len(), "Agent registered via Bridge");
        Ok(Response::new(AgentRegisterResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    type EventStreamStream = std::pin::Pin<
        Box<dyn futures_core::Stream<Item = std::result::Result<ServerEvent, Status>> + Send>,
    >;

    #[tracing::instrument(skip(self, request), fields(agent_id = tracing::field::Empty))]
    async fn event_stream(
        &self,
        request: Request<tonic::Streaming<ClientEvent>>,
    ) -> std::result::Result<Response<Self::EventStreamStream>, Status> {
        let mut inbound = request.into_inner();

        // Expect first message to be an Ack containing agent_id in message_id for simplicity (lightweight handshake)
        let agent_id: String = if let Some(Ok(first)) = inbound.message().await.transpose() {
            match first.msg {
                Some(client_event::Msg::Ack(a)) => a.message_id,
                _ => {
                    return Err(Status::invalid_argument(
                        "first stream message must be Ack carrying agent_id",
                    ));
                }
            }
        } else {
            return Err(Status::invalid_argument("no first message"));
        };

        // Record agent_id in span
        tracing::Span::current().record("agent_id", &agent_id);

        // Create outbound channel
        let (tx, rx) = mpsc::channel::<ServerEvent>(512);
        self.state.streams.insert(agent_id.clone(), tx.clone());
        let agent_id_for_inbound = agent_id.clone();

        // For each subscribed topic, subscribe and spawn a forwarding task, tracking ids and handles
        if let Some(topics) = self.state.subscriptions.get(&agent_id).map(|v| v.clone()) {
            let mut sub_ids: Vec<String> = Vec::new();
            let mut handles: Vec<JoinHandle<()>> = Vec::new();
            for topic in topics.iter() {
                let topic_clone = topic.clone();
                let event_bus_local = Arc::clone(&self.state.event_bus);
                let flow_tracker = self.state.flow_tracker.clone();
                let agent_id_for_flow = agent_id.clone();
                // subscribe first to capture subscription id and receiver
                if let Ok((sub_id, mut rx_bus)) = event_bus_local
                    .subscribe(
                        topic_clone.clone(),
                        vec![],
                        loom_proto::QoSLevel::QosBatched,
                    )
                    .await
                {
                    sub_ids.push(sub_id.clone());
                    let tx_clone = tx.clone();
                    let handle: JoinHandle<()> = tokio::spawn(async move {
                        while let Some(ev) = rx_bus.recv().await {
                            // Record flow: subscription -> agent
                            if let Some(ref tracker) = flow_tracker {
                                tracker
                                    .record_flow(&sub_id, &agent_id_for_flow, &topic_clone)
                                    .await;
                            }

                            // Create a span for forwarding this event to the agent stream
                            let fwd_span = tracing::info_span!(
                                "bridge.forward",
                                topic = %topic_clone,
                                agent_id = %agent_id_for_flow,
                                event_id = %ev.id,
                                trace_id = tracing::field::Empty,
                                span_id = tracing::field::Empty
                            );
                            let _fwd_guard = fwd_span.enter();

                            // Apply remote parent if present
                            let env = loom_core::Envelope::from_event(&ev);
                            if env.extract_trace_context() {
                                tracing::Span::current()
                                    .record("trace_id", &tracing::field::display(&env.trace_id));
                                tracing::Span::current()
                                    .record("span_id", &tracing::field::display(&env.span_id));
                            }

                            if tx_clone
                                .send(ServerEvent {
                                    msg: Some(server_event::Msg::Delivery(Delivery {
                                        topic: topic_clone.clone(),
                                        event: Some(ev),
                                    })),
                                })
                                .await
                                .is_err()
                            {
                                break; // stream dropped
                            }
                        }
                        // Ensure unsubscribe to release EventBus resources on normal end
                        let _ = event_bus_local.unsubscribe(&sub_id).await;
                    });
                    handles.push(handle);
                }
            }
            // record for cleanup
            self.state
                .subscription_ids
                .insert(agent_id.clone(), sub_ids);
            self.state
                .forwarding_tasks
                .insert(agent_id.clone(), handles);
        }

        // Spawn task handling inbound messages
        let event_bus = Arc::clone(&self.state.event_bus);
        let tx_in = tx.clone();
        let streams_map = self.state.streams.clone();
        let tool_results = self.state.tool_results.clone();
        let subscription_ids = self.state.subscription_ids.clone();
        let forwarding_tasks = self.state.forwarding_tasks.clone();
        let tool_result_index = self.state.tool_result_index.clone();
        let agent_directory = Arc::clone(&self.state.agent_directory);
        let dashboard_broadcaster = self.state.dashboard_broadcaster.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = inbound.message().await.transpose() {
                match msg.msg {
                    Some(client_event::Msg::Publish(p)) => {
                        if let (Some(ev), topic) = (p.event, p.topic) {
                            // Build span first, then enter and set remote parent on THIS span
                            let span = tracing::info_span!(
                                "bridge.publish",
                                agent_id = %agent_id_for_inbound,
                                topic = %topic,
                                event_id = %ev.id,
                                trace_id = tracing::field::Empty,
                                span_id = tracing::field::Empty
                            );
                            let _guard = span.enter();

                            // Extract trace context from event and set as parent of current span
                            let envelope = loom_core::Envelope::from_event(&ev);
                            if envelope.extract_trace_context() {
                                // Record extracted identifiers on the span for debugging/visibility
                                tracing::Span::current().record(
                                    "trace_id",
                                    &tracing::field::display(&envelope.trace_id),
                                );
                                tracing::Span::current()
                                    .record("span_id", &tracing::field::display(&envelope.span_id));
                            }

                            let _ = event_bus.publish(&topic, ev).await;
                        }
                    }
                    Some(client_event::Msg::Ping(_hb)) => {
                        // Update heartbeat in AgentDirectory
                        agent_directory.update_heartbeat(&agent_id_for_inbound);

                        let _ = tx_in
                            .send(ServerEvent {
                                msg: Some(server_event::Msg::Pong(HeartbeatResponse {
                                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                                    status: "ok".into(),
                                })),
                            })
                            .await;
                    }
                    Some(client_event::Msg::ToolResult(tr)) => {
                        info!(tool_id=%tr.id, "Received tool result from agent");
                        tool_results.insert(tr.id.clone(), tr.clone());
                        // index this result under agent for cleanup
                        tool_result_index
                            .entry(agent_id_for_inbound.clone())
                            .or_insert_with(Vec::new)
                            .push(tr.id);
                    }
                    Some(client_event::Msg::Ack(_)) => { /* ignore */ }
                    None => {}
                }
            }
            info!(agent_id=%agent_id_for_inbound, "EventStream inbound ended");

            // Update agent status to Disconnected
            agent_directory.update_status(&agent_id_for_inbound, AgentStatus::Disconnected);

            // Cleanup stream sender on disconnect
            streams_map.remove(&agent_id_for_inbound);

            // Unregister agent from directory
            agent_directory.unregister_agent(&agent_id_for_inbound);

            // Broadcast AgentUnregistered event to Dashboard
            if let Some(ref broadcaster) = dashboard_broadcaster {
                broadcaster.broadcast(loom_core::dashboard::DashboardEvent {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    event_type: loom_core::dashboard::DashboardEventType::AgentUnregistered,
                    event_id: format!("unregister_{}", agent_id_for_inbound),
                    topic: "system.agent.unregister".to_string(),
                    sender: Some(agent_id_for_inbound.clone()),
                    thread_id: None,
                    correlation_id: None,
                    payload_preview: format!("Agent {} disconnected", agent_id_for_inbound),
                    trace_id: String::new(),
                });
            }

            // Unsubscribe all topic subscriptions for this agent
            if let Some(ids) = subscription_ids.remove(&agent_id_for_inbound) {
                for sid in ids.1.iter() {
                    let _ = event_bus.unsubscribe(sid).await;
                }
            }
            // Abort forwarding tasks to stop background loops promptly
            if let Some(handles) = forwarding_tasks.remove(&agent_id_for_inbound) {
                for h in handles.1.into_iter() {
                    h.abort();
                }
            }
            // Drop any stored tool results indexed for this agent to avoid leaks
            if let Some(res_ids) = tool_result_index.remove(&agent_id_for_inbound) {
                for rid in res_ids.1.into_iter() {
                    tool_results.remove(&rid);
                }
            }
        });

        let id_for_log = agent_id;
        let outbound = ReceiverStream::new(rx).map(|ev| Ok(ev));
        info!(agent_id=%id_for_log, "EventStream outbound established");
        Ok(Response::new(Box::pin(outbound) as Self::EventStreamStream))
    }

    #[tracing::instrument(skip(self, request), fields(
        tool_name = tracing::field::Empty,
        call_id = tracing::field::Empty,
        trace_id = tracing::field::Empty
    ))]
    async fn forward_tool_call(
        &self,
        request: Request<ToolCall>,
    ) -> std::result::Result<Response<ToolResult>, Status> {
        let call = request.into_inner();

        // Record call details in span
        tracing::Span::current().record("tool_name", &call.name.as_str());
        tracing::Span::current().record("call_id", &call.id.as_str());

        // Extract trace context from tool call
        let envelope = loom_core::Envelope::from_metadata(&call.headers, &call.id);
        if envelope.extract_trace_context() {
            tracing::Span::current()
                .record("trace_id", &tracing::field::display(&envelope.trace_id));
        }

        // Parse arguments from JSON string
        let arguments: serde_json::Value = match serde_json::from_str(&call.arguments) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::new(ToolResult {
                    id: call.id,
                    status: ToolStatus::ToolInvalidArguments as i32,
                    output: String::new(),
                    error: Some(loom_proto::ToolError {
                        code: "INVALID_ARGUMENTS".into(),
                        message: e.to_string(),
                        details: Default::default(),
                    }),
                }));
            }
        };

        // Call the tool via ToolRegistry
        let registry = Arc::clone(&self.state.tool_registry);

        // Execute tool - tracing spans are already set via #[tracing::instrument] on the registry.call method
        let tool_result = registry.call(&call.name, arguments).await;

        match tool_result {
            Ok(output) => {
                let output_str = serde_json::to_string(&output).unwrap_or_default();
                Ok(Response::new(ToolResult {
                    id: call.id,
                    status: ToolStatus::ToolOk as i32,
                    output: output_str,
                    error: None,
                }))
            }
            Err(e) => {
                let (status, code) = match &e {
                    loom_core::ToolError::NotFound(_) => (ToolStatus::ToolNotFound, "NOT_FOUND"),
                    loom_core::ToolError::InvalidArguments(_) => {
                        (ToolStatus::ToolInvalidArguments, "INVALID_ARGUMENTS")
                    }
                    loom_core::ToolError::Timeout => (ToolStatus::ToolTimeout, "TIMEOUT"),
                    _ => (ToolStatus::ToolError, "EXECUTION_ERROR"),
                };
                Ok(Response::new(ToolResult {
                    id: call.id,
                    status: status as i32,
                    output: String::new(),
                    error: Some(loom_proto::ToolError {
                        code: code.into(),
                        message: e.to_string(),
                        details: Default::default(),
                    }),
                }))
            }
        }
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> std::result::Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();

        Ok(Response::new(HeartbeatResponse {
            timestamp_ms: req.timestamp_ms,
            status: "ok".into(),
        }))
    }
}

pub async fn start_server(
    addr: SocketAddr,
    event_bus: Arc<EventBus>,
    tool_registry: Arc<ToolRegistry>,
    agent_directory: Arc<AgentDirectory>,
) -> Result<()> {
    info!(addr = %addr, "Starting Loom Bridge gRPC server");

    let svc = BridgeService::new(BridgeState::new(event_bus, tool_registry, agent_directory));

    // Create memory store and handler
    let memory_store = trading_memory::InMemoryMemory::new();
    let memory_handler = memory_handler::MemoryHandler::new(memory_store);

    tonic::transport::Server::builder()
        .add_service(BridgeServer::new(svc))
        .add_service(MemoryServiceServer::new(memory_handler))
        .serve(addr)
        .await
        .map_err(|e| BridgeError::Internal(e.to_string()))
}

/// Start server with dashboard integration
pub async fn start_server_with_dashboard(
    addr: SocketAddr,
    event_bus: Arc<EventBus>,
    tool_registry: Arc<ToolRegistry>,
    agent_directory: Arc<AgentDirectory>,
    dashboard_broadcaster: Option<loom_core::dashboard::EventBroadcaster>,
    flow_tracker: Option<Arc<loom_core::dashboard::FlowTracker>>,
) -> Result<()> {
    info!(addr = %addr, "Starting Loom Bridge gRPC server with Dashboard integration");

    let mut state = BridgeState::new(event_bus, tool_registry, agent_directory);

    if let Some(broadcaster) = dashboard_broadcaster {
        state.set_dashboard_broadcaster(broadcaster);
    }

    if let Some(tracker) = flow_tracker {
        state.set_flow_tracker(tracker);
    }

    let svc = BridgeService::new(state);

    // Create memory store and handler
    let memory_store = trading_memory::InMemoryMemory::new();
    let memory_handler = memory_handler::MemoryHandler::new(memory_store);

    tonic::transport::Server::builder()
        .add_service(BridgeServer::new(svc))
        .add_service(MemoryServiceServer::new(memory_handler))
        .serve(addr)
        .await
        .map_err(|e| BridgeError::Internal(e.to_string()))
}
