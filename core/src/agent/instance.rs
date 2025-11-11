use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::action_broker::ActionBroker;
use crate::proto::{Action, AgentConfig, AgentState};
use crate::router::{
    AgentContext, ModelRouter, PrivacyLevel, Route, RoutingDecision, RoutingPolicy,
};
use crate::{Envelope, Event, EventBus, Result};

use super::behavior::AgentBehavior;

/// Agent instance
pub struct Agent {
    pub(crate) config: AgentConfig,
    pub(crate) state: Arc<RwLock<AgentState>>,
    pub(crate) behavior: Box<dyn AgentBehavior>,
    pub(crate) event_rx: tokio::sync::mpsc::Receiver<Event>,
    pub(crate) action_broker: Arc<ActionBroker>,
    pub(crate) event_bus: Arc<EventBus>,
    pub(crate) model_router: ModelRouter,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        behavior: Box<dyn AgentBehavior>,
        event_rx: tokio::sync::mpsc::Receiver<Event>,
        action_broker: Arc<ActionBroker>,
        event_bus: Arc<EventBus>,
        model_router: ModelRouter,
    ) -> Self {
        let state = AgentState {
            agent_id: config.agent_id.clone(),
            persistent_state: vec![],
            ephemeral_context: vec![],
            last_update_ms: chrono::Utc::now().timestamp_millis(),
            metadata: config.parameters.clone(),
        };

        Self {
            config,
            state: Arc::new(RwLock::new(state)),
            behavior,
            event_rx,
            action_broker,
            event_bus,
            model_router,
        }
    }

    /// Start agent event loop
    pub async fn run(mut self) -> Result<()> {
        info!("Agent {} starting", self.config.agent_id);

        // Initialize
        self.behavior.on_init(&self.config).await?;

        // Event loop
        while let Some(mut event) = self.event_rx.recv().await {
            debug!("Agent {} received event {}", self.config.agent_id, event.id);

            // Ensure envelope metadata present; attach defaults if missing
            let mut env = Envelope::from_event(&event);
            if env.sender.is_empty() {
                env.sender = format!("agent.{}", self.config.agent_id);
            }
            // Increment hop & ttl; drop if expired
            if !env.next_hop() {
                debug!("Dropping event {} due to TTL exhaustion", event.id);
                continue;
            }
            env.attach_to_event(&mut event);

            // Snapshot state (read) for routing context
            let state_snapshot = {
                let state_read = self.state.read().await;
                state_read.clone()
            };

            // Route the event first
            let decision = self.route_event(&event, &state_snapshot, &env).await;

            match self.handle_with_route(event, decision).await {
                Ok(actions) => {
                    // Execute actions
                    for action in actions {
                        self.execute_action(action).await?;
                    }
                }
                Err(e) => {
                    warn!("Agent {} error handling event: {}", self.config.agent_id, e);
                }
            }

            // Update timestamp
            {
                let mut state = self.state.write().await;
                state.last_update_ms = chrono::Utc::now().timestamp_millis();
            }
        }

        // Cleanup
        self.behavior.on_shutdown().await?;
        info!("Agent {} stopped", self.config.agent_id);

        Ok(())
    }

    /// Determine routing for the event, log the decision, and publish an observability event
    async fn route_event(
        &self,
        event: &Event,
        state: &AgentState,
        env: &Envelope,
    ) -> RoutingDecision {
        // Build optional AgentContext
        let ctx = AgentContext {
            recent_events: vec![],
            current_task: state.metadata.get("current_task").cloned(),
            available_quota: state
                .metadata
                .get("available_quota")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.0),
        };

        // Effective policy: base router policy with optional overrides from config
        let effective_policy = self.effective_policy();
        let router = self.model_router.with_policy(effective_policy.clone());

        let decision = match router.route(event, Some(&ctx)).await {
            Ok(d) => d,
            Err(e) => {
                warn!(
                    "Router error for agent {}: {}. Falling back to Local.",
                    self.config.agent_id, e
                );
                RoutingDecision {
                    route: Route::Local,
                    confidence: 0.0,
                    reason: "Router error fallback to local".to_string(),
                    estimated_latency_ms: 0,
                    estimated_cost: 0.0,
                }
            }
        };

        // Log decision with reason
        info!(
            target: "router", agent_id = %self.config.agent_id, event_id = %event.id,
            route = ?decision.route, confidence = decision.confidence,
            reason = %decision.reason, est_latency_ms = decision.estimated_latency_ms,
            est_cost = decision.estimated_cost,
            privacy = ?effective_policy.privacy_level,
            latency_budget_ms = effective_policy.latency_budget_ms,
            cost_cap = effective_policy.cost_cap_per_event,
            quality_threshold = effective_policy.quality_threshold,
            "Routing decision"
        );

        // Publish a routing_decision event for observability (best-effort)
        let mut md = std::collections::HashMap::new();
        md.insert("route".into(), format!("{:?}", decision.route));
        md.insert("reason".into(), decision.reason.clone());
        md.insert("confidence".into(), format!("{:.3}", decision.confidence));
        md.insert(
            "est_latency_ms".into(),
            decision.estimated_latency_ms.to_string(),
        );
        md.insert("est_cost".into(), format!("{:.4}", decision.estimated_cost));

        let mut obs_evt = Event {
            id: format!("evt_route_{}", chrono::Utc::now().timestamp_millis()),
            r#type: "routing_decision".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            source: format!("agent.{}", self.config.agent_id),
            metadata: md,
            payload: Vec::new(),
            confidence: decision.confidence,
            tags: vec!["router".into()],
            priority: 50,
        };
        env.attach_to_event(&mut obs_evt);
        let _ = self
            .event_bus
            .publish(&format!("agent.{}", self.config.agent_id), obs_evt)
            .await;

        decision
    }

    /// Execute behavior according to routing decision. In Hybrid, perform local quick pass then cloud refine.
    async fn handle_with_route(
        &mut self,
        mut event: Event,
        decision: RoutingDecision,
    ) -> Result<Vec<Action>> {
        // annotate event with routing decision for downstream behavior logic
        event
            .metadata
            .insert("routing_decision".into(), format!("{:?}", decision.route));
        event
            .metadata
            .insert("routing_reason".into(), decision.reason.clone());

        match decision.route {
            Route::Local | Route::LocalFallback => {
                let mut state = self.state.write().await;
                self.behavior.on_event(event, &mut state).await
            }
            Route::Cloud => {
                event
                    .metadata
                    .insert("routing_target".into(), "cloud".into());
                let mut state = self.state.write().await;
                self.behavior.on_event(event, &mut state).await
            }
            Route::Hybrid => {
                // First pass: local quick
                let mut local_evt = event.clone();
                local_evt
                    .metadata
                    .insert("routing_target".into(), "local".into());
                local_evt.metadata.insert("phase".into(), "quick".into());
                let mut actions = {
                    let mut state = self.state.write().await;
                    self.behavior
                        .on_event(local_evt, &mut state)
                        .await
                        .unwrap_or_default()
                };

                // Second pass: cloud refine (sequential, marked for refinement)
                let mut cloud_evt = event;
                cloud_evt
                    .metadata
                    .insert("routing_target".into(), "cloud".into());
                cloud_evt.metadata.insert("phase".into(), "refine".into());
                // Signal to behavior that this is a refinement pass
                cloud_evt.metadata.insert("refine".into(), "true".into());
                let mut refine_actions = {
                    let mut state = self.state.write().await;
                    self.behavior
                        .on_event(cloud_evt, &mut state)
                        .await
                        .unwrap_or_default()
                };
                actions.append(&mut refine_actions);
                Ok(actions)
            }
            Route::Defer | Route::Drop => {
                // No-op action list
                Ok(vec![])
            }
        }
    }

    /// Compute effective routing policy from agent config parameters (fallback to router defaults)
    fn effective_policy(&self) -> RoutingPolicy {
        let base = self.model_router.policy();
        let p = &self.config.parameters;

        let privacy = p
            .get("routing.privacy")
            .map(|s| match s.as_str() {
                "public" => PrivacyLevel::Public,
                "sensitive" => PrivacyLevel::Sensitive,
                "private" => PrivacyLevel::Private,
                "local-only" => PrivacyLevel::LocalOnly,
                _ => base.privacy_level.clone(),
            })
            .unwrap_or(base.privacy_level.clone());

        let latency_budget_ms = p
            .get("routing.latency_budget_ms")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(base.latency_budget_ms);

        let cost_cap_per_event = p
            .get("routing.cost_cap")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(base.cost_cap_per_event);

        let quality_threshold = p
            .get("routing.quality_threshold")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(base.quality_threshold);

        RoutingPolicy {
            privacy_level: privacy,
            latency_budget_ms,
            cost_cap_per_event,
            quality_threshold,
        }
    }

    async fn execute_action(&self, action: Action) -> Result<()> {
        use crate::proto::{ActionCall, ActionStatus, QoSLevel};
        debug!("Executing action: {}", action.action_type);

        // Map priority to QoS
        let qos = if action.priority >= 70 {
            QoSLevel::QosRealtime
        } else if action.priority >= 30 {
            QoSLevel::QosBatched
        } else {
            QoSLevel::QosBackground
        };

        // Convert parameters into headers for the call
        let headers = action.parameters.clone();

        // Build ActionCall
        let now = chrono::Utc::now();
        let call_id = format!(
            "act_{}",
            now.timestamp_nanos_opt()
                .unwrap_or_else(|| now.timestamp_millis() * 1_000_000)
        );
        let mut call = ActionCall {
            id: call_id.clone(),
            capability: action.action_type.clone(),
            version: "".to_string(), // resolve first provider by name if version unspecified
            payload: action.payload.clone(),
            headers,
            timeout_ms: 0, // broker default (30s)
            correlation_id: self.config.agent_id.clone(),
            qos: qos as i32,
        };

        // Attach envelope into call headers
        let mut env = Envelope::new(call_id.clone(), format!("agent.{}", self.config.agent_id));
        env.correlation_id = call_id.clone();
        env.apply_to_action_call(&mut call);

        let res = self.action_broker.invoke(call).await?;

        // Optionally publish result event for observability
        let mut evt = Event {
            id: format!(
                "evt_action_result_{}",
                chrono::Utc::now().timestamp_millis()
            ),
            r#type: "action_result".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            source: format!("agent.{}", self.config.agent_id),
            metadata: {
                let mut m = std::collections::HashMap::new();
                m.insert("action_type".into(), action.action_type.clone());
                m.insert(
                    "status".into(),
                    match res.status {
                        x if x == ActionStatus::ActionOk as i32 => "ok".into(),
                        x if x == ActionStatus::ActionTimeout as i32 => "timeout".into(),
                        x if x == ActionStatus::ActionRetryable as i32 => "retryable".into(),
                        _ => "error".into(),
                    },
                );
                m
            },
            payload: res.output.clone(),
            confidence: 1.0,
            tags: vec!["action".into()],
            priority: action.priority,
        };
        // Reuse envelope from action call to maintain thread/correlation consistency
        env.attach_to_event(&mut evt);
        // Best-effort publish; ignore delivery count
        let _ = self
            .event_bus
            .publish(&format!("agent.{}", self.config.agent_id), evt)
            .await;

        Ok(())
    }
}
