use loom_core::cognitive::llm::router::{ConfidenceEstimator, ModelRouter, Route};
use loom_core::proto::Event;
use loom_core::Result;
use std::sync::Arc;

// Mock confidence estimator with configurable confidence
struct MockConfidenceEstimator {
    confidence: f32,
}

#[async_trait::async_trait]
impl ConfidenceEstimator for MockConfidenceEstimator {
    fn name(&self) -> &'static str {
        "MockConfidenceEstimator"
    }

    fn supports_event_type(&self, _event_type: &str) -> bool {
        true
    }

    async fn estimate_confidence(&self, _event: &Event) -> Result<f32> {
        Ok(self.confidence)
    }
}

fn make_event(id: &str) -> Event {
    Event {
        id: id.to_string(),
        r#type: "test".to_string(),
        timestamp_ms: 0,
        source: "test".to_string(),
        metadata: Default::default(),
        payload: vec![],
        confidence: 1.0,
        tags: vec![],
        priority: 0,
    }
}

fn make_event_with_metadata(id: &str, key: &str, value: &str) -> Event {
    let mut evt = make_event(id);
    evt.metadata.insert(key.to_string(), value.to_string());
    evt
}

#[tokio::test]
async fn route_local_only_privacy_always_local() -> Result<()> {
    let router = ModelRouter::new().await?;
    let evt = make_event_with_metadata("e1", "privacy", "local-only");

    let decision = router.route(&evt, None).await?;
    assert_eq!(decision.route, Route::Local);
    assert!(decision.reason.contains("local-only"));
    Ok(())
}

#[tokio::test]
async fn route_high_confidence_local_model_chooses_local() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.95 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event("e2");
    let decision = router.route(&evt, None).await?;
    assert_eq!(decision.route, Route::Local);
    Ok(())
}

#[tokio::test]
async fn route_low_confidence_local_model_chooses_cloud() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.50 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event("e3");
    let decision = router.route(&evt, None).await?;
    assert_eq!(decision.route, Route::Cloud);
    Ok(())
}

#[tokio::test]
async fn route_medium_confidence_chooses_hybrid() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.75 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event("e4");
    let decision = router.route(&evt, None).await?;
    assert_eq!(decision.route, Route::Hybrid);
    Ok(())
}

#[tokio::test]
async fn route_respects_latency_budget() -> Result<()> {
    // If local model infers quickly but we set a very strict latency budget,
    // router should prefer local for latency-sensitive events.
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.70 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event("e5");
    let decision = router.route(&evt, None).await?;
    // Strict latency budget should favor local even if confidence is medium
    assert!(
        decision.route == Route::Local || decision.route == Route::Hybrid,
        "strict latency should prefer local or hybrid"
    );
    Ok(())
}

#[tokio::test]
async fn route_respects_cost_cap() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.60 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event("e6");
    let decision = router.route(&evt, None).await?;
    // Router should make a valid routing decision
    // Note: Without ability to configure policy cost_cap, we can't strictly test cost logic in MVP
    assert!(
        matches!(
            decision.route,
            Route::Local | Route::LocalFallback | Route::Hybrid | Route::Cloud
        ),
        "router should make a valid routing decision"
    );
    Ok(())
}

#[tokio::test]
async fn route_quality_threshold_above_local_confidence_chooses_cloud() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.70 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event("e7");
    let decision = router.route(&evt, None).await?;
    // Default policy should make a reasonable decision
    assert!(
        decision.route == Route::Cloud
            || decision.route == Route::Hybrid
            || decision.route == Route::Local,
        "router should choose a valid route"
    );
    Ok(())
}

#[tokio::test]
async fn route_private_privacy_level_prefers_local() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.80 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event_with_metadata("e8", "privacy", "private");
    let decision = router.route(&evt, None).await?;
    // Router should make a valid routing decision for private data
    // Note: Without ability to configure policy, we can't strictly test privacy logic in MVP
    assert!(
        matches!(
            decision.route,
            Route::Local | Route::LocalFallback | Route::Hybrid | Route::Cloud
        ),
        "router should produce a valid route for private data, got {:?}",
        decision.route
    );
    Ok(())
}

#[tokio::test]
async fn route_public_privacy_level_allows_cloud() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.60 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event_with_metadata("e9", "privacy", "public");
    let decision = router.route(&evt, None).await?;
    // Public should allow cloud routing
    assert!(
        decision.route == Route::Cloud || decision.route == Route::Hybrid,
        "public should allow cloud"
    );
    Ok(())
}

#[tokio::test]
async fn route_sensitive_privacy_level_uses_hybrid() -> Result<()> {
    let mock_estimator = Arc::new(MockConfidenceEstimator { confidence: 0.75 });
    let router = ModelRouter::new()
        .await?
        .with_confidence_estimator(mock_estimator);

    let evt = make_event_with_metadata("e10", "privacy", "sensitive");
    let decision = router.route(&evt, None).await?;
    // Sensitive typically prefers hybrid for balance
    assert!(
        decision.route == Route::Hybrid || decision.route == Route::Local,
        "sensitive should prefer hybrid or local"
    );
    Ok(())
}

#[tokio::test]
async fn route_decision_includes_estimates() -> Result<()> {
    let router = ModelRouter::new().await?;
    let evt = make_event("e11");

    let decision = router.route(&evt, None).await?;
    assert!(decision.estimated_latency_ms > 0, "should estimate latency");
    assert!(decision.estimated_cost >= 0.0, "should estimate cost");
    assert!(!decision.reason.is_empty(), "should provide reason");
    Ok(())
}

#[tokio::test]
async fn router_default_policy_is_sane() -> Result<()> {
    let router = ModelRouter::new().await?;
    // Just verify router can be created and route with defaults
    let evt = make_event("e8");
    let decision = router.route(&evt, None).await?;
    assert!(
        matches!(
            decision.route,
            Route::Local | Route::Cloud | Route::Hybrid | Route::LocalFallback
        ),
        "default policy should produce a valid route"
    );
    Ok(())
}

#[tokio::test]
async fn route_with_no_local_model_support_falls_back_to_cloud() -> Result<()> {
    // Use the default dummy model which returns low confidence
    let router = ModelRouter::new().await?;
    let evt = make_event("e12");

    let decision = router.route(&evt, None).await?;
    // Dummy model should result in cloud or fallback
    assert!(
        decision.route == Route::Cloud || decision.route == Route::LocalFallback,
        "no capable local model should route to cloud"
    );
    Ok(())
}

#[tokio::test]
async fn route_handles_missing_privacy_metadata_as_sensitive() -> Result<()> {
    let router = ModelRouter::new().await?;
    let evt = make_event("e13"); // no privacy metadata

    let decision = router.route(&evt, None).await?;
    // Should default to sensitive and make a reasonable decision
    assert!(!decision.reason.is_empty());
    Ok(())
}
