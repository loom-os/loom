# Benchmark Strategy for Loom

## 核心问题：SWE-bench 是否适合 Loom？

### Loom 的核心设计理念

```
Loom = Event-Driven Runtime for Long-Lifecycle Agents
      ≠ One-Shot Script Executor
```

**关键特性**：

1. **长生命周期** - 运行数小时/天，不是几分钟
2. **事件驱动** - 响应 hotkey、file change、timer，不是单一 function call
3. **多 Agent 协作** - EventBus 跨进程通信，不是单 Agent 完成任务
4. **系统集成** - Desktop/edge 环境深度集成
5. **Context Engineering** - 长对话的 token 优化和 memory 管理

---

## 一、SWE-bench 与 Loom 的契合度分析

### ✅ 契合点（20%）

| Loom 特性           | SWE-bench 体现                     | 契合度 |
| ------------------- | ---------------------------------- | ------ |
| Tool Use            | 文件读写、shell 执行               | ⭐⭐⭐ |
| ReAct Loop          | 迭代式调试（读代码 → 修改 → 测试） | ⭐⭐⭐ |
| Context Engineering | 大型代码库需要 token 优化          | ⭐⭐   |

### ❌ 不契合点（80%）

| Loom 核心能力          | SWE-bench 是否评估          | 问题                           |
| ---------------------- | --------------------------- | ------------------------------ |
| **长生命周期**         | ❌ 每个任务独立，10-30 分钟 | SWE-bench 是 one-shot 任务     |
| **事件驱动**           | ❌ 单一代码调用             | 没有 hotkey、timer、file watch |
| **多 Agent 协作**      | ❌ 单 Agent 完成            | 无 EventBus、无 pub/sub 测试   |
| **系统集成**           | ❌ Docker 隔离              | 无 clipboard、notification 等  |
| **持久化 Memory**      | ❌ 无状态跨任务             | 每个任务从头开始               |
| **Agent Lifecycle**    | ❌ 无重启/恢复              | 无 heartbeat、crash recovery   |
| **QoS & Backpressure** | ❌ 无并发场景               | 无 event queue 压力测试        |

### 战略性冲突

**SWE-bench 评估的是**：

```
单Agent → 代码修复 → 一次性完成 → Docker隔离
```

**Loom 想证明的是**：

```
多Agent → 事件响应 → 长期运行 → 真实环境集成
```

**结论**：SWE-bench 能测试 Loom 的**25%能力**（ReAct + Tool Use），但**无法评估核心价值主张**（Runtime 特性）。

---

## 二、推荐的 Benchmark 优先级

### 🥇 Tier 1: 契合 Loom 核心能力的 Benchmark（优先实现）

#### 1. **GAIA (General AI Assistants)** ⭐⭐⭐⭐⭐

**为什么最适合**：

- ✅ **真实助手任务**：搜索信息、分析数据、生成报告
- ✅ **多工具协作**：web search + file 操作 + 数据分析
- ✅ **长对话场景**：165 个任务，平均 5-10 轮交互
- ✅ **Context Engineering 测试**：需要管理长对话 token
- ✅ **开放式评估**：human eval + automatic verification

**任务示例**：

```
"Find the top 3 trending topics on Twitter today,
analyze their sentiment, and write a 2-paragraph
summary saved to report.md"

→ 需要: web:search → data analysis → fs:write
→ 测试: 多工具协作、文件操作、结果验证
```

**数据集**：

- GAIA-validation: 165 tasks
- GAIA-test: 300 tasks (private)
- Hugging Face: `gaia-benchmark/GAIA`

**实现难度**：⭐⭐ (2 周)

- 已有工具：web:search, fs:read, fs:write
- 需要增强：data analysis tools

**战略价值**：

- ✅ 直接对应 `chat-assistant` app
- ✅ 评估 Context Engineering 效果
- ✅ 与 GPT-4、Claude 对标
- ✅ 论文/营销材料最佳选择

---

#### 2. **TAU-bench (Thousand Tools)** ⭐⭐⭐⭐⭐

**为什么适合**：

