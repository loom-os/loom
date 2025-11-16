use super::{MemoryReader, MemoryWriter};
use crate::proto::{
    CheckDuplicateRequest, CheckDuplicateResponse, CheckExecutedRequest, CheckExecutedResponse,
    ExecutionRecord, GetExecutionStatsRequest, GetExecutionStatsResponse, GetRecentPlansRequest,
    GetRecentPlansResponse, MarkExecutedRequest, MarkExecutedResponse, PlanRecord, SavePlanRequest,
    SavePlanResponse,
};
use crate::{LoomError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Enhanced memory store for agent decision tracking
/// Stores plans, execution history, and generic events
#[derive(Default)]
pub struct AgentMemoryStore {
    // session_id -> list of plans
    plans: DashMap<String, Vec<PlanRecord>>,

    // session_id -> set of executed plan hashes
    executed_plans: DashMap<String, Vec<ExecutionRecord>>,

    // session_id -> list of event summaries (legacy support)
    events: DashMap<String, Vec<String>>,
}

impl AgentMemoryStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            plans: DashMap::new(),
            executed_plans: DashMap::new(),
            events: DashMap::new(),
        })
    }

    /// Save a trading plan to memory
    pub fn save_plan(&self, req: SavePlanRequest) -> Result<SavePlanResponse> {
        debug!(
            session_id = %req.session_id,
            symbol = %req.plan.as_ref().map(|p| p.symbol.as_str()).unwrap_or("unknown"),
            "Saving plan to memory"
        );

        let plan = req
            .plan
            .ok_or_else(|| LoomError::AgentError("Plan is required".to_string()))?;

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
    pub fn get_recent_plans(&self, req: GetRecentPlansRequest) -> Result<GetRecentPlansResponse> {
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

        // Reverse to get chronological order (oldest first)
        let mut plans = plans;
        plans.reverse();

        Ok(GetRecentPlansResponse {
            plans,
            success: true,
            error_message: String::new(),
        })
    }

    /// Check if a plan is duplicate within time window
    pub fn check_duplicate(&self, req: CheckDuplicateRequest) -> Result<CheckDuplicateResponse> {
        let plan = req
            .plan
            .ok_or_else(|| LoomError::AgentError("Plan is required".to_string()))?;
        let time_window_ms = (req.time_window_sec as i64) * 1000;

        debug!(
            session_id = %req.session_id,
            symbol = %plan.symbol,
            action = %plan.action,
            time_window_sec = req.time_window_sec,
            "Checking for duplicate plans"
        );

        let mut duplicate_found = None;

        if let Some(plans) = self.plans.get(&req.session_id) {
            // Look at recent plans in reverse chronological order
            for existing_plan in plans.iter().rev() {
                // Stop if outside time window
                let time_diff = plan.timestamp_ms - existing_plan.timestamp_ms;
                if time_diff > time_window_ms {
                    break;
                }

                // Check for duplicate: same symbol + action + hash
                if existing_plan.symbol == plan.symbol
                    && existing_plan.action == plan.action
                    && existing_plan.plan_hash == plan.plan_hash
                {
                    duplicate_found = Some((existing_plan.clone(), time_diff));
                    break;
                }
            }
        }

        if let Some((dup_plan, time_diff)) = duplicate_found {
            warn!(
                session_id = %req.session_id,
                symbol = %plan.symbol,
                action = %plan.action,
                time_since_ms = time_diff,
                "Duplicate plan detected"
            );

            Ok(CheckDuplicateResponse {
                is_duplicate: true,
                duplicate_plan: Some(dup_plan),
                time_since_duplicate_ms: time_diff,
            })
        } else {
            Ok(CheckDuplicateResponse {
                is_duplicate: false,
                duplicate_plan: None,
                time_since_duplicate_ms: 0,
            })
        }
    }

    /// Mark a plan as executed
    pub fn mark_executed(&self, req: MarkExecutedRequest) -> Result<MarkExecutedResponse> {
        let execution = req
            .execution
            .ok_or_else(|| LoomError::AgentError("Execution record is required".to_string()))?;

        debug!(
            session_id = %req.session_id,
            plan_hash = %req.plan_hash,
            status = %execution.status,
            "Marking plan as executed"
        );

        self.executed_plans
            .entry(req.session_id.clone())
            .or_default()
            .push(execution);

        // Keep only last 1000 execution records per session
        if let Some(mut executions) = self.executed_plans.get_mut(&req.session_id) {
            if executions.len() > 1000 {
                let drain_count = executions.len() - 1000;
                executions.drain(0..drain_count);
            }
        }

        Ok(MarkExecutedResponse {
            success: true,
            error_message: String::new(),
        })
    }

    /// Check if a plan was already executed
    pub fn check_executed(&self, req: CheckExecutedRequest) -> Result<CheckExecutedResponse> {
        debug!(
            session_id = %req.session_id,
            plan_hash = %req.plan_hash,
            "Checking if plan was executed"
        );

        let execution = self
            .executed_plans
            .get(&req.session_id)
            .and_then(|executions| {
                executions
                    .iter()
                    .rev()
                    .find(|e| e.plan_hash == req.plan_hash)
                    .cloned()
            });

        Ok(CheckExecutedResponse {
            is_executed: execution.is_some(),
            execution,
        })
    }

    /// Get execution statistics
    pub fn get_execution_stats(
        &self,
        req: GetExecutionStatsRequest,
    ) -> Result<GetExecutionStatsResponse> {
        debug!(
            session_id = %req.session_id,
            symbol = %req.symbol,
            "Retrieving execution statistics"
        );

        let executions = self
            .executed_plans
            .get(&req.session_id)
            .map(|entry| {
                entry
                    .iter()
                    .filter(|e| e.symbol == req.symbol)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let total = executions.len() as i32;
        let successful = executions.iter().filter(|e| e.status == "success").count() as i32;
        let failed = executions.iter().filter(|e| e.status == "error").count() as i32;
        let win_rate = if total > 0 {
            successful as f32 / total as f32
        } else {
            0.0
        };

        // Count duplicate prevented (status == "skipped")
        let duplicate_prevented =
            executions.iter().filter(|e| e.status == "skipped").count() as i32;

        // Get recent executions (last 10)
        let recent_executions = executions.iter().rev().take(10).cloned().collect();

        Ok(GetExecutionStatsResponse {
            total_executions: total,
            successful_executions: successful,
            failed_executions: failed,
            win_rate,
            duplicate_prevented,
            recent_executions,
        })
    }

    // Legacy event storage (for backward compatibility)
    fn summarize_event(event: &crate::proto::Event) -> String {
        format!(
            "[{ts}] {ty} from {src}",
            ts = event.timestamp_ms,
            ty = event.r#type,
            src = event.source
        )
    }
}

// Implement legacy MemoryWriter trait for backward compatibility
#[async_trait]
impl MemoryWriter for AgentMemoryStore {
    async fn append_event(&self, session: &str, event: crate::proto::Event) -> Result<()> {
        let line = Self::summarize_event(&event);
        self.events
            .entry(session.to_string())
            .or_default()
            .push(line);
        Ok(())
    }

    async fn summarize_episode(&self, session: &str) -> Result<Option<String>> {
        if let Some(list) = self.events.get(session) {
            let tail = list.iter().rev().take(10).cloned().collect::<Vec<_>>();
            let summary = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }
}

// Implement legacy MemoryReader trait for backward compatibility
#[async_trait]
impl MemoryReader for AgentMemoryStore {
    async fn retrieve(
        &self,
        query: &str,
        k: usize,
        _filters: Option<serde_json::Value>,
    ) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for entry in self.events.iter() {
            for line in entry.iter() {
                if line.contains(query) {
                    out.push(line.clone());
                    if out.len() >= k {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_retrieve_plans() {
        let store = AgentMemoryStore::new();

        let plan = PlanRecord {
            timestamp_ms: 1000,
            symbol: "BTC".to_string(),
            action: "BUY".to_string(),
            confidence: 0.8,
            reasoning: "Bullish trend".to_string(),
            plan_hash: "abc123".to_string(),
            method: "llm".to_string(),
            metadata: Default::default(),
        };

        // Save plan
        let save_resp = store
            .save_plan(SavePlanRequest {
                session_id: "test-session".to_string(),
                plan: Some(plan.clone()),
            })
            .unwrap();

        assert!(save_resp.success);
        assert_eq!(save_resp.plan_hash, "abc123");

        // Retrieve plans
        let get_resp = store
            .get_recent_plans(GetRecentPlansRequest {
                session_id: "test-session".to_string(),
                symbol: "BTC".to_string(),
                limit: 10,
            })
            .unwrap();

        assert_eq!(get_resp.plans.len(), 1);
        assert_eq!(get_resp.plans[0].action, "BUY");
    }

    #[test]
    fn test_duplicate_detection() {
        let store = AgentMemoryStore::new();

        let plan1 = PlanRecord {
            timestamp_ms: 1000,
            symbol: "BTC".to_string(),
            action: "BUY".to_string(),
            confidence: 0.8,
            reasoning: "Bullish".to_string(),
            plan_hash: "abc123".to_string(),
            method: "llm".to_string(),
            metadata: Default::default(),
        };

        let plan2 = PlanRecord {
            timestamp_ms: 2000, // 1 second later
            symbol: "BTC".to_string(),
            action: "BUY".to_string(),
            confidence: 0.8,
            reasoning: "Bullish".to_string(),
            plan_hash: "abc123".to_string(), // Same hash
            method: "llm".to_string(),
            metadata: Default::default(),
        };

        // Save first plan
        store
            .save_plan(SavePlanRequest {
                session_id: "test".to_string(),
                plan: Some(plan1),
            })
            .unwrap();

        // Check duplicate (within 300 second window)
        let dup_resp = store
            .check_duplicate(CheckDuplicateRequest {
                session_id: "test".to_string(),
                plan: Some(plan2),
                time_window_sec: 300,
            })
            .unwrap();

        assert!(dup_resp.is_duplicate);
        assert_eq!(dup_resp.time_since_duplicate_ms, 1000);
    }

    #[test]
    fn test_execution_idempotency() {
        let store = AgentMemoryStore::new();

        let execution = ExecutionRecord {
            timestamp_ms: 1000,
            plan_hash: "abc123".to_string(),
            symbol: "BTC".to_string(),
            action: "BUY".to_string(),
            confidence: 0.8,
            status: "success".to_string(),
            executed: true,
            order_id: "order-123".to_string(),
            order_size_usdt: 100.0,
            error_message: String::new(),
        };

        // Mark as executed
        store
            .mark_executed(MarkExecutedRequest {
                session_id: "test".to_string(),
                plan_hash: "abc123".to_string(),
                execution: Some(execution),
            })
            .unwrap();

        // Check if executed
        let check_resp = store
            .check_executed(CheckExecutedRequest {
                session_id: "test".to_string(),
                plan_hash: "abc123".to_string(),
            })
            .unwrap();

        assert!(check_resp.is_executed);
        assert_eq!(check_resp.execution.unwrap().order_id, "order-123");
    }
}
