/// Unit tests for InMemoryMemory trading plan and execution tracking
use loom_core::context::memory::InMemoryMemory;
use loom_core::proto::{
    CheckDuplicateRequest, CheckExecutedRequest, ExecutionRecord, GetExecutionStatsRequest,
    GetRecentPlansRequest, MarkExecutedRequest, PlanRecord, SavePlanRequest,
};

#[test]
fn test_save_and_retrieve_plans() {
    let store = InMemoryMemory::new();

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
    let store = InMemoryMemory::new();

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
    let store = InMemoryMemory::new();

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

#[test]
fn test_plan_limit_enforcement() {
    let store = InMemoryMemory::new();

    // Add 105 plans (exceeds limit of 100)
    for i in 0..105 {
        let plan = PlanRecord {
            timestamp_ms: i as i64 * 1000,
            symbol: "BTC".to_string(),
            action: "BUY".to_string(),
            confidence: 0.8,
            reasoning: format!("Plan {}", i),
            plan_hash: format!("hash-{}", i),
            method: "llm".to_string(),
            metadata: Default::default(),
        };

        store
            .save_plan(SavePlanRequest {
                session_id: "test".to_string(),
                plan: Some(plan),
            })
            .unwrap();
    }

    // Should only keep most recent 100 plans
    let resp = store
        .get_recent_plans(GetRecentPlansRequest {
            session_id: "test".to_string(),
            symbol: "BTC".to_string(),
            limit: 200, // Request more than available
        })
        .unwrap();

    assert_eq!(resp.plans.len(), 100);
    // get_recent_plans returns oldest first after filtering/limiting
    // After draining first 5 plans, oldest remaining is "Plan 5"
    assert_eq!(resp.plans[0].reasoning, "Plan 5");
    // Most recent plan should be the last one added
    assert_eq!(resp.plans[99].reasoning, "Plan 104");
}

#[test]
fn test_execution_stats() {
    let store = InMemoryMemory::new();

    // Add multiple executions with different outcomes
    let executions = vec![
        ("hash1", "BUY", "success", true),
        ("hash2", "SELL", "success", true),
        ("hash3", "BUY", "hold", false),
        ("hash4", "BUY", "success", true),
        ("hash5", "HOLD", "hold", false),
    ];

    for (i, (hash, action, status, executed)) in executions.iter().enumerate() {
        let exec = ExecutionRecord {
            timestamp_ms: (i as i64 + 1) * 1000,
            plan_hash: hash.to_string(),
            symbol: "BTC".to_string(),
            action: action.to_string(),
            confidence: 0.8,
            status: status.to_string(),
            executed: *executed,
            order_id: format!("order-{}", i),
            order_size_usdt: 100.0,
            error_message: String::new(),
        };

        store
            .mark_executed(MarkExecutedRequest {
                session_id: "test".to_string(),
                plan_hash: hash.to_string(),
                execution: Some(exec),
            })
            .unwrap();
    }

    // Get stats
    let stats_resp = store
        .get_execution_stats(GetExecutionStatsRequest {
            session_id: "test".to_string(),
            symbol: "BTC".to_string(),
        })
        .unwrap();

    assert_eq!(stats_resp.total_executions, 5);
    // 3 successful out of 5 total = 60% win rate
    assert!((stats_resp.win_rate - 0.6).abs() < 0.01);
    assert_eq!(stats_resp.recent_executions.len(), 5);
}

#[test]
fn test_cross_session_isolation() {
    let store = InMemoryMemory::new();

    let plan1 = PlanRecord {
        timestamp_ms: 1000,
        symbol: "BTC".to_string(),
        action: "BUY".to_string(),
        confidence: 0.8,
        reasoning: "Session 1 plan".to_string(),
        plan_hash: "hash1".to_string(),
        method: "llm".to_string(),
        metadata: Default::default(),
    };

    let plan2 = PlanRecord {
        timestamp_ms: 2000,
        symbol: "BTC".to_string(),
        action: "SELL".to_string(),
        confidence: 0.7,
        reasoning: "Session 2 plan".to_string(),
        plan_hash: "hash2".to_string(),
        method: "rule".to_string(),
        metadata: Default::default(),
    };

    // Save to different sessions
    store
        .save_plan(SavePlanRequest {
            session_id: "session-1".to_string(),
            plan: Some(plan1),
        })
        .unwrap();

    store
        .save_plan(SavePlanRequest {
            session_id: "session-2".to_string(),
            plan: Some(plan2),
        })
        .unwrap();

    // Retrieve from session 1 - should only see plan1
    let resp1 = store
        .get_recent_plans(GetRecentPlansRequest {
            session_id: "session-1".to_string(),
            symbol: "BTC".to_string(),
            limit: 10,
        })
        .unwrap();

    assert_eq!(resp1.plans.len(), 1);
    assert_eq!(resp1.plans[0].reasoning, "Session 1 plan");

    // Retrieve from session 2 - should only see plan2
    let resp2 = store
        .get_recent_plans(GetRecentPlansRequest {
            session_id: "session-2".to_string(),
            symbol: "BTC".to_string(),
            limit: 10,
        })
        .unwrap();

    assert_eq!(resp2.plans.len(), 1);
    assert_eq!(resp2.plans[0].reasoning, "Session 2 plan");
}

#[test]
fn test_duplicate_outside_time_window() {
    let store = InMemoryMemory::new();

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
        timestamp_ms: 302_000, // 301 seconds later (outside 300s window)
        symbol: "BTC".to_string(),
        action: "BUY".to_string(),
        confidence: 0.8,
        reasoning: "Bullish".to_string(),
        plan_hash: "abc123".to_string(),
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

    // Check duplicate with 300 second window
    let dup_resp = store
        .check_duplicate(CheckDuplicateRequest {
            session_id: "test".to_string(),
            plan: Some(plan2),
            time_window_sec: 300,
        })
        .unwrap();

    // Should NOT be considered duplicate (outside window)
    assert!(!dup_resp.is_duplicate);
}

#[test]
fn test_symbol_filtering() {
    let store = InMemoryMemory::new();

    // Add plans for different symbols
    for (i, symbol) in ["BTC", "ETH", "BTC", "SOL", "BTC"].iter().enumerate() {
        let plan = PlanRecord {
            timestamp_ms: (i as i64 + 1) * 1000,
            symbol: symbol.to_string(),
            action: "BUY".to_string(),
            confidence: 0.8,
            reasoning: format!("{} plan", symbol),
            plan_hash: format!("hash-{}", i),
            method: "llm".to_string(),
            metadata: Default::default(),
        };

        store
            .save_plan(SavePlanRequest {
                session_id: "test".to_string(),
                plan: Some(plan),
            })
            .unwrap();
    }

    // Get only BTC plans
    let btc_resp = store
        .get_recent_plans(GetRecentPlansRequest {
            session_id: "test".to_string(),
            symbol: "BTC".to_string(),
            limit: 10,
        })
        .unwrap();

    assert_eq!(btc_resp.plans.len(), 3);
    for plan in &btc_resp.plans {
        assert_eq!(plan.symbol, "BTC");
    }

    // Get only ETH plans
    let eth_resp = store
        .get_recent_plans(GetRecentPlansRequest {
            session_id: "test".to_string(),
            symbol: "ETH".to_string(),
            limit: 10,
        })
        .unwrap();

    assert_eq!(eth_resp.plans.len(), 1);
    assert_eq!(eth_resp.plans[0].symbol, "ETH");
}
