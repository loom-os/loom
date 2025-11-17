## Memory & Context System

**Status**: Implemented — this document describes Loom's memory system with both general-purpose episodic memory and specialized agent decision tracking.

---

### Overview

The memory system provides two complementary capabilities:

1. **General-purpose memory** via `MemoryReader/Writer` traits for episodic summaries and retrieval
2. **Agent decision memory** for plan storage, deduplication, execution tracking, and statistics

Both are implemented in `InMemoryMemory` (Rust core) and exposed via gRPC Bridge to Python SDK and external agents.

---

### Architecture

#### Core Traits (General-purpose)

```rust
#[async_trait::async_trait]
pub trait MemoryWriter: Send + Sync {
    async fn append_event(&self, session: &str, event: Event) -> Result<()>;
    async fn summarize_episode(&self, session: &str) -> Result<Option<String>>;
}

#[async_trait::async_trait]
pub trait MemoryReader: Send + Sync {
    async fn retrieve(
        &self,
        query: &str,
        k: usize,
        filters: Option<serde_json::Value>,
    ) -> Result<Vec<String>>;
}
```

These traits support:

- **Episodic memory**: Event sequences and summaries scoped to sessions
- **Simple retrieval**: Substring-based search (upgradable to semantic search)

#### Agent Decision Methods (Extended API)

```rust
impl InMemoryMemory {
    // Plan storage with deduplication
    async fn save_plan(
        &self,
        session_id: &str,
        symbol: &str,
        action: &str,
        confidence: f32,
        reasoning: &str,
        method: &str,
    ) -> Result<String> // Returns plan_hash

    // Query recent decisions
    async fn get_recent_plans(
        &self,
        session_id: &str,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<PlanRecord>>

    // Duplicate detection
    async fn check_duplicate(
        &self,
        session_id: &str,
        symbol: &str,
        action: &str,
        reasoning: &str,
        time_window_sec: u64,
    ) -> Result<(bool, Option<PlanRecord>)>

    // Execution tracking (idempotency)
    async fn mark_executed(
        &self,
        session_id: &str,
        plan_hash: &str,
        // ... execution details
    ) -> Result<()>

    async fn check_executed(
        &self,
        session_id: &str,
        plan_hash: &str,
    ) -> Result<(bool, Option<ExecutionRecord>)>

    // Statistics
    async fn get_execution_stats(
        &self,
        session_id: &str,
        symbol: &str,
    ) -> Result<ExecutionStats>
}
```

These methods support:

- **Decision deduplication**: Prevent duplicate plans within time windows
- **Execution idempotency**: Ensure plans execute at most once
- **Performance tracking**: Win rates, success counts, recent executions

---

### Implementation: InMemoryMemory

#### Data Structures

```rust
pub struct InMemoryMemory {
    // General-purpose episodic memory
    events: Arc<DashMap<String, Vec<String>>>,        // session_id -> events
    summaries: Arc<DashMap<String, String>>,          // session_id -> summary

    // Agent decision memory
    plans: Arc<DashMap<String, Vec<PlanRecord>>>,     // session_id -> plans (max 100)
    executed_plans: Arc<DashMap<String, ExecutionRecord>>, // plan_hash -> execution
}
```

**Key characteristics**:

- **DashMap**: Lock-free concurrent hashmap for high-performance multi-agent access
- **Session isolation**: All data partitioned by session_id
- **Plan limit**: Keep last 100 plans per session (FIFO eviction)
- **Hash-based lookups**: O(1) execution tracking via plan_hash

#### Deduplication Algorithm

```rust
// 1. Generate deterministic hash
let content = format!("{}|{}|{}", symbol, action, reasoning);
let hash = md5::compute(content.as_bytes());
let plan_hash = format!("{:x}", hash)[..8]; // 8-char prefix

// 2. Check time window (default: 5 minutes)
for existing_plan in plans {
    if existing_plan.plan_hash == plan_hash {
        let time_diff = current_time - existing_plan.timestamp_ms;
        if time_diff < time_window_ms {
            return (true, Some(existing_plan)); // Duplicate!
        }
    }
}

// 3. Save if unique
plans.push(new_plan);
```

**Why MD5**:

- Fast (not cryptographic security-critical)
- Deterministic (same plan → same hash)
- Collision-resistant enough for 8-char prefix in trading contexts

#### Execution Idempotency

```rust
// Before execution
let (already_executed, exec_info) = memory.check_executed(session, &plan_hash).await?;

if already_executed {
    log::warn!("Plan {} already executed: {:?}", plan_hash, exec_info);
    return Ok(()); // Skip
}

// Execute order...
let order_result = exchange.place_order(...).await?;

// Mark as executed (atomic)
memory.mark_executed(
    session, &plan_hash, symbol, action, confidence,
    "success", true, &order_id, order_size
).await?;
```

**Guarantees**:

- Single execution per plan_hash
- Crash-safe with persistent memory backends (future: RocksDB)
- Observable via execution stats

---

### gRPC Bridge Integration