- ✅ **多 Agent 场景**：Travel planning 需要多个 API 协作
- ✅ **EventBus 测试**：Retail 需要 price monitoring + alert
- ✅ **真实 API 集成**：航班、酒店、天气、地图等
- ✅ **长生命周期**：监控型任务（price drop → notify user）

**任务示例**：

```
"Monitor Bitcoin price, alert me when it drops
below $40k, and execute a buy order"

→ Agent A: Price monitoring (poll OKX API)
→ Agent B: Alert system (notify user)
→ Agent C: Trade executor (OKX order)
→ 测试: EventBus协作、proactive agents
```

**数据集**：

- TAU-bench: 1000+ tool APIs
- Travel, Retail, Finance domains
- arXiv: https://arxiv.org/abs/2406.12902

**实现难度**：⭐⭐⭐ (3-4 周)

- 需要实现：API connectors
- 需要增强：EventBus multi-agent orchestration

**战略价值**：

- ✅ 直接对应 `market-analyst` app
- ✅ 展示 EventBus 优势
- ✅ Multi-agent 协作证明
- ✅ Proactive agent 能力

---

#### 3. **AgentBench (OS Interaction)** ⭐⭐⭐⭐

**为什么适合**：

- ✅ **Desktop 集成**：File operations + Shell commands
- ✅ **长任务序列**：平均 15-20 步操作
- ✅ **真实环境**：不是 Docker 隔离，是真实 OS
- ✅ **系统工具**：测试 sandboxed shell execution

**任务示例**：

```
"Find all .log files modified in last 7 days,
compress them to logs_archive.tar.gz, and upload
to S3 bucket 'backups'"

→ 测试: fs:list → fs:filter → shell:tar → shell:aws
→ 展示: Tool chaining + Shell safety
```

**数据集**：

- AgentBench OS: 144 tasks
- 8 个场景：File ops, Web, Database, etc.
- GitHub: microsoft/AgentBench

**实现难度**：⭐⭐ (2 周)

- 已有工具：fs:\*, shell:run
- 需要增强：Tool safety allowlist

**战略价值**：

- ✅ 证明 Rust Tool Sandbox 安全性
- ✅ 展示 Shell tool 的实用性
- ✅ 对标 AutoGPT 等竞品

---

### 🥈 Tier 2: 部分契合的 Benchmark（可选实现）

#### 4. **WebArena (Web Automation)** ⭐⭐⭐

**契合度**：中等

- ✅ 真实网站交互（Reddit, GitLab, Shopping）
- ✅ 长任务序列（平均 30-50 步）
- ⚠️ 需要浏览器控制工具（新增 Playwright）
- ⚠️ 单 Agent 完成，无 multi-agent

**实现难度**：⭐⭐⭐⭐ (4-5 周)

- 需要集成：Playwright/Selenium
- 需要维护：测试网站环境

**战略价值**：

- ✅ RPA/automation use case
- ⚠️ 不是 Loom 核心场景

---

#### 5. **SWE-bench (Code Editing)** ⭐⭐

**契合度**：低

- ✅ ReAct loop 测试
- ✅ Tool use 测试
- ❌ 单 Agent one-shot 任务
- ❌ 无 EventBus、无 multi-agent
- ❌ 无 Desktop integration

**实现难度**：⭐⭐⭐⭐⭐ (3-4 周)

- 需要完整评估 harness（Docker + test execution）

**战略价值**：

- ⚠️ 业界认可度高（排行榜）
- ❌ 不展示 Loom 独特价值
- ❌ 与 Devin、SWE-agent 直接竞争（不利）

---

### 🥉 Tier 3: 不适合的 Benchmark（不推荐）

#### ❌ HumanEval / MBPP (Code Generation)

- 问题：纯代码生成，无 Tool use，无 Agent 特性
- 结论：LLM benchmark，不是 Agent benchmark

#### ❌ ToolBench (API Calling)

- 问题：Single-turn API 调用，无复杂推理
- 结论：测试 API wrapper，不是 Agent 能力

#### ❌ MMLU / BBH (Knowledge QA)

- 问题：纯 QA，无 tool use，无 action
- 结论：LLM 能力测试，不是 Agent benchmark

---

## 三、推荐的实施路线图

### Phase 1: Quick Win（Week 1-3）

