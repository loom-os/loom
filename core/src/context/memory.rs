use super::{MemoryReader, MemoryWriter};
use crate::proto::{
    CheckDuplicateRequest, CheckDuplicateResponse, CheckExecutedRequest, CheckExecutedResponse,
    Event, ExecutionRecord, GetExecutionStatsRequest, GetExecutionStatsResponse,
    GetRecentPlansRequest, GetRecentPlansResponse, MarkExecutedRequest, MarkExecutedResponse,
    PlanRecord, SavePlanRequest, SavePlanResponse,
};
use crate::{LoomError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, warn};

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
}

#[async_trait]
impl MemoryWriter for InMemoryMemory {
    async fn append_event(&self, session: &str, event: Event) -> Result<()> {
        let line = Self::summarize_event(&event);
        self.store
            .entry(session.to_string())
            .or_default()
            .value_mut()
            .push(line);
        Ok(())
    }

    async fn summarize_episode(&self, session: &str) -> Result<Option<String>> {
        if let Some(list) = self.store.get(session) {
            let tail = list.iter().rev().take(10).cloned().collect::<Vec<_>>();
            let summary = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl MemoryReader for InMemoryMemory {
    async fn retrieve(
        &self,
        query: &str,
        k: usize,
        _filters: Option<serde_json::Value>,
    ) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for entry in self.store.iter() {
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