#### Protobuf Service

```protobuf
service MemoryService {
  // Agent decision memory
  rpc SavePlan(SavePlanRequest) returns (SavePlanResponse);
  rpc GetRecentPlans(GetRecentPlansRequest) returns (GetRecentPlansResponse);
  rpc CheckDuplicate(CheckDuplicateRequest) returns (CheckDuplicateResponse);
  rpc MarkExecuted(MarkExecutedRequest) returns (MarkExecutedResponse);
  rpc CheckExecuted(CheckExecutedRequest) returns (CheckExecutedResponse);
  rpc GetExecutionStats(GetExecutionStatsRequest) returns (GetExecutionStatsResponse);

  // Legacy general-purpose memory
  rpc Store(StoreRequest) returns (StoreResponse);
  rpc Retrieve(RetrieveRequest) returns (RetrieveResponse);
  rpc Summarize(SummarizeRequest) returns (SummarizeResponse);
}
```

#### Bridge Handler

```rust
pub struct MemoryHandler {
    memory: Arc<InMemoryMemory>,
}

#[tonic::async_trait]
impl MemoryService for MemoryHandler {
    async fn save_plan(&self, req: Request<SavePlanRequest>)
        -> Result<Response<SavePlanResponse>, Status> {
        // Delegate to InMemoryMemory
        let hash = self.memory.save_plan(...).await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(SavePlanResponse {
            success: true,
            plan_hash: hash,
            error_message: String::new(),
        }))
    }

    // ... other RPCs
}
```

**Architecture benefits**:

- Single source of truth (Rust Core)
- Type-safe protobuf contracts
- Cross-language support (Python, future: JS/Go)
- Observable via gRPC interceptors/tracing

---

### Python SDK Usage

```python
from loom import Context

# Initialize with Bridge client
ctx = Context(agent_id="planner-btc", client=bridge_client)

# Check for duplicate plans
is_dup, dup_info = await ctx.check_duplicate_plan(
    symbol="BTC",
    action="BUY",
    reasoning="Strong bullish trend detected",
    time_window_sec=300  # 5 minutes
)

if is_dup:
    print(f"Duplicate plan from {dup_info['time_since_ms']}ms ago")
    return

# Save new plan
plan_hash = await ctx.save_plan(
    symbol="BTC",
    action="BUY",
    confidence=0.85,
    reasoning="Strong bullish trend detected",
    method="llm"
)

# Query recent plans for context
recent_plans = await ctx.get_recent_plans(symbol="BTC", limit=10)
for plan in recent_plans:
    print(f"{plan['action']} @ {plan['confidence']}: {plan['reasoning']}")

# Check execution status
is_executed, exec_info = await ctx.check_plan_executed(plan_hash)

if not is_executed:
    # Execute and track
    await ctx.mark_plan_executed(
        plan_hash=plan_hash,
        symbol="BTC",
        action="BUY",
        confidence=0.85,
        status="success",
        executed=True,
        order_id="order-123",
        order_size_usdt=100.0
    )

# Get statistics
stats = await ctx.get_execution_stats("BTC")
print(f"Win rate: {stats['win_rate']:.1%}")
print(f"Total executions: {stats['total_executions']}")
```

---

### Use Case: Market Analyst Demo

#### Planner Agent

```python
# 1. Query recent decisions for context
recent_plans = await ctx.get_recent_plans(symbol="BTC", limit=5)
context_lines = [
    f"- {p['timestamp_ms']}: {p['action']} @ {p['confidence']:.2f} ({p['reasoning']})"
    for p in recent_plans
]

# 2. Get LLM decision
prompt = f"""
Recent BTC decisions:
{chr(10).join(context_lines)}

Current market: {market_data}
Decide: BUY/SELL/HOLD with reasoning.
"""
decision = await llm_client.complete(prompt)

# 3. Check for duplicate
is_dup, _ = await ctx.check_duplicate_plan(
    symbol="BTC",
    action=decision.action,
    reasoning=decision.reasoning,
    time_window_sec=300
)

if is_dup:
    logger.info("Duplicate plan, skipping")
    return

# 4. Save plan
plan_hash = await ctx.save_plan(
    symbol="BTC",
    action=decision.action,
    confidence=decision.confidence,
    reasoning=decision.reasoning,
    method="llm"
)

# 5. Send to Executor
await ctx.emit_event(topic="executor.plan", data={
    "plan_hash": plan_hash,
    "symbol": "BTC",
    "action": decision.action,
    "confidence": decision.confidence
})
```

#### Executor Agent

