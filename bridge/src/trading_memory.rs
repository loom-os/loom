//! In-Memory Trading Memory Store
//!
//! Provides memory storage for market-analyst agents, including:
//! - Trading plan storage and retrieval
//! - Execution record tracking
//! - Duplicate detection
//! - Event history
//!
//! This is a specialized memory store for the market-analyst demo,
//! separate from the general-purpose context memory in loom-core.

use async_trait::async_trait;
use dashmap::DashMap;
use loom_core::context::{MemoryReader, MemoryWriter};
use loom_proto::{
    CheckDuplicateRequest, CheckDuplicateResponse, CheckExecutedRequest, CheckExecutedResponse,
    Event, ExecutionRecord, GetExecutionStatsRequest, GetExecutionStatsResponse,
    GetRecentPlansRequest, GetRecentPlansResponse, MarkExecutedRequest, MarkExecutedResponse,
    PlanRecord, SavePlanRequest, SavePlanResponse,
};
use std::sync::Arc;
use tracing::debug;

/// Error type for InMemoryMemory operations
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    #[error("Plan is required")]
    PlanRequired,

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<MemoryError> for loom_core::LoomError {
    fn from(e: MemoryError) -> Self {
        loom_core::LoomError::AgentError(e.to_string())
    }
}

/// A simple in-memory memory store for demo/testing.
/// Stores textual summaries of events keyed by session id.
/// Also stores structured trading plans and execution records for market-analyst agents.
#[derive(Default)]
pub struct InMemoryMemory {
    // session -> list of event summary lines (legacy)
    store: DashMap<String, Vec<String>>,

    // session_id -> list of trading plans
    plans: DashMap<String, Vec<PlanRecord>>,

    // session_id -> list of execution records
    executed_plans: DashMap<String, Vec<ExecutionRecord>>,
}

