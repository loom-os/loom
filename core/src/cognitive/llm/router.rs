// Model Router implementation
//
// The Router makes Local/Cloud/Hybrid routing decisions based on policy
// (privacy, latency, cost, quality) and confidence estimates.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, Span};

use crate::{proto::Event, Result};

// OpenTelemetry imports
use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
    KeyValue,
};

// ============================================================================
// Confidence Estimator
// ============================================================================

/// Confidence estimator for routing decisions.
///
/// This trait is used by the Router to quickly assess whether a local model
/// can handle an event with sufficient confidence, without performing full inference.
/// Implementations should be fast and lightweight (< 10ms).
///
/// Future ML inference capabilities (e.g., ONNX Runtime, TensorFlow Lite) can
/// implement this trait for Router integration while exposing their own
/// comprehensive inference APIs independently.
#[async_trait]
pub trait ConfidenceEstimator: Send + Sync {
    /// A static identifier for the estimator implementation
    fn name(&self) -> &'static str;

    /// Quick capability check without heavy computation.
    ///
    /// Returns true if this estimator can provide meaningful confidence scores
    /// for the given event type.
    fn supports_event_type(&self, event_type: &str) -> bool;

    /// Lightweight confidence estimation for routing decisions.
    ///
    /// Returns a confidence score between 0.0 and 1.0 indicating how well
    /// a local model could handle this event. Higher values suggest the Router
    /// should prefer local processing.
    async fn estimate_confidence(&self, event: &Event) -> Result<f32>;
}

/// A no-op dummy estimator used as the default placeholder.
///
/// Behavior:
/// - Claims support for common edge event types (video, audio, face)
/// - Returns a fixed confidence score of 0.87
///
/// This is useful for testing and development. In production, replace with a real
/// estimator based on historical data, heuristics, or lightweight model checks.
#[derive(Debug, Default, Clone)]
pub struct DummyConfidenceEstimator;

#[async_trait]
impl ConfidenceEstimator for DummyConfidenceEstimator {
    fn name(&self) -> &'static str {
        "dummy-confidence-estimator"
    }

    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(event_type, "video_frame" | "audio_chunk" | "face_event")
    }

    async fn estimate_confidence(&self, _event: &Event) -> Result<f32> {
        // Simulate a reasonable confidence score for routing decisions
        Ok(0.87)
    }
}

// ============================================================================
// Router Types
// ============================================================================

/// Routing target
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Route {
    Local,
    Cloud,
    Hybrid,
    LocalFallback,
    Defer,
    Drop,
}

/// Routing decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub route: Route,
    pub confidence: f32,
    pub reason: String,
    pub estimated_latency_ms: u64,
    pub estimated_cost: f32,
}

/// Routing policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub privacy_level: PrivacyLevel,
    pub latency_budget_ms: u64,
    pub cost_cap_per_event: f32,
    pub quality_threshold: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    Public,
    Sensitive,
    Private,
    LocalOnly,
}

/// Model Router core
#[derive(Clone)]
pub struct ModelRouter {
    policy: RoutingPolicy,
    local_models: Vec<String>,
    cloud_endpoints: Vec<String>,
    confidence_estimator: Arc<dyn ConfidenceEstimator>,

    // OpenTelemetry metrics
    decisions_counter: Counter<u64>,
    confidence_histogram: Histogram<f64>,
    estimated_latency_histogram: Histogram<f64>,
    estimated_cost_histogram: Histogram<f64>,
    policy_violations_counter: Counter<u64>,
}

impl ModelRouter {
    pub async fn new() -> Result<Self> {
        // Initialize OpenTelemetry metrics
        let meter = global::meter("loom.router");

        let decisions_counter = meter
            .u64_counter("loom.router.decisions_total")
            .with_description("Total number of routing decisions")
            .init();

        let confidence_histogram = meter
            .f64_histogram("loom.router.confidence_score")
            .with_description("Confidence score for routing decisions")
            .init();

        let estimated_latency_histogram = meter
            .f64_histogram("loom.router.estimated_latency_ms")
            .with_description("Estimated latency in milliseconds")
            .init();

        let estimated_cost_histogram = meter
            .f64_histogram("loom.router.estimated_cost")
            .with_description("Estimated cost per event")
            .init();

        let policy_violations_counter = meter
            .u64_counter("loom.router.policy_violations_total")
            .with_description("Total number of policy violations")
            .init();

        Ok(Self {
            policy: RoutingPolicy {
                privacy_level: PrivacyLevel::Sensitive,
                latency_budget_ms: 200,
                cost_cap_per_event: 0.01,
                quality_threshold: 0.85,
            },
            local_models: vec![
                "face_detector".to_string(),
                "emotion_classifier".to_string(),
                "lightweight_llm".to_string(),
            ],
            cloud_endpoints: vec!["gpt-4".to_string(), "claude-3".to_string()],
            confidence_estimator: Arc::new(DummyConfidenceEstimator),
            decisions_counter,
            confidence_histogram,
            estimated_latency_histogram,
            estimated_cost_histogram,
            policy_violations_counter,
        })
    }