```python
# 1. Receive plan from Planner
plan = event.data

# 2. Check idempotency
is_executed, exec_info = await ctx.check_plan_executed(plan["plan_hash"])

if is_executed:
    logger.warning(f"Plan already executed: {exec_info}")
    return

# 3. Execute order
try:
    order = await exchange.place_order(
        symbol=plan["symbol"],
        side=plan["action"],
        size=calculate_size(plan["confidence"])
    )

    # 4. Track execution
    await ctx.mark_plan_executed(
        plan_hash=plan["plan_hash"],
        symbol=plan["symbol"],
        action=plan["action"],
        confidence=plan["confidence"],
        status="success",
        executed=True,
        order_id=order.id,
        order_size_usdt=order.size
    )

except Exception as e:
    # 5. Track failure
    await ctx.mark_plan_executed(
        plan_hash=plan["plan_hash"],
        symbol=plan["symbol"],
        action=plan["action"],
        confidence=plan["confidence"],
        status="error",
        executed=False,
        order_id="",
        order_size_usdt=0.0
    )

# 6. Query stats periodically
stats = await ctx.get_execution_stats("BTC")
logger.info(f"BTC win rate: {stats['win_rate']:.1%} ({stats['successful_executions']}/{stats['total_executions']})")
```

**Benefits**:

- **No duplicate orders**: Planner prevents duplicate decisions
- **Idempotent execution**: Executor prevents double-execution (e.g., on retry)
- **Observable performance**: Real-time win rate tracking
- **Context awareness**: Recent decisions inform future planning

---

### Testing

#### Core Unit Tests (`core/tests/memory_test.rs`)

- ✅ `test_save_and_retrieve_plans`: Basic plan storage and retrieval
- ✅ `test_duplicate_detection`: Hash-based deduplication within time windows
- ✅ `test_execution_idempotency`: Prevent double-execution
- ✅ `test_plan_limit_enforcement`: FIFO eviction at 100 plans
- ✅ `test_execution_stats`: Win rate calculation
- ✅ `test_cross_session_isolation`: Session-scoped data
- ✅ `test_duplicate_outside_time_window`: Time-based duplicate expiry
- ✅ `test_symbol_filtering`: Symbol-specific queries

**Run**: `cargo test -p loom-core --test memory_test`

#### Bridge Integration Tests (`bridge/tests/memory_service_test.rs`)

- ✅ `test_memory_service_save_and_retrieve`: gRPC save/get round-trip
- ✅ `test_memory_service_duplicate_detection`: gRPC duplicate checking
- ✅ `test_memory_service_execution_tracking`: gRPC execution idempotency
- ✅ `test_memory_service_execution_stats`: gRPC statistics
- ✅ `test_memory_service_session_isolation`: Session isolation via gRPC

**Run**: `cargo test -p loom-bridge --test memory_service_test -- --test-threads=1`

#### Python SDK Tests (`loom-py/tests/test_memory.py`)

- ✅ `test_plan_hash_consistency`: Hash generation
- ✅ `test_save_plan_success/failure`: Plan saving with mocked gRPC
- ✅ `test_get_recent_plans`: Plan retrieval
- ✅ `test_check_duplicate_plan_*`: Duplicate detection
- ✅ `test_mark_plan_executed`: Execution tracking
- ✅ `test_check_plan_executed_*`: Execution status queries
- ✅ `test_get_execution_stats`: Statistics retrieval
- ✅ `test_rpc_error_handling`: Error propagation

**Run**: `cd loom-py && pytest tests/test_memory.py -v`

**Total**: 25 tests (8 Core + 5 Bridge + 12 Python) — all passing ✅

---

### Future Enhancements

#### 1. Persistent Memory Backends

Replace `InMemoryMemory` with durable storage:

```rust
pub struct RocksDBMemory {
    db: Arc<rocksdb::DB>,
    // ... column families for plans, executions, episodes
}

#[async_trait]
impl MemoryReader for RocksDBMemory { ... }

#[async_trait]
impl MemoryWriter for RocksDBMemory { ... }
```

**Benefits**:

- Crash recovery
- Historical analysis
- Larger plan histories (>100 per session)

#### 2. Semantic Memory (Vector Search)

Add embedding-based retrieval:

```rust
#[async_trait]
pub trait SemanticMemory: Send + Sync {
    async fn embed_and_store(&self, session: &str, text: &str) -> Result<()>;
    async fn semantic_search(&self, query: &str, k: usize) -> Result<Vec<String>>;
}

// Adapters: FAISS, Milvus, Qdrant, etc.
```

**Use cases**:

- Find similar past decisions by reasoning (not just symbol/action)
- Cross-symbol pattern matching
- Knowledge base retrieval

#### 3. Distributed Memory Service

Run memory as a separate service:

```
┌─────────┐      ┌──────────────┐      ┌──────────────┐
│ Agent   │─gRPC─│ Memory       │─────→│ Persistent   │
│ (Python)│      │ Service      │      │ Backend      │
└─────────┘      │ (Rust Core)  │      │ (RocksDB)    │
                 └──────────────┘      └──────────────┘
```

**Benefits**:

- Shared memory across agent processes
- Independent scaling
- Centralized observability

#### 4. Memory Events

Standardize memory operations as events:

- `memory.{session_id}.plan.saved`
- `memory.{session_id}.execution.recorded`
- `memory.{session_id}.stats.updated`

**Benefits**:

- Dashboard visibility
- Audit logging
- Event-driven memory invalidation/refresh
