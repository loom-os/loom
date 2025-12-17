# Dashboard 测试指南

## 快速验证所有修复

### 准备环境

```bash
# Terminal 1: 启动 loom 环境 (bridge + agents)
cd /home/jared/loom/apps/chat-assistant
loom run

# Terminal 2: 启动 dashboard
cd /home/jared/loom/loom-dashboard/backend
cargo run --release --example basic_server

# 浏览器访问: http://localhost:3030
```

---

## 测试 1: 流式响应持久化 ✅

**场景**: 在流式响应过程中切换页面，返回后继续接收

### 步骤
1. 选择 `chat-assistant` agent
2. 发送消息: "Write a detailed explanation of neural networks"
3. **在响应流式输出过程中**，点击 "Observability" 标签
4. 等待 2-3 秒
5. 点击 "Chat" 标签返回

### 预期结果
- ✅ 消息继续累积，显示完整内容
- ✅ 不会卡住或只显示部分内容
- ✅ `isLoading` 状态正确（完成后消失）

### 控制台验证
```
[useChat] Restored streaming state for message: msg-1234567890
```

---

## 测试 2: 聊天状态持久化 ✅

**场景**: 页面切换和刷新后，消息历史保持

### 步骤
1. 发送 3-5 条消息
2. 切换到 Observability 页面
3. 返回 Chat 页面
4. 按 F5 刷新浏览器

### 预期结果
- ✅ 步骤 3: 所有消息仍然显示
- ✅ 步骤 3: 选中的 agent 保持
- ✅ 步骤 4: 刷新后历史消息恢复
- ✅ 步骤 4: Thread ID 保持不变

### 控制台验证
```
[ChatContext] Loaded state from localStorage
messages: 5 items
threadId: "thread-1734367890123"
```

---

## 测试 3: Observability 真实数据 ✅

**场景**: O11y 页面显示实时 EventBus 数据

### 步骤
1. 打开 Observability 页面
2. 切换回 Chat 页面，发送消息
3. 再次打开 Observability 页面

### 预期结果
- ✅ **Events/sec** 不为 0 (显示实时速率)
- ✅ **Active Agents** 显示正确数量 (至少 1 个)
- ✅ **Event Flow** 显示滚动的事件列表
- ✅ **Agent Communication** 显示工具调用和输出
- ✅ 右上角显示 "● Live" 绿色标记

### 后端日志验证
```bash
# 在 dashboard 终端查看日志
grep "Subscribed to topic" <terminal-output>
```

预期输出:
```
[EventBusBridge] Subscribed to topic: * for observability
[EventBusBridge] Broadcasting event to 1 clients
```

---

## 测试 4: EventBus 通配符支持 ✅

**场景**: 验证 "*" 通配符订阅所有 topics

### 步骤
```bash
cd /home/jared/loom/core
cargo test single_star_wildcard_matches_all_topics -- --nocapture
```

### 预期结果
```
test single_star_wildcard_matches_all_topics ... ok
```

---

## 常见问题排查

### O11y 页面仍然显示 0 或 mock 数据

**检查清单**:
1. `loom run` 是否正在运行？
   ```bash
   ps aux | grep -E "loom|bridge" | grep -v grep
   ```

2. Dashboard 是否连接到正确的 bridge？
   - 默认: `ws://localhost:3030/ws`
   - 检查浏览器控制台是否有 WebSocket 连接

3. EventBus 是否有事件发布？
   ```bash
   # 在 Chat 页面发送消息后，检查后端日志
   # 应该看到 "Processing event: stream.request"
   ```

4. 浏览器是否显示 "● Live"？
   - 如果显示 "● Disconnected"，检查 WebSocket URL
   - F12 → Network → WS → 查看连接状态

### 流式响应在页面切换后没有继续

**检查清单**:
1. 消息是否标记为 `isStreaming: true`？
   ```javascript
   // 在浏览器控制台
   console.log(messages[messages.length - 1].isStreaming)
   ```

2. `streamingMessageId` 是否正确保存？
   ```javascript
   // 在浏览器控制台
   localStorage.getItem('loom-chat-state')
   // 应该包含 streamingMessageId
   ```

3. WebSocket 是否仍然连接？
   - 检查右上角连接状态

### 页面刷新后消息丢失

**检查清单**:
1. localStorage 是否启用？
   ```javascript
   // 在浏览器控制台
   typeof localStorage !== 'undefined'
   ```

2. 检查存储大小
   ```javascript
   // 在浏览器控制台
   JSON.stringify(localStorage.getItem('loom-chat-state')).length
   // 应该 < 5MB
   ```

---

## 性能验证

### 检查 EventBus 订阅数量
```bash
# 在 bridge 日志中查找
grep "active_subscriptions" <terminal-output>
```

### 检查前端内存使用
1. F12 → Performance → Memory
2. 发送 20+ 条消息
3. 切换页面多次
4. 检查内存是否稳定 (不应持续增长)

### 检查 WebSocket 流量
1. F12 → Network → WS
2. 查看消息频率
3. 正常情况: 发送消息时有流量，idle 时无流量

---

## 成功标准

所有以下检查通过:

- [ ] 测试 1: 流式响应持久化 ✅
- [ ] 测试 2: 聊天状态持久化 ✅
- [ ] 测试 3: O11y 真实数据 ✅
- [ ] 测试 4: EventBus 通配符 ✅
- [ ] 前端编译无错误
- [ ] 后端编译无错误
- [ ] 单元测试全部通过
- [ ] 无内存泄漏
- [ ] WebSocket 连接稳定

---

## 回归测试

在发布前运行完整测试套件:

```bash
# 后端测试
cd /home/jared/loom/loom-dashboard/backend
cargo test --lib

cd /home/jared/loom/core
cargo test event_test

# 前端构建
cd /home/jared/loom/loom-dashboard/frontend
npm run build
```

全部通过 = 可以发布 ✅