    /// Create a router with an injected confidence estimator implementation
    pub fn with_confidence_estimator(mut self, estimator: Arc<dyn ConfidenceEstimator>) -> Self {
        self.confidence_estimator = estimator;
        self
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Model Router started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Model Router shutting down");
        Ok(())
    }

    /// Route event based on policy
    #[tracing::instrument(skip(self, event, _context), fields(event_id = %event.id, event_type = %event.r#type, route, confidence, reason))]
    pub async fn route(
        &self,
        event: &Event,
        _context: Option<&AgentContext>,
    ) -> Result<RoutingDecision> {
        debug!("Routing event: {} type: {}", event.id, event.r#type);

        // 1. Check privacy policy
        let privacy_level = event
            .metadata
            .get("privacy")
            .and_then(|p| match p.as_str() {
                "public" => Some(PrivacyLevel::Public),
                "sensitive" => Some(PrivacyLevel::Sensitive),
                "private" => Some(PrivacyLevel::Private),
                "local-only" => Some(PrivacyLevel::LocalOnly),
                _ => None,
            })
            .unwrap_or(PrivacyLevel::Sensitive);

        if privacy_level == PrivacyLevel::LocalOnly {
            let decision = RoutingDecision {
                route: Route::Local,
                confidence: 1.0,
                reason: "Privacy policy requires local-only processing".to_string(),
                estimated_latency_ms: 50,
                estimated_cost: 0.0,
            };
            self.record_decision(&decision, &event.r#type);
            return Ok(decision);
        }

        // 2. Check if local model supports event type
        let local_model_available = self.has_local_model_for(&event.r#type);
        let estimator_supports = self.confidence_estimator.supports_event_type(&event.r#type);
        let local_supported = local_model_available && estimator_supports;

        // 3. Estimate local confidence
        let local_confidence = if local_supported {
            self.confidence_estimator.estimate_confidence(event).await?
        } else {
            0.0
        };

        // 4. Make routing decision based on rules
        if local_supported && local_confidence >= self.policy.quality_threshold {
            let decision = RoutingDecision {
                route: Route::Local,
                confidence: local_confidence,
                reason: "Local model confidence exceeds threshold".to_string(),
                estimated_latency_ms: 50,
                estimated_cost: 0.0,
            };
            self.record_decision(&decision, &event.r#type);
            return Ok(decision);
        }

        // 5. Check latency budget
        if self.policy.latency_budget_ms < 100 {
            self.policy_violations_counter
                .add(1, &[KeyValue::new("violation_type", "latency_budget")]);
            let decision = RoutingDecision {
                route: Route::Local,
                confidence: local_confidence,
                reason: "Latency budget too tight for cloud".to_string(),
                estimated_latency_ms: 50,
                estimated_cost: 0.0,
            };
            self.record_decision(&decision, &event.r#type);
            return Ok(decision);
        }

        // 6. Check cost limit
        let cloud_cost = self.estimate_cloud_cost(event);
        if cloud_cost > self.policy.cost_cap_per_event {
            self.policy_violations_counter
                .add(1, &[KeyValue::new("violation_type", "cost_cap")]);
            let decision = RoutingDecision {
                route: Route::LocalFallback,
                confidence: local_confidence,
                reason: "Cloud cost exceeds budget".to_string(),
                estimated_latency_ms: 50,
                estimated_cost: 0.0,
            };
            self.record_decision(&decision, &event.r#type);
            return Ok(decision);
        }

        // 7. Hybrid strategy: local quick + cloud refine
        if local_supported && local_confidence > 0.5 {
            let decision = RoutingDecision {
                route: Route::Hybrid,
                confidence: local_confidence,
                reason: "Hybrid: local quick + cloud refine".to_string(),
                estimated_latency_ms: 300,
                estimated_cost: cloud_cost,
            };
            self.record_decision(&decision, &event.r#type);
            return Ok(decision);
        }

        // 8. Default to cloud if available, otherwise defer
        if self.has_cloud_endpoint() {
            let decision = RoutingDecision {
                route: Route::Cloud,
                confidence: 0.0,
                reason: "Default to cloud for quality".to_string(),
                estimated_latency_ms: 500,
                estimated_cost: cloud_cost,
            };
            self.record_decision(&decision, &event.r#type);
            Ok(decision)
        } else {
            // No cloud endpoints available - defer or drop
            let decision = RoutingDecision {
                route: Route::Defer,
                confidence: 0.0,
                reason: "No cloud endpoints available".to_string(),
                estimated_latency_ms: 0,
                estimated_cost: 0.0,
            };
            self.record_decision(&decision, &event.r#type);
            Ok(decision)
        }
    }

    // Helper method to record routing decision metrics and span attributes
    fn record_decision(&self, decision: &RoutingDecision, event_type: &str) {
        let route_str = format!("{:?}", decision.route);

        // Record metrics
        self.decisions_counter.add(
            1,
            &[
                KeyValue::new("route", route_str.clone()),
                KeyValue::new("reason", decision.reason.clone()),
                KeyValue::new("event_type", event_type.to_string()),
            ],
        );

        self.confidence_histogram.record(
            decision.confidence as f64,
            &[
                KeyValue::new("route", route_str.clone()),
                KeyValue::new("event_type", event_type.to_string()),
            ],
        );

        self.estimated_latency_histogram.record(
            decision.estimated_latency_ms as f64,
            &[KeyValue::new("route", route_str.clone())],
        );

        self.estimated_cost_histogram.record(
            decision.estimated_cost as f64,
            &[KeyValue::new("route", route_str.clone())],
        );

        // Record span attributes
        Span::current().record("route", &route_str);
        Span::current().record("confidence", decision.confidence);
        Span::current().record("reason", &decision.reason);
    }

    fn estimate_cloud_cost(&self, event: &Event) -> f32 {
        // Estimate cost based on event type and payload size
        match event.r#type.as_str() {
            "video_frame" => 0.005,
            "audio_chunk" => 0.002,
            "intent" => 0.01,
            _ => 0.001,
        }
    }
}

/// Agent context for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub recent_events: Vec<String>,
    pub current_task: Option<String>,
    pub available_quota: f32,
}

impl ModelRouter {
    /// Replace the current policy with a new one and return a new router instance
    pub fn with_policy(&self, policy: RoutingPolicy) -> Self {
        let mut cloned = self.clone();
        cloned.policy = policy;
        cloned
    }

    /// Get a copy of the active routing policy
    pub fn policy(&self) -> RoutingPolicy {
        self.policy.clone()
    }

    /// Check if a local model is available for the given event type
    ///
    /// This method maps event types to their corresponding local models
    /// and checks if the model is registered in the router.
    ///
    /// Returns true if:
    /// 1. A specific model mapping exists for the event type and is registered, OR
    /// 2. At least one local model is available (for unknown/generic event types)
    fn has_local_model_for(&self, event_type: &str) -> bool {
        // First check for explicit model mappings
        let model_name = match event_type {
            "video_frame" | "face_event" => Some("face_detector"),
            "audio_chunk" => Some("emotion_classifier"),
            "intent" | "chat" => Some("lightweight_llm"),
            _ => None,
        };

        if let Some(name) = model_name {
            // For known event types, check if the specific model is available
            self.local_models.iter().any(|m| m == name)
        } else {
            // For unknown event types, assume local models can handle if any are available
            // This allows the confidence estimator to make the final determination
            !self.local_models.is_empty()
        }
    }

    /// Check if a cloud endpoint is available
    ///
    /// Returns true if at least one cloud endpoint is configured.
    /// In production, this could be extended to check endpoint health,
    /// rate limits, or specific model capabilities.
    fn has_cloud_endpoint(&self) -> bool {
        !self.cloud_endpoints.is_empty()
    }

    /// Get available local models
    pub fn local_models(&self) -> &[String] {
        &self.local_models
    }

    /// Get available cloud endpoints
    pub fn cloud_endpoints(&self) -> &[String] {
        &self.cloud_endpoints
    }
}