impl InMemoryMemory {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: DashMap::new(),
            plans: DashMap::new(),
            executed_plans: DashMap::new(),
        })
    }

    fn summarize_event(event: &Event) -> String {
        // Minimal summary without parsing payload
        format!(
            "[{ts}] {ty} from {src}",
            ts = event.timestamp_ms,
            ty = event.r#type,
            src = event.source
        )
    }

    // === Trading Plan Management (for market-analyst agents) ===

    /// Save a trading plan to memory
    pub fn save_plan(&self, req: SavePlanRequest) -> Result<SavePlanResponse, MemoryError> {
        debug!(
            session_id = %req.session_id,
            symbol = %req.plan.as_ref().map(|p| p.symbol.as_str()).unwrap_or("unknown"),
            "Saving plan to memory"
        );

        let plan = req.plan.ok_or(MemoryError::PlanRequired)?;

        self.plans
            .entry(req.session_id.clone())
            .or_default()
            .push(plan.clone());

        // Keep only last 100 plans per session to avoid memory bloat
        if let Some(mut plans) = self.plans.get_mut(&req.session_id) {
            if plans.len() > 100 {
                let drain_count = plans.len() - 100;
                plans.drain(0..drain_count);
            }
        }

        Ok(SavePlanResponse {
            success: true,
            plan_hash: plan.plan_hash.clone(),
            error_message: String::new(),
        })
    }

    /// Get recent plans for a symbol
    pub fn get_recent_plans(
        &self,
        req: GetRecentPlansRequest,
    ) -> Result<GetRecentPlansResponse, MemoryError> {
        debug!(
            session_id = %req.session_id,
            symbol = %req.symbol,
            limit = req.limit,
            "Retrieving recent plans"
        );

        let limit = req.limit.clamp(1, 100) as usize;

        let plans = self
            .plans
            .get(&req.session_id)
            .map(|entry| {
                entry
                    .iter()
                    .filter(|p| p.symbol == req.symbol)
                    .rev()
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(GetRecentPlansResponse {
            plans,
            success: true,
            error_message: String::new(),
        })
    }

    /// Check if a plan is a duplicate
    pub fn check_duplicate(
        &self,
        req: CheckDuplicateRequest,
    ) -> Result<CheckDuplicateResponse, MemoryError> {
        debug!(
            session_id = %req.session_id,
            "Checking for duplicate plan"
        );

        let time_window_ms = (req.time_window_sec.max(60) as i64) * 1000;

        // Get timestamp from the plan being checked, or use current time
        let check_ts = req
            .plan
            .as_ref()
            .map(|p| p.timestamp_ms)
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        // Find a duplicate plan if it exists
        let duplicate = req.plan.as_ref().and_then(|plan| {
            self.plans.get(&req.session_id).and_then(|entry| {
                entry
                    .iter()
                    .find(|p| {
                        p.symbol == plan.symbol
                            && p.action == plan.action
                            && (check_ts - p.timestamp_ms).abs() < time_window_ms
                    })
                    .cloned()
            })
        });

        let time_since = duplicate
            .as_ref()
            .map(|d| (check_ts - d.timestamp_ms).abs())
            .unwrap_or(0);

        Ok(CheckDuplicateResponse {
            is_duplicate: duplicate.is_some(),
            duplicate_plan: duplicate,
            time_since_duplicate_ms: time_since,
        })
    }

    /// Mark a plan as executed
    pub fn mark_executed(
        &self,
        req: MarkExecutedRequest,
    ) -> Result<MarkExecutedResponse, MemoryError> {
        debug!(
            session_id = %req.session_id,
            plan_hash = %req.plan_hash,
            "Marking plan as executed"
        );

        // Use provided execution record or create a minimal one
        let record = req.execution.unwrap_or_else(|| ExecutionRecord {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            plan_hash: req.plan_hash.clone(),
            symbol: String::new(),
            action: String::new(),
            confidence: 0.0,
            status: "executed".to_string(),
            executed: true,
            order_id: String::new(),
            order_size_usdt: 0.0,
            error_message: String::new(),
        });

        self.executed_plans
            .entry(req.session_id.clone())
            .or_default()
            .push(record);

        // Keep only last 1000 execution records per session
        if let Some(mut records) = self.executed_plans.get_mut(&req.session_id) {
            if records.len() > 1000 {
                let drain_count = records.len() - 1000;
                records.drain(0..drain_count);
            }
        }

        Ok(MarkExecutedResponse {
            success: true,
            error_message: String::new(),
        })
    }

    /// Check if a plan was executed
    pub fn check_executed(
        &self,
        req: CheckExecutedRequest,
    ) -> Result<CheckExecutedResponse, MemoryError> {
        debug!(
            session_id = %req.session_id,
            plan_hash = %req.plan_hash,
            "Checking if plan was executed"
        );

        let execution = self
            .executed_plans
            .get(&req.session_id)
            .and_then(|entry| entry.iter().find(|r| r.plan_hash == req.plan_hash).cloned());

        Ok(CheckExecutedResponse {
            is_executed: execution.is_some(),
            execution,
        })
    }

    /// Get execution statistics for a symbol
    pub fn get_execution_stats(
        &self,
        req: GetExecutionStatsRequest,
    ) -> Result<GetExecutionStatsResponse, MemoryError> {
        debug!(
            session_id = %req.session_id,
            symbol = %req.symbol,
            "Getting execution stats"
        );

        let executions: Vec<ExecutionRecord> = self
            .executed_plans
            .get(&req.session_id)
            .map(|entry| {
                entry
                    .iter()
                    .filter(|r| req.symbol.is_empty() || r.symbol == req.symbol)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let total_executions = executions.len() as i32;
        let successful_executions =
            executions.iter().filter(|r| r.status == "success").count() as i32;
        let failed_executions = executions.iter().filter(|r| r.status == "error").count() as i32;
        let win_rate = if total_executions > 0 {
            successful_executions as f32 / total_executions as f32
        } else {
            0.0
        };

        // Count duplicates prevented (plans that were not executed due to duplicate check)
        let duplicate_prevented = 0; // This would need additional tracking

        // Get recent executions (last 10)
        let recent_executions: Vec<ExecutionRecord> =
            executions.iter().rev().take(10).cloned().collect();

        Ok(GetExecutionStatsResponse {
            total_executions,
            successful_executions,
            failed_executions,
            win_rate,
            duplicate_prevented,
            recent_executions,
        })
    }
}

#[async_trait]
impl MemoryWriter for InMemoryMemory {
    async fn append_event(&self, session: &str, event: Event) -> loom_core::Result<()> {
        let summary = Self::summarize_event(&event);
        self.store
            .entry(session.to_string())
            .or_default()
            .push(summary);

        // Keep only last 500 events per session
        if let Some(mut events) = self.store.get_mut(session) {
            if events.len() > 500 {
                let drain_count = events.len() - 500;
                events.drain(0..drain_count);
            }
        }

        Ok(())
    }

    async fn summarize_episode(&self, session: &str) -> loom_core::Result<Option<String>> {
        let summary = self.store.get(session).map(|entry| {
            entry
                .iter()
                .take(50) // Limit for summary
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        });
        Ok(summary)
    }
}

#[async_trait]
impl MemoryReader for InMemoryMemory {
    async fn retrieve(
        &self,
        query: &str,
        k: usize,
        _filters: Option<serde_json::Value>,
    ) -> loom_core::Result<Vec<String>> {
        // Simple substring search across all sessions
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for entry in self.store.iter() {
            for event_summary in entry.value().iter() {
                if event_summary.to_lowercase().contains(&query_lower) {
                    results.push(event_summary.clone());
                    if results.len() >= k {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_plan(symbol: &str, action: &str) -> PlanRecord {
        PlanRecord {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            symbol: symbol.to_string(),
            action: action.to_string(),
            confidence: 0.8,
            reasoning: "Test plan".to_string(),
            plan_hash: format!("{}_{}_{}", symbol, action, chrono::Utc::now().timestamp()),
            method: "llm".to_string(),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_save_and_get_plans() {
        let memory = InMemoryMemory::new();
        let session_id = "test-session";

        let plan = create_test_plan("BTCUSDT", "BUY");

        let save_req = SavePlanRequest {
            session_id: session_id.to_string(),
            plan: Some(plan.clone()),
        };

        let save_resp = memory.save_plan(save_req).unwrap();
        assert!(save_resp.success);

        let get_req = GetRecentPlansRequest {
            session_id: session_id.to_string(),
            symbol: "BTCUSDT".to_string(),
            limit: 10,
        };

        let get_resp = memory.get_recent_plans(get_req).unwrap();
        assert!(get_resp.success);
        assert_eq!(get_resp.plans.len(), 1);
        assert_eq!(get_resp.plans[0].symbol, "BTCUSDT");
    }

    #[tokio::test]
    async fn test_check_duplicate() {
        let memory = InMemoryMemory::new();
        let session_id = "test-session";

        let plan = create_test_plan("ETHUSDT", "SELL");

        // Save the plan first
        memory
            .save_plan(SavePlanRequest {
                session_id: session_id.to_string(),
                plan: Some(plan.clone()),
            })
            .unwrap();

        // Check for duplicate - should be true
        let check_req = CheckDuplicateRequest {
            session_id: session_id.to_string(),
            plan: Some(plan.clone()),
            time_window_sec: 300,
        };

        let check_resp = memory.check_duplicate(check_req).unwrap();
        assert!(check_resp.is_duplicate);

        // Check with different action - should not be duplicate
        let mut different_plan = plan.clone();
        different_plan.action = "BUY".to_string();

        let check_req2 = CheckDuplicateRequest {
            session_id: session_id.to_string(),
            plan: Some(different_plan),
            time_window_sec: 300,
        };

        let check_resp2 = memory.check_duplicate(check_req2).unwrap();
        assert!(!check_resp2.is_duplicate);
    }

    #[tokio::test]
    async fn test_execution_tracking() {
        let memory = InMemoryMemory::new();
        let session_id = "test-session";

        let plan = create_test_plan("BTCUSDT", "BUY");
        let plan_hash = plan.plan_hash.clone();

        // Save the plan
        memory
            .save_plan(SavePlanRequest {
                session_id: session_id.to_string(),
                plan: Some(plan),
            })
            .unwrap();

        // Check not executed yet
        let check_req = CheckExecutedRequest {
            session_id: session_id.to_string(),
            plan_hash: plan_hash.clone(),
        };

        let check_resp = memory.check_executed(check_req).unwrap();
        assert!(!check_resp.is_executed);

        // Mark as executed
        let execution = ExecutionRecord {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            plan_hash: plan_hash.clone(),
            symbol: "BTCUSDT".to_string(),
            action: "BUY".to_string(),
            confidence: 0.8,
            status: "success".to_string(),
            executed: true,
            order_id: "order-123".to_string(),
            order_size_usdt: 100.0,
            error_message: String::new(),
        };

        let mark_req = MarkExecutedRequest {
            session_id: session_id.to_string(),
            plan_hash: plan_hash.clone(),
            execution: Some(execution),
        };

        memory.mark_executed(mark_req).unwrap();

        // Check executed now
        let check_req2 = CheckExecutedRequest {
            session_id: session_id.to_string(),
            plan_hash: plan_hash.clone(),
        };

        let check_resp2 = memory.check_executed(check_req2).unwrap();
        assert!(check_resp2.is_executed);
        assert!(check_resp2.execution.is_some());
    }
}
