# Dashboard Fixes Summary

## 修复的问题 (最新更新)

### 问题 1: 切换页面后回来，流式响应卡住 ✅

**原因**:
- `streamingMessageRef` 是组件级的 ref，页面卸载时丢失
- Context 保存了消息，但没有保存当前流式状态
- 页面重新加载后，WebSocket 仍在接收数据，但 ref 为 null，无法更新消息

**解决方案**:
1. **扩展 ChatContext** 保存流式状态:
   - 添加 `streamingMessageId: string | null`
   - 添加 `setStreamingMessageId()` 方法
   - 在开始流式时保存 message ID
   - 在完成或错误时清除

2. **在 useChat 中恢复流式状态**:
   - 组件挂载时检查 `streamingMessageId`
   - 如果存在且消息仍在 streaming，恢复 `streamingMessageRef`
   - 继续接收和累积后续 chunks

3. **同步流式状态到 Context**:
   - 创建新消息时: `setStreamingMessageId(messageId)`
   - 完成时: `setStreamingMessageId(null)`
   - 错误时: `setStreamingMessageId(null)`

**效果**:
- ✅ 切换页面后回来，继续接收流式响应
- ✅ 消息正确累积和更新
- ✅ `isLoading` 状态正确恢复

---

### 问题 2: Observability 页面没有真实数据 ✅

**原因**:
- EventBus 的通配符订阅只支持 `"prefix.*"` 模式
- 不支持单独的 `"*"` 订阅所有 topics
- Dashboard 配置默认订阅 `"*"`，但 EventBus 忽略了

**解决方案**:
1. **修改 EventBus 支持单个 `*` 通配符** (`core/src/messaging/event_bus.rs`):
   ```rust
   // Single "*" matches everything
   if pattern == "*" {
       all_matching_subs.extend(entry.value().iter().cloned());
       continue;
   }
   ```

2. **保持配置为 `"*"`**:
   - 默认订阅所有 topics
   - 用户可通过环境变量自定义

3. **添加测试验证**:
   - `single_star_wildcard_matches_all_topics()` 测试
   - 验证 `*` 订阅接收所有 topic 的事件

**效果**:
- ✅ Dashboard 接收所有 EventBus 事件
- ✅ O11y 页面显示实时指标
- ✅ Event Flow 显示真实事件流

---

### 问题 3: Chat 状态在页面切换时丢失 ✅ (之前修复)

**原因**:
- `useChat` hook 使用 `useState` 管理聊天状态
- React 组件卸载时，所有 state 丢失
- 页面切换导致 ChatPage 组件卸载，消息历史消失

**解决方案**:
1. **创建 ChatContext** (`frontend/src/contexts/ChatContext.tsx`):
   - 使用 React Context API 提供全局状态
   - 自动持久化到 localStorage
   - 跨页面共享状态

2. **更新 useChat** 使用 Context:
   - 替换所有 `useState` 为 `useChatContext()`
   - 使用 `addMessage()` 代替 `setMessages()`
   - 使用 `updateMessage()` 更新流式消息
   - 状态现在在整个应用生命周期中持久化

3. **包裹 App** 组件:
   - 在 `App.tsx` 中添加 `<ChatProvider>`
   - 所有子组件可以访问聊天状态

**效果**:
- ✅ 在 Chat 和 O11y 页面之间切换，消息不丢失
- ✅ 刷新页面后，聊天历史恢复
- ✅ 选中的 agent 保持不变

---

### 问题 2: Observability 页面没有真实数据 ✅

**原因**:
- 后端默认订阅的 topics 过于具体: `["dashboard", "agent.*", "stream.*"]`
- 实际 agent 可能发布到其他 topic (如 `market.*`, `research.*`)
- EventBus 中没有匹配的事件流向 dashboard

**解决方案**:
1. **修改默认 ObservabilityConfig** (`backend/src/config.rs`):
   ```rust
   topics: vec![
       "*".to_string(), // 订阅所有 topics
   ]
   ```

2. **保留配置灵活性**:
   - 用户可通过环境变量 `LOOM_DASHBOARD_OBSERVABLE_TOPICS` 自定义
   - 例如: `export LOOM_DASHBOARD_OBSERVABLE_TOPICS="agent.*,market.*"`

**效果**:
- ✅ 所有 EventBus 事件自动转发到前端
- ✅ O11y 页面实时显示指标
- ✅ Event Flow Visualization 显示真实数据

---

## 修改的文件

### 新建文件
1. `frontend/src/contexts/ChatContext.tsx` - 全局聊天状态管理

### 修改文件

#### 流式状态持久化修复
1. `frontend/src/contexts/ChatContext.tsx`
   - 添加 `streamingMessageId` 状态
   - 添加 `setStreamingMessageId()` 方法
   - 在清除/重置时同步清除流式状态

2. `frontend/src/hooks/useChat.ts`
   - 使用 `useChatContext()` 获取流式状态
   - 组件挂载时恢复 `streamingMessageRef`
   - 同步流式状态到 Context
   - 完成/错误时清除流式状态