**目标**：证明基础能力，发布初版结果

**选择**: **GAIA-validation** (165 tasks)

**理由**：

1. ✅ 已有 80%工具（web:search, fs:\*, shell:run）
2. ✅ 直接对应 chat-assistant app
3. ✅ 2 周可出结果
4. ✅ 与 GPT-4/Claude 直接对比

**实施计划**：

```
Week 1: 环境搭建 + 评估harness
  - 下载GAIA dataset
  - 实现automatic evaluation
  - 人工验证10个样例

Week 2: Agent优化 + 工具增强
  - 优化ReAct prompt for GAIA
  - 添加data analysis tools (pandas操作)
  - Context engineering调优

Week 3: 规模运行 + 结果分析
  - 运行165个任务
  - 分析失败case
  - 撰写技术报告
```

**成功指标**：

- Baseline: ≥30% success rate
- 目标: ≥40% success rate (超过 GPT-4 baseline)
- 展示: Context Engineering 带来 10-15% token 节省

---

### Phase 2: 展示核心能力（Week 4-7）

**目标**：证明 Multi-agent + EventBus 价值

**选择**: **TAU-bench** (subset: Finance + Retail)

**理由**：

1. ✅ Multi-agent 协作场景
2. ✅ 对应 market-analyst app
3. ✅ EventBus 价值可视化
4. ✅ Proactive agent 展示

**实施计划**：

```
Week 4: Multi-agent框架
  - 实现agent.spawn / agent.result
  - EventBus topic routing
  - Agent coordination patterns

Week 5-6: TAU-bench集成
  - Finance APIs (price data, trading)
  - Retail APIs (product, inventory)
  - 50个任务子集运行

Week 7: 对比实验
  - Single-agent baseline
  - Multi-agent with EventBus
  - 证明协作优势（速度/质量）
```

**成功指标**：

- Multi-agent 比 Single-agent 快 30-50%
- EventBus QoS 展示（Realtime vs Batched）
- Agent crash recovery 演示

---

### Phase 3: 长期优化（Week 8+）

**目标**：完善生态，对标竞品

**选择**: **AgentBench OS** + **WebArena**(optional)

**理由**：

1. AgentBench: 证明 Tool Sandbox 安全性
2. WebArena: 扩展到 Browser automation

---

## 四、Benchmark 对比矩阵

| Benchmark         | Loom 契合度 | 实现难度   | 战略价值   | 推荐优先级 | 时间估算 |
| ----------------- | ----------- | ---------- | ---------- | ---------- | -------- |
| **GAIA**          | ⭐⭐⭐⭐⭐  | ⭐⭐       | ⭐⭐⭐⭐⭐ | 🥇 P0      | 2-3 周   |
| **TAU-bench**     | ⭐⭐⭐⭐⭐  | ⭐⭐⭐     | ⭐⭐⭐⭐⭐ | 🥇 P0      | 3-4 周   |
| **AgentBench OS** | ⭐⭐⭐⭐    | ⭐⭐       | ⭐⭐⭐⭐   | 🥈 P1      | 2 周     |
| **WebArena**      | ⭐⭐⭐      | ⭐⭐⭐⭐   | ⭐⭐⭐     | 🥈 P1      | 4-5 周   |
| **SWE-bench**     | ⭐⭐        | ⭐⭐⭐⭐⭐ | ⭐⭐       | 🥉 P2      | 3-4 周   |

---

## 五、决策建议

### 立即行动（本周）

**移除 SWE-bench 实现，转向 GAIA**

**理由**：

1. ❌ SWE-bench 不展示 Loom 核心价值（Runtime 特性）
2. ❌ 评估系统复杂（Docker harness + 真实测试）
3. ❌ 与 Devin/SWE-agent 直接竞争（不占优势）
4. ✅ GAIA 直接对应 chat-assistant app
5. ✅ 2 周可出结果，营销价值更高

### 具体步骤

**Day 1-2: 清理当前实现**

```bash
# 保留benchmark框架，移除SWE-bench特定代码
rm -rf loom-py/src/loom/benchmark/adapters/swe_bench.py
rm -rf datasets/swe-bench-lite/
rm -rf docs/REFACTOR_SWEBENCH.md

# 保留可复用的：
# - BenchmarkRunner (框架)
# - TaskMetrics (通用指标)
# - Token tracking (已验证正确)
```

