use loom_core::context::agent_memory::AgentMemoryStore;
use loom_proto::{
    memory_service_server::MemoryService, CheckDuplicateRequest, CheckDuplicateResponse,
    CheckExecutedRequest, CheckExecutedResponse, GetExecutionStatsRequest,
    GetExecutionStatsResponse, GetRecentPlansRequest, GetRecentPlansResponse, MarkExecutedRequest,
    MarkExecutedResponse, MemoryRetrieveRequest, MemoryRetrieveResponse, MemorySummarizeRequest,
    MemorySummarizeResponse, MemoryWriteRequest, MemoryWriteResponse, SavePlanRequest,
    SavePlanResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::debug;

/// Memory handler service exposed via gRPC
#[derive(Clone)]
pub struct MemoryHandler {
    store: Arc<AgentMemoryStore>,
}

impl MemoryHandler {
    pub fn new(store: Arc<AgentMemoryStore>) -> Self {
        Self { store }
    }
}

#[tonic::async_trait]
impl MemoryService for MemoryHandler {
    async fn save_plan(
        &self,
        request: Request<SavePlanRequest>,
    ) -> Result<Response<SavePlanResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            symbol = %req.plan.as_ref().map(|p| p.symbol.as_str()).unwrap_or("unknown"),
            "Saving plan via Bridge"
        );

        match self.store.save_plan(req) {
            Ok(resp) => Ok(Response::new(resp)),
            Err(e) => Err(Status::internal(format!("Failed to save plan: {}", e))),
        }
    }

    async fn get_recent_plans(
        &self,
        request: Request<GetRecentPlansRequest>,
    ) -> Result<Response<GetRecentPlansResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            symbol = %req.symbol,
            limit = req.limit,
            "Retrieving recent plans via Bridge"
        );

        match self.store.get_recent_plans(req) {
            Ok(resp) => Ok(Response::new(resp)),
            Err(e) => Err(Status::internal(format!(
                "Failed to get recent plans: {}",
                e
            ))),
        }
    }

    async fn check_duplicate(
        &self,
        request: Request<CheckDuplicateRequest>,
    ) -> Result<Response<CheckDuplicateResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            "Checking duplicate plan via Bridge"
        );

        match self.store.check_duplicate(req) {
            Ok(resp) => Ok(Response::new(resp)),
            Err(e) => Err(Status::internal(format!(
                "Failed to check duplicate: {}",
                e
            ))),
        }
    }

    async fn mark_executed(
        &self,
        request: Request<MarkExecutedRequest>,
    ) -> Result<Response<MarkExecutedResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            plan_hash = %req.plan_hash,
            "Marking plan as executed via Bridge"
        );

        match self.store.mark_executed(req) {
            Ok(resp) => Ok(Response::new(resp)),
            Err(e) => Err(Status::internal(format!("Failed to mark executed: {}", e))),
        }
    }

    async fn check_executed(
        &self,
        request: Request<CheckExecutedRequest>,
    ) -> Result<Response<CheckExecutedResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            plan_hash = %req.plan_hash,
            "Checking if plan was executed via Bridge"
        );

        match self.store.check_executed(req) {
            Ok(resp) => Ok(Response::new(resp)),
            Err(e) => Err(Status::internal(format!("Failed to check executed: {}", e))),
        }
    }

    async fn get_execution_stats(
        &self,
        request: Request<GetExecutionStatsRequest>,
    ) -> Result<Response<GetExecutionStatsResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            symbol = %req.symbol,
            "Retrieving execution stats via Bridge"
        );

        match self.store.get_execution_stats(req) {
            Ok(resp) => Ok(Response::new(resp)),
            Err(e) => Err(Status::internal(format!(
                "Failed to get execution stats: {}",
                e
            ))),
        }
    }

    async fn append_event(
        &self,
        request: Request<MemoryWriteRequest>,
    ) -> Result<Response<MemoryWriteResponse>, Status> {
        let req = request.into_inner();
        debug!(session_id = %req.session_id, "Appending event via Bridge");

        if let Some(event) = req.event {
            use loom_core::context::MemoryWriter;
            match self.store.append_event(&req.session_id, event).await {
                Ok(_) => Ok(Response::new(MemoryWriteResponse {
                    success: true,
                    error_message: String::new(),
                })),
                Err(e) => Err(Status::internal(format!("Failed to append event: {}", e))),
            }
        } else {
            Err(Status::invalid_argument("Event is required"))
        }
    }

    async fn retrieve(
        &self,
        request: Request<MemoryRetrieveRequest>,
    ) -> Result<Response<MemoryRetrieveResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            query = %req.query,
            k = req.k,
            "Retrieving from memory via Bridge"
        );

        use loom_core::context::MemoryReader;
        match self.store.retrieve(&req.query, req.k as usize, None).await {
            Ok(results) => Ok(Response::new(MemoryRetrieveResponse {
                results,
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Err(Status::internal(format!("Failed to retrieve: {}", e))),
        }
    }

    async fn summarize_episode(
        &self,
        request: Request<MemorySummarizeRequest>,
    ) -> Result<Response<MemorySummarizeResponse>, Status> {
        let req = request.into_inner();
        debug!(
            session_id = %req.session_id,
            "Summarizing episode via Bridge"
        );

        use loom_core::context::MemoryWriter;
        match self.store.summarize_episode(&req.session_id).await {
            Ok(summary) => Ok(Response::new(MemorySummarizeResponse {
                summary: summary.unwrap_or_default(),
                event_count: req.max_events,
                success: true,
                error_message: String::new(),
            })),
            Err(e) => Err(Status::internal(format!(
                "Failed to summarize episode: {}",
                e
            ))),
        }
    }
}
