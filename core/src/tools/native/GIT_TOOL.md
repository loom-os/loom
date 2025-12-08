# Git Native Tool

Git native tool 提供安全的 git 操作能力，用于 benchmark 和工作流中的代码仓库管理。

## 功能特性

- ✅ **安全路径验证**：所有路径必须是相对路径，防止目录遍历攻击
- ✅ **完整的 Tool trait 实现**：包含详细的参数描述和示例
- ✅ **异步支持**：使用 async_trait 实现非阻塞操作
- ✅ **错误处理**：提供清晰的错误信息

## 支持的操作

### 1. clone - 克隆仓库

克隆 git 仓库到本地目录。

**参数**：
```json
{
  "operation": "clone",
  "url": "https://github.com/user/repo.git",
  "path": "my-repo",
  "branch": "main"  // 可选，默认克隆所有分支
}
```

**返回**：
```json
{
  "success": true,
  "path": "my-repo",
  "message": "Cloned repository to my-repo",
  "output": "Cloning into 'my-repo'..."
}
```

**安全限制**：
- `path` 必须是相对路径
- 不能包含 `..` （父目录遍历）
- 不能以 `/` 开头（绝对路径）

### 2. checkout - 切换分支/提交

切换到指定的分支或提交。

**参数**：
```json
{
  "operation": "checkout",
  "repo_path": "my-repo",
  "commit": "abc123def"  // 可以是 commit hash、branch、或 tag
}
```

**返回**：
```json
{
  "success": true,
  "commit": "abc123def",
  "message": "Checked out commit abc123def"
}
```

### 3. apply_patch - 应用补丁

应用 git patch 到仓库。

**参数**：
```json
{
  "operation": "apply_patch",
  "repo_path": "my-repo",
  "patch_content": "diff --git a/file.txt b/file.txt\n..."
}
```

**返回**：
```json
{
  "success": true,
  "message": "Patch applied successfully"
}
```

### 4. current_commit - 获取当前提交

获取当前 HEAD 指向的 commit hash。

**参数**：
```json
{
  "operation": "current_commit",
  "repo_path": "my-repo"
}
```

**返回**：
```json
{
  "success": true,
  "commit": "abc123def456789..."
}
```

## 配置

### 在 loom.toml 中启用

```toml
[agents.chat-assistant]
tools = ["fs:read", "fs:write", "git"]
```

### 在 Rust 中注册

```rust
use loom_core::{GitTool, ToolRegistry};

let registry = ToolRegistry::new();
registry.register(Arc::new(GitTool::new())).await;
```

## 使用示例

### SWE-bench 工作流

```python
# 1. 克隆仓库
await agent.tool("git", {
    "operation": "clone",
    "url": "https://github.com/astropy/astropy.git",
    "path": "repo"
})

# 2. 切换到特定提交
await agent.tool("git", {
    "operation": "checkout",
    "repo_path": "repo",
    "commit": "d16bfe05"
})

# 3. 检查当前提交
result = await agent.tool("git", {
    "operation": "current_commit",
    "repo_path": "repo"
})
print(f"Current commit: {result['commit']}")
```

### Agent 使用（通过 LLM）

Agent 可以通过 ReAct 模式调用 git 工具：

```
Thought: I need to clone the repository first
Action: {"tool": "git", "args": {
  "operation": "clone",
  "url": "https://github.com/user/repo.git",
  "path": "workspace/repo"
}}

Observation: {"success": true, "path": "workspace/repo", ...}

Thought: Now I'll checkout the specific commit
Action: {"tool": "git", "args": {
  "operation": "checkout",
  "repo_path": "workspace/repo",
  "commit": "abc123"
}}
```

## 安全考虑

### 路径验证

所有路径都经过严格验证：
- ✅ 必须是相对路径
- ❌ 不允许 `../` 父目录遍历
- ❌ 不允许绝对路径 `/path/to/dir`
- ❌ 不允许符号链接逃逸

### 命令执行

- 使用 Rust 的 `std::process::Command`，不通过 shell
- 所有参数都经过转义
- 超时保护（通过异步实现）

### 建议的使用场景

✅ **适合使用**：
- Benchmark 中克隆测试仓库
- 工作流中检出特定版本
- 自动化测试中应用补丁

❌ **不适合使用**：
- 生产环境中的敏感仓库操作
- 需要 SSH 认证的私有仓库
- 复杂的 git 工作流（merge, rebase 等）

## 限制和已知问题

1. **浅克隆**：默认使用 `--depth 1`，减少网络传输
2. **临时文件**：`apply_patch` 会创建临时文件 `.loom_temp.patch`
3. **错误信息**：git 的 stderr 输出会直接返回给调用者
4. **认证**：目前只支持 HTTPS 公开仓库

## 未来改进

- [ ] 支持 SSH 认证
- [ ] 添加 `git status` 操作
- [ ] 添加 `git diff` 操作
- [ ] 支持 `git log` 查询
- [ ] 添加进度回调（大仓库克隆）
- [ ] 支持 git 子模块

## 相关文档

- [Native Tools Overview](../README.md)
- [Tool Registry](../registry.rs)
- [SWE-bench Adapter](../../../../loom-py/src/loom/benchmark/adapters/swe_bench.py)
