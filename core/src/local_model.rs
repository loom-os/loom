// Local model interface (stub)
//
// This trait defines the minimal interface for a local on-device model that can be
// used by the router for Local/Hybrid routing decisions. The goal is to allow
// swapping in real backends later (e.g., TFLite, ONNX Runtime) without changing
// router or agents.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::{proto::Event, Result};

/// Minimal output of a local inference. Extend this as implementations mature.
#[derive(Debug, Clone, Default)]
pub struct LocalInference {
    pub confidence: f32,
    pub metadata: HashMap<String, String>,
}

/// Minimal local model interface
#[async_trait]
pub trait LocalModel: Send + Sync {
    /// A static identifier for the model implementation
    fn name(&self) -> &'static str;

    /// Quick capability check without heavy computation
    fn supports_event_type(&self, event_type: &str) -> bool;

    /// Lightweight confidence estimation for routing decisions
    async fn estimate_confidence(&self, event: &Event) -> Result<f32>;

    /// Optional full inference path used by Local/Hybrid execution
    async fn infer(&self, event: &Event) -> Result<LocalInference>;
}

/// A no-op dummy model used as the default placeholder.
///
/// Behavior:
/// - Claims support for common edge event types (video, audio, face)
/// - Returns a fixed confidence for estimation and inference
#[derive(Debug, Default, Clone)]
pub struct DummyLocalModel;

#[async_trait]
impl LocalModel for DummyLocalModel {
    fn name(&self) -> &'static str {
        "dummy-local-model"
    }

    fn supports_event_type(&self, event_type: &str) -> bool {
        matches!(event_type, "video_frame" | "audio_chunk" | "face_event")
    }

    async fn estimate_confidence(&self, _event: &Event) -> Result<f32> {
        // Simulate local model confidence
        Ok(0.87)
    }

    async fn infer(&self, _event: &Event) -> Result<LocalInference> {
        Ok(LocalInference {
            confidence: 0.87,
            metadata: HashMap::from([
                ("provider".to_string(), self.name().to_string()),
                ("mode".to_string(), "stub".to_string()),
            ]),
        })
    }
}
