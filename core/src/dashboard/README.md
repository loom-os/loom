# Dashboard MVP

简单的实时事件流可视化界面，用于查看 Loom 系统中的事件流动。

## 快速开始

### 1. 启动 Dashboard 演示

```bash
cd core
export LOOM_DASHBOARD_PORT=3030
cargo run --example dashboard_demo
```

### 2. 打开浏览器

```bash
open http://localhost:3030
```

你将看到：

- **实时事件流**：所有发布到 EventBus 的事件
- **Agent 拓扑**：已注册的 Agent 列表
- **关键指标**：事件速率、活跃 Agent 数量

## 功能特性

### ✅ 已实现

- **实时事件流 (SSE)**

  - 按时间顺序显示事件
  - 显示：timestamp, event_id, topic, sender, thread_id, correlation_id, payload
  - 按 thread_id/topic/sender 过滤
  - 暂停/恢复自动滚动
  - 保留最近 100 个事件

- **Agent 拓扑**

  - 显示已注册的 Agent 列表
  - 显示订阅的 topics
  - 自动刷新（每 5 秒）

- **关键指标**

  - Events/sec
  - Active Agents

- **零依赖前端**
  - 纯 HTML/CSS/JS（无构建步骤）
  - 响应式设计
  - 暗色主题

### 🚧 待实现

- **高级可视化**

  - D3.js 拓扑图（力导向图）
  - Thread timeline (Gantt chart)
  - Event 关联关系可视化

- **更多指标**

  - Tool invocations/sec
  - P99 latency
  - 从 Prometheus 读取实时指标

- **交互功能**
  - 点击事件查看详情
  - 事件搜索
  - 导出事件日志为 JSON

## API 端点

### `GET /`

返回 Dashboard HTML 页面

### `GET /api/events/stream`

**Server-Sent Events (SSE)** 端点，推送实时事件

响应格式：

```json
{
  "timestamp": "2025-11-12T10:30:00Z",
  "event_type": "event_published",
  "event_id": "event-123",
  "topic": "agent.task",
  "sender": "planner",
  "thread_id": "thread-456",
  "correlation_id": "corr-789",
  "payload_preview": "Task 1 payload..."
}
```

### `GET /api/topology`

返回当前 Agent 拓扑快照

响应格式：

```json
{
  "agents": [
    {
      "id": "planner",
      "topics": ["agent.task"],
      "capabilities": ["plan.create"]
    }
  ],
  "edges": [
    {
      "from_topic": "agent.task",
      "to_agent": "planner",
      "event_count": 0
    }
  ],
  "timestamp": "2025-11-12T10:30:00Z"
}
```

### `GET /api/metrics`

返回关键指标快照

响应格式：

```json
{
  "events_per_sec": 0,
  "active_agents": 3,
  "active_subscriptions": 0,
  "tool_invocations_per_sec": 0
}
```

## 环境变量

| 变量                  | 默认值      | 说明                |
| --------------------- | ----------- | ------------------- |
| `LOOM_DASHBOARD`      | `false`     | 是否启用 Dashboard  |
| `LOOM_DASHBOARD_PORT` | `3030`      | Dashboard HTTP 端口 |
| `LOOM_DASHBOARD_HOST` | `127.0.0.1` | Dashboard 绑定地址  |

## 集成到应用

```rust
use loom_core::{
    dashboard::{DashboardConfig, DashboardServer, EventBroadcaster},
    event::EventBus,
    directory::AgentDirectory,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create core components
    let mut event_bus = EventBus::new().await?;
    let agent_directory = Arc::new(AgentDirectory::new());

    // Enable Dashboard
    let broadcaster = EventBroadcaster::new(1000);
    event_bus.set_dashboard_broadcaster(broadcaster.clone());

    // Start Dashboard server
    let config = DashboardConfig::from_env();
    let dashboard = DashboardServer::new(config, broadcaster, agent_directory);

    tokio::spawn(async move {
        dashboard.serve().await.unwrap();
    });

    // ... your application code ...

    Ok(())
}
```

## 架构

```
┌─────────────┐
│  EventBus   │
│             │
│  publish()  ├──────┐
└─────────────┘      │
                     │ broadcast
                     ▼
             ┌───────────────────┐
             │ EventBroadcaster  │
             │  (tokio channel)  │
             └────────┬──────────┘
                      │
                      │ SSE
                      ▼
              ┌───────────────┐
              │ DashboardServer│
              │   (Axum)       │
              └────────┬───────┘
                       │
                       │ HTTP
                       ▼
                  ┌─────────┐
                  │ Browser │
                  │   UI    │
                  └─────────┘
```

## 性能

- **事件缓冲**: 1000 个事件（可配置）
- **前端限制**: 显示最近 100 个事件
- **更新频率**:
  - 事件流: 实时（SSE 推送）
  - 拓扑: 每 5 秒
  - 指标: 每 1 秒

## 下一步

- [ ] 完成 ROADMAP 更新
- [ ] 测试与 trio.py 集成
- [ ] 添加 D3.js 拓扑可视化
- [ ] 集成 Prometheus metrics
- [ ] 添加 Thread timeline 视图
