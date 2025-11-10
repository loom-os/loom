use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::info;

use loom_core::{ActionBroker, EventBus};
use loom_proto::{
    bridge_server::{Bridge, BridgeServer},
    client_event, server_event, ActionCall, ActionResult, AgentRegisterRequest,
    AgentRegisterResponse, CapabilityDescriptor, ClientEvent, Delivery, HeartbeatRequest,
    HeartbeatResponse, ServerEvent,
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
    pub action_broker: Arc<ActionBroker>,
    // agent_id -> subscribed topics
    pub subscriptions: Arc<DashMap<String, Vec<String>>>,
    // agent_id -> capabilities
    pub capabilities: Arc<DashMap<String, Vec<CapabilityDescriptor>>>,
    // agent_id -> sender to push ServerEvent into gRPC stream task
    pub streams: Arc<DashMap<String, mpsc::Sender<ServerEvent>>>,
}

impl BridgeState {
    pub fn new(event_bus: Arc<EventBus>, action_broker: Arc<ActionBroker>) -> Self {
        Self {
            event_bus,
            action_broker,
            subscriptions: Arc::new(DashMap::new()),
            capabilities: Arc::new(DashMap::new()),
            streams: Arc::new(DashMap::new()),
        }
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
                error_message: "agent_id empty".into(),
            }));
        }
        self.state
            .subscriptions
            .insert(agent_id.clone(), req.subscribed_topics.clone());
        self.state
            .capabilities
            .insert(agent_id.clone(), req.capabilities.clone());
        info!(agent_id=%agent_id, topics=?req.subscribed_topics, caps=req.capabilities.len(), "Agent registered via Bridge");
        Ok(Response::new(AgentRegisterResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    type EventStreamStream = std::pin::Pin<
        Box<dyn futures_core::Stream<Item = std::result::Result<ServerEvent, Status>> + Send>,
    >;
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

        // Create outbound channel
        let (tx, rx) = mpsc::channel::<ServerEvent>(512);
        self.state.streams.insert(agent_id.clone(), tx.clone());
        let agent_id_for_inbound = agent_id.clone();

        // For each subscribed topic, spawn forwarding task
        if let Some(topics) = self.state.subscriptions.get(&agent_id).map(|v| v.clone()) {
            for topic in topics.iter() {
                let topic_clone = topic.clone();
                let event_bus = Arc::clone(&self.state.event_bus);
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Ok((_sub_id, mut rx_bus)) = event_bus
                        .subscribe(
                            topic_clone.clone(),
                            vec![],
                            loom_proto::QoSLevel::QosBatched,
                        )
                        .await
                    {
                        while let Some(ev) = rx_bus.recv().await {
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
                    }
                });
            }
        }

        // Spawn task handling inbound messages
        let event_bus = Arc::clone(&self.state.event_bus);
        let tx_in = tx.clone();
        let streams_map = self.state.streams.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = inbound.message().await.transpose() {
                match msg.msg {
                    Some(client_event::Msg::Publish(p)) => {
                        if let (Some(ev), topic) = (p.event, p.topic) {
                            let _ = event_bus.publish(&topic, ev).await;
                        }
                    }
                    Some(client_event::Msg::Ping(_hb)) => {
                        let _ = tx_in
                            .send(ServerEvent {
                                msg: Some(server_event::Msg::Pong(HeartbeatResponse {
                                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                                    status: "ok".into(),
                                })),
                            })
                            .await;
                    }
                    Some(client_event::Msg::ActionResult(ar)) => {
                        // For now just log; real flow would correlate and maybe push somewhere
                        info!(action_id=%ar.id, "Received action result from agent");
                    }
                    Some(client_event::Msg::Ack(_)) => { /* ignore */ }
                    None => {}
                }
            }
            info!(agent_id=%agent_id_for_inbound, "EventStream inbound ended");
            // Cleanup stream sender on disconnect
            streams_map.remove(&agent_id_for_inbound);
        });

        let id_for_log = agent_id;
        let outbound = ReceiverStream::new(rx).map(|ev| Ok(ev));
        info!(agent_id=%id_for_log, "EventStream outbound established");
        Ok(Response::new(Box::pin(outbound) as Self::EventStreamStream))
    }

    async fn forward_action(
        &self,
        request: Request<ActionCall>,
    ) -> std::result::Result<Response<ActionResult>, Status> {
        let call = request.into_inner();
        let broker = Arc::clone(&self.state.action_broker);
        match broker.invoke(call.clone()).await {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Ok(Response::new(ActionResult {
                id: call.id,
                status: loom_proto::ActionStatus::ActionError as i32,
                output: Vec::new(),
                error: Some(loom_proto::ActionError {
                    code: "BROKER_ERROR".into(),
                    message: e.to_string(),
                    details: Default::default(),
                }),
            })),
        }
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> std::result::Result<Response<HeartbeatResponse>, Status> {
        Ok(Response::new(HeartbeatResponse {
            timestamp_ms: request.into_inner().timestamp_ms,
            status: "ok".into(),
        }))
    }
}

pub async fn start_server(
    addr: std::net::SocketAddr,
    event_bus: Arc<EventBus>,
    action_broker: Arc<ActionBroker>,
) -> Result<()> {
    let svc = BridgeService::new(BridgeState::new(event_bus, action_broker));
    info!(%addr, "Starting Loom Bridge gRPC server");
    tonic::transport::Server::builder()
        .add_service(BridgeServer::new(svc))
        .serve(addr)
        .await
        .map_err(|e| BridgeError::Internal(e.to_string()))
}