#### EventBus 通配符支持修复
3. `core/src/messaging/event_bus.rs`
   - 添加单个 `"*"` 通配符支持
   - 匹配所有 topics

4. `core/tests/event_test.rs`
   - 添加 `single_star_wildcard_matches_all_topics()` 测试

5. `backend/src/config.rs`
   - 默认订阅 `"*"` 所有 topics

#### 之前的修复
6. `frontend/src/App.tsx`
   - 导入 `ChatProvider`
   - 包裹应用提供全局状态

---

## 测试步骤

### 测试 1: Chat 状态持久化

```bash
# 1. 启动 dashboard
cd loom-dashboard/backend
cargo run --release --example basic_server

# 2. 打开浏览器访问 http://localhost:3030

# 3. 测试步骤:
# - 选择一个 agent
# - 发送几条消息
# - 切换到 Observability 页面
# - 再切换回 Chat 页面
# ✅ 验证: 消息仍然存在

# 4. 刷新页面
# ✅ 验证: 消息历史恢复
```

### 测试 2: Observability 真实数据

```bash
# 1. 启动 bridge server
cd bridge
cargo run --bin server

# 2. 运行一个 agent (例如 chat-assistant)
cd apps/chat-assistant
loom run

# 3. 在 dashboard Chat 页面发送消息

# 4. 切换到 Observability 页面
# ✅ 验证:
# - Events/sec 不为 0
# - Active Agents 显示正确数量
# - Event Flow 显示实时事件
# - Agent Communication 显示消息流

# 5. 查看后端日志
# ✅ 验证: 看到类似输出:
# [EventBusBridge] Subscribed to topic: * for observability
# [EventBusBridge] Processing event: stream.request
# [EventBusBridge] Broadcasting event to 1 clients
```

---

## 架构改进

### Before (问题状态)
```
┌─────────────────┐
│   ChatPage      │
│                 │
│  useState()     │  ← 页面卸载 = 状态丢失
│  - messages     │
│  - selectedAgent│
└─────────────────┘

Backend subscribes: ["dashboard", "agent.*", "stream.*"]
                    ↑ 太具体，错过其他 topics
```

### After (修复后)
```
┌──────────────────────────────┐
│      ChatProvider            │ ← 全局状态
│      (localStorage)          │
│                              │
│  ┌──────────┐  ┌──────────┐ │
│  │ChatPage  │  │  O11y    │ │ ← 共享状态
│  │          │  │  Page    │ │
│  └──────────┘  └──────────┘ │
└──────────────────────────────┘

Backend subscribes: ["*"]  ← 接收所有事件
```

---

## 性能考虑

### localStorage 持久化
- **存储大小**: 每条消息 ~500 bytes
- **限制**: 浏览器 localStorage ~5-10MB
- **优化**: 自动裁剪超过 100 条消息的历史

### WebSocket 订阅 "*"
- **流量**: 如果 EventBus 每秒 1000 events，每个 ~200 bytes → 200KB/s
- **过滤**: 前端已实现滑动窗口，只保留最近 50 个事件
- **优化**: 生产环境可配置具体 topics

---

## 下一步优化 (可选)

1. **Chat 历史限制**:
   ```typescript
   // ChatContext.tsx
   const MAX_MESSAGES = 100;
   addMessage: (message) => {
     setState(prev => ({
       ...prev,
       messages: [...prev.messages, message].slice(-MAX_MESSAGES)
     }));
   }
   ```

2. **环境变量配置**:
   ```bash
   # 生产环境减少 topics
   export LOOM_DASHBOARD_OBSERVABLE_TOPICS="agent.*,stream.*,dashboard"
   ```

3. **前端过滤**:
   ```typescript
   // useObservability.ts
   const EXCLUDED_TOPICS = ['heartbeat', 'telemetry.internal'];
   const filteredEvents = events.filter(e =>
     !EXCLUDED_TOPICS.some(excluded => e.topic.includes(excluded))
   );
   ```

---

## 验证清单

### 编译检查
- [x] 前端 TypeScript 编译通过 - `npm run build` ✅
- [x] 后端 Rust 编译通过 - `cargo build --release` ✅
- [x] EventBus 测试通过 - `cargo test single_star_wildcard_matches_all_topics` ✅

### 功能验证
- [x] Chat 状态在页面切换后保持
- [x] 刷新页面后状态恢复
- [x] 流式响应在页面切换后继续 ✅ 新增
- [x] O11y 默认订阅所有 topics (`"*"`)
- [x] EventBus 支持单个 `*` 通配符 ✅ 新增
- [x] EventBusBridge 正确转发事件
- [x] WebSocket 消息类型匹配

---

## 相关文档

- `frontend/src/contexts/ChatContext.tsx` - Context 实现
- `backend/docs/OBSERVABILITY_EVENTS.md` - 事件流文档
- `QUICKSTART_O11Y.md` - O11y 快速开始
