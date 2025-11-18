/// Integration tests for Memory gRPC service via Bridge
///
/// Tests the full stack: Bridge MemoryHandler → Core InMemoryMemory
use loom_bridge::memory_handler::MemoryHandler;
use loom_core::context::memory::InMemoryMemory;
use loom_core::proto::{
    memory_service_client::MemoryServiceClient, memory_service_server::MemoryServiceServer,
    CheckDuplicateRequest, CheckExecutedRequest, ExecutionRecord, GetExecutionStatsRequest,
    GetRecentPlansRequest, MarkExecutedRequest, PlanRecord, SavePlanRequest,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tonic::Request;

/// Helper to start a test memory service
async fn start_test_service() -> (
    Arc<InMemoryMemory>,
    tokio::task::JoinHandle<Result<(), tonic::transport::Error>>,
    String,
) {
    let memory_store = Arc::new(InMemoryMemory::new());
    let memory_handler = MemoryHandler::new(Arc::clone(&memory_store));

    // Use a unique port per test based on PID and current time
    let port = 50052
        + ((std::process::id()
            + std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u32)
            % 1000) as u16;
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();

    let server = tonic::transport::Server::builder()
        .timeout(Duration::from_secs(30)) // Increase server timeout
        .add_service(MemoryServiceServer::new(memory_handler))
        .serve(addr);

    let server_handle = tokio::spawn(server);

    // Give server time to start
    sleep(Duration::from_millis(300)).await;

    let endpoint = format!("http://127.0.0.1:{}", port);

    (Arc::clone(&memory_store), server_handle, endpoint)
}

#[tokio::test]
async fn test_memory_service_save_and_retrieve() {
    let (_memory_store, server_handle, endpoint) = start_test_service().await;

    let channel = tonic::transport::Channel::from_shared(endpoint)
        .expect("Invalid endpoint")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = MemoryServiceClient::new(channel);

    // Test: Save a plan
    let plan = PlanRecord {
        timestamp_ms: 1000,
        symbol: "BTC".to_string(),
        action: "BUY".to_string(),
        confidence: 0.85,
        reasoning: "Strong bullish trend".to_string(),
        plan_hash: "test-hash-123".to_string(),
        method: "llm".to_string(),
        metadata: Default::default(),
    };

    let save_req = Request::new(SavePlanRequest {
        session_id: "integration-test-session".to_string(),
        plan: Some(plan.clone()),
    });

    let save_resp = client.save_plan(save_req).await.expect("SavePlan failed");
    let save_data = save_resp.into_inner();
    assert!(save_data.success);
    assert_eq!(save_data.plan_hash, "test-hash-123");

    // Test: Retrieve the plan
    let get_req = Request::new(GetRecentPlansRequest {
        session_id: "integration-test-session".to_string(),
        symbol: "BTC".to_string(),
        limit: 10,
    });

    let get_resp = client
        .get_recent_plans(get_req)
        .await
        .expect("GetRecentPlans failed");
    let get_data = get_resp.into_inner();

    assert_eq!(get_data.plans.len(), 1);
    assert_eq!(get_data.plans[0].action, "BUY");
    assert_eq!(get_data.plans[0].symbol, "BTC");
    assert!((get_data.plans[0].confidence - 0.85).abs() < 0.01);

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_memory_service_duplicate_detection() {
    let (_memory_store, server_handle, endpoint) = start_test_service().await;

    let channel = tonic::transport::Channel::from_shared(endpoint)
        .expect("Invalid endpoint")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = MemoryServiceClient::new(channel);

    // Save first plan
    let plan1 = PlanRecord {
        timestamp_ms: 1000,
        symbol: "BTC".to_string(),
        action: "BUY".to_string(),
        confidence: 0.8,
        reasoning: "Bullish".to_string(),
        plan_hash: "dup-test-hash".to_string(),
        method: "llm".to_string(),
        metadata: Default::default(),
    };

    client
        .save_plan(Request::new(SavePlanRequest {
            session_id: "dup-test".to_string(),
            plan: Some(plan1.clone()),
        }))
        .await
        .expect("First SavePlan failed");

    // Check duplicate with same hash (within time window)
    let plan2 = PlanRecord {
        timestamp_ms: 2000, // 1 second later
        ..plan1.clone()
    };

    let dup_req = Request::new(CheckDuplicateRequest {
        session_id: "dup-test".to_string(),
        plan: Some(plan2),
        time_window_sec: 300,
    });

    let dup_resp = client
        .check_duplicate(dup_req)
        .await
        .expect("CheckDuplicate failed");
    let dup_data = dup_resp.into_inner();

    assert!(dup_data.is_duplicate);
    assert_eq!(dup_data.time_since_duplicate_ms, 1000);

    server_handle.abort();
}

#[tokio::test]
async fn test_memory_service_execution_tracking() {
    let (_memory_store, server_handle, endpoint) = start_test_service().await;

    let channel = tonic::transport::Channel::from_shared(endpoint)
        .expect("Invalid endpoint")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = MemoryServiceClient::new(channel);

    let plan_hash = "exec-test-hash";

    // Check before execution - should not be executed
    let check_req1 = Request::new(CheckExecutedRequest {
        session_id: "exec-test".to_string(),
        plan_hash: plan_hash.to_string(),
    });

    let check_resp1 = client
        .check_executed(check_req1)
        .await
        .expect("CheckExecuted failed");
    assert!(!check_resp1.into_inner().is_executed);

    // Mark as executed
    let execution = ExecutionRecord {
        timestamp_ms: 1000,
        plan_hash: plan_hash.to_string(),
        symbol: "BTC".to_string(),
        action: "BUY".to_string(),
        confidence: 0.8,
        status: "success".to_string(),
        executed: true,
        order_id: "order-123".to_string(),
        order_size_usdt: 100.0,
        error_message: String::new(),
    };

    let mark_req = Request::new(MarkExecutedRequest {
        session_id: "exec-test".to_string(),
        plan_hash: plan_hash.to_string(),
        execution: Some(execution),
    });

    let mark_resp = client
        .mark_executed(mark_req)
        .await
        .expect("MarkExecuted failed");
    assert!(mark_resp.into_inner().success);

    // Check after execution - should be executed
    let check_req2 = Request::new(CheckExecutedRequest {
        session_id: "exec-test".to_string(),
        plan_hash: plan_hash.to_string(),
    });

    let check_resp2 = client
        .check_executed(check_req2)
        .await
        .expect("CheckExecuted failed");
    let check_data = check_resp2.into_inner();

    assert!(check_data.is_executed);
    assert_eq!(check_data.execution.unwrap().order_id, "order-123");

    server_handle.abort();
}

#[tokio::test]
async fn test_memory_service_execution_stats() {
    let (_memory_store, server_handle, endpoint) = start_test_service().await;

    let channel = tonic::transport::Channel::from_shared(endpoint)
        .expect("Invalid endpoint")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = MemoryServiceClient::new(channel);

    // Add multiple executions
    for i in 0..5 {
        let execution = ExecutionRecord {
            timestamp_ms: (i + 1) * 1000,
            plan_hash: format!("hash-{}", i),
            symbol: "BTC".to_string(),
            action: if i % 2 == 0 { "BUY" } else { "SELL" }.to_string(),
            confidence: 0.8,
            status: if i < 3 { "success" } else { "hold" }.to_string(),
            executed: i < 3,
            order_id: format!("order-{}", i),
            order_size_usdt: 100.0,
            error_message: String::new(),
        };

        client
            .mark_executed(Request::new(MarkExecutedRequest {
                session_id: "stats-test".to_string(),
                plan_hash: format!("hash-{}", i),
                execution: Some(execution),
            }))
            .await
            .expect("MarkExecuted failed");
    }

    // Get stats
    let stats_req = Request::new(GetExecutionStatsRequest {
        session_id: "stats-test".to_string(),
        symbol: "BTC".to_string(),
    });

    let stats_resp = client
        .get_execution_stats(stats_req)
        .await
        .expect("GetExecutionStats failed");
    let stats_data = stats_resp.into_inner();

    assert_eq!(stats_data.total_executions, 5);
    // 3 successful out of 5 = 60%
    assert!((stats_data.win_rate - 0.6).abs() < 0.01);
    assert_eq!(stats_data.recent_executions.len(), 5);

    server_handle.abort();
}

#[tokio::test]
async fn test_memory_service_session_isolation() {
    let (_memory_store, server_handle, endpoint) = start_test_service().await;

    let channel = tonic::transport::Channel::from_shared(endpoint)
        .expect("Invalid endpoint")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = MemoryServiceClient::new(channel);

    // Save plan in session 1
    let plan1 = PlanRecord {
        timestamp_ms: 1000,
        symbol: "BTC".to_string(),
        action: "BUY".to_string(),
        confidence: 0.8,
        reasoning: "Session 1".to_string(),
        plan_hash: "hash-s1".to_string(),
        method: "llm".to_string(),
        metadata: Default::default(),
    };

    client
        .save_plan(Request::new(SavePlanRequest {
            session_id: "session-1".to_string(),
            plan: Some(plan1),
        }))
        .await
        .expect("SavePlan session-1 failed");

    // Save plan in session 2
    let plan2 = PlanRecord {
        timestamp_ms: 2000,
        symbol: "BTC".to_string(),
        action: "SELL".to_string(),
        confidence: 0.7,
        reasoning: "Session 2".to_string(),
        plan_hash: "hash-s2".to_string(),
        method: "rule".to_string(),
        metadata: Default::default(),
    };

    client
        .save_plan(Request::new(SavePlanRequest {
            session_id: "session-2".to_string(),
            plan: Some(plan2),
        }))
        .await
        .expect("SavePlan session-2 failed");

    // Get plans from session 1 - should only see plan1
    let get_req1 = Request::new(GetRecentPlansRequest {
        session_id: "session-1".to_string(),
        symbol: "BTC".to_string(),
        limit: 10,
    });

    let get_resp1 = client
        .get_recent_plans(get_req1)
        .await
        .expect("GetRecentPlans session-1 failed");
    let plans1 = get_resp1.into_inner().plans;

    assert_eq!(plans1.len(), 1);
    assert_eq!(plans1[0].reasoning, "Session 1");

    // Get plans from session 2 - should only see plan2
    let get_req2 = Request::new(GetRecentPlansRequest {
        session_id: "session-2".to_string(),
        symbol: "BTC".to_string(),
        limit: 10,
    });

    let get_resp2 = client
        .get_recent_plans(get_req2)
        .await
        .expect("GetRecentPlans session-2 failed");
    let plans2 = get_resp2.into_inner().plans;

    assert_eq!(plans2.len(), 1);
    assert_eq!(plans2[0].reasoning, "Session 2");

    server_handle.abort();
}