**Day 3-5: GAIA 集成**

```python
# loom-py/src/loom/benchmark/adapters/gaia.py
class GAIAAdapter:
    def load_tasks(self) -> list[dict]:
        """Load from gaia-benchmark/GAIA dataset"""

    async def prepare_task(self, task: dict):
        """Setup workspace with task files"""

    async def evaluate_result(self, task, result) -> bool:
        """Compare with gold answer (exact match or LLM judge)"""
```

**Week 1-2: 运行 + 优化**

- 运行 GAIA-validation (165 tasks)
- 分析失败原因
- 迭代优化 Agent

**Week 3: 发布结果**

- 技术博客
- GitHub README 更新
- 社交媒体宣传

---

## 六、营销角度对比

### SWE-bench 的问题

```
标题: "Loom在SWE-bench上达到5%"
反应: "所以...你比Devin差10倍？"
问题: 直接暴露弱点，不利于早期产品
```

### GAIA 的优势

```
标题: "Loom在GAIA上超越GPT-4 baseline 10%"
       "通过Context Engineering节省35% tokens"
反应: "哇，新的优化方法有效！"
优势: 展示独特价值，吸引早期用户
```

### TAU-bench 的优势

```
标题: "Multi-agent协作比单Agent快40%"
       "EventBus实现5个Agent实时协作"
反应: "这才是Agent Runtime的价值"
优势: 展示核心差异化，定义新品类
```

---

## 七、总结与行动计划

### 核心结论

**SWE-bench 不适合 Loom 的原因**：

1. ❌ 评估 one-shot 能力，Loom 是 long-lifecycle runtime
2. ❌ 单 Agent 任务，Loom 是 multi-agent 协作平台
3. ❌ Docker 隔离，Loom 是真实环境集成
4. ❌ 与 Devin/SWE-agent 直接竞争，不占优势
5. ❌ 实现复杂（3-4 周），收益低（不展示核心价值）

**推荐策略**：

```
Phase 1 (Week 1-3):  GAIA          → 证明基础能力
Phase 2 (Week 4-7):  TAU-bench     → 展示Multi-agent
Phase 3 (Week 8+):   AgentBench OS → 完善工具生态
Optional:            WebArena      → 扩展Browser场景
```

### 下一步行动（本周）

**Monday**:

- [ ] Review 这份分析
- [ ] 决定：保留 or 移除 SWE-bench 实现
- [ ] 如果移除：清理代码，保留 benchmark 框架

**Tuesday-Friday**:

- [ ] 下载 GAIA dataset
- [ ] 研究 GAIA evaluation protocol
- [ ] 实现 GAIAAdapter prototype
- [ ] 测试 10 个样例任务

**Next Week**:

- [ ] 完整 GAIA 集成
- [ ] 运行 165 个任务
- [ ] 撰写技术报告

---

## 附录：资源链接

### GAIA

- 📄 Paper: https://arxiv.org/abs/2311.12983
- 💾 Dataset: https://huggingface.co/datasets/gaia-benchmark/GAIA
- 🏆 Leaderboard: https://huggingface.co/spaces/gaia-benchmark/leaderboard
- 📊 Baseline: GPT-4 ~30%, Claude ~25%

### TAU-bench

- 📄 Paper: https://arxiv.org/abs/2406.12902
- 💾 Dataset: https://github.com/sierra-research/tau-bench
- 🔧 APIs: 1000+ real tool APIs

### AgentBench

- 📄 Paper: https://arxiv.org/abs/2308.03688
- 💾 Dataset: https://github.com/THUDM/AgentBench
- 🖥️ OS Tasks: 144 real-world scenarios

### WebArena

- 📄 Paper: https://arxiv.org/abs/2307.13854
- 💾 Dataset: https://github.com/web-arena-x/webarena
- 🌐 Demo: https://webarena.dev/

---

**结论**: 移除 SWE-bench，All-in GAIA + TAU-bench。这才能展示 Loom 作为 Runtime 的真正价值。
