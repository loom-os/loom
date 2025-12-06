# Data & Logic Offloading

This document details the offloading strategies for keeping LLM context lean.

> 📖 **See also:** [LIFECYCLE.md](LIFECYCLE.md) for the complete 8-phase offload lifecycle (creation → archival)

---

## Principle

> **The best context is small context. Offload everything recoverable.**

Three types of offloading:

1. **Data Offloading** — Large outputs → workspace files
2. **Tool Offloading** — Complex operations → CLI sandbox
3. **Logic Offloading** — Business logic → external scripts

---

## 1. Data Offloading

### When to Offload

| Content Type    | Threshold   | Target                   |
| --------------- | ----------- | ------------------------ |
| File content    | > 1KB       | `outputs/{step_id}.txt`  |
| Search results  | > 5 results | `outputs/{step_id}.json` |
| Shell output    | > 500 chars | `outputs/{step_id}.log`  |
| Browser content | Always      | `outputs/{step_id}.html` |
| API responses   | > 2KB       | `outputs/{step_id}.json` |

### Implementation

```python
class DataOffloader:
    """Offloads large data to workspace files."""

    THRESHOLDS = {
        "text": 1024,      # 1KB
        "json": 2048,      # 2KB
        "log": 500,        # 500 chars
        "html": 0,         # Always offload
    }

    def __init__(self, workspace: Path):
        self.output_dir = workspace / "outputs"
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def should_offload(self, data: str, content_type: str) -> bool:
        threshold = self.THRESHOLDS.get(content_type, 1024)
        return len(data) > threshold

    def offload(
        self,
        data: str,
        step_id: str,
        content_type: str = "text"
    ) -> str:
        """Offload data to file, return reference path."""

        ext_map = {"text": "txt", "json": "json", "log": "log", "html": "html"}
        ext = ext_map.get(content_type, "txt")

        filename = f"{step_id}.{ext}"
        path = self.output_dir / filename
        path.write_text(data)

        return f"workspace/outputs/{filename}"

    def create_reference(self, path: str, summary: str) -> str:
        """Create a prompt-friendly reference."""
        return f"{summary} [Saved to {path}. Use fs:read_file to retrieve.]"
```

### Offload Flow

```
Tool execution:
    ┌─────────────┐
    │ web:search  │
    │ (10 results)│
    └──────┬──────┘
           │
           ▼
    ┌─────────────────────────────────────┐
    │ DataOffloader.should_offload()      │
    │ len(results) > 5? YES               │
    └──────┬──────────────────────────────┘
           │
           ▼
    ┌─────────────────────────────────────┐
    │ DataOffloader.offload()             │
    │ → workspace/outputs/search_001.json │
    └──────┬──────────────────────────────┘
           │
           ▼
    ┌─────────────────────────────────────┐
    │ Step observation:                   │
    │ "Found 10 results for 'Bitcoin'.    │
    │  Saved to workspace/outputs/        │
    │  search_001.json"                   │
    └─────────────────────────────────────┘
```

### Retrieval Pattern

When LLM needs offloaded data:

```
LLM sees:
    Observation: Found 10 results. Saved to workspace/outputs/search_001.json

LLM responds:
    Thought: I need to see the search results to continue.
    Action: {"tool": "fs:read_file", "args": {"path": "workspace/outputs/search_001.json"}}
```

**This is intentional**: LLM must be explicit about what it needs.

---

## 2. Tool Offloading

### Hierarchical Action Space

```
┌─────────────────────────────────────────────────────────────────┐
│  Level 1: LLM-Facing Tools (Always in prompt)                   │
│  ─────────────────────────────────────────────                  │
│  fs:read_file     - Read file content                           │
│  fs:write_file    - Write file content                          │
│  fs:list_dir      - List directory                              │
│  web:search       - Web search                                  │
│  agent:spawn      - Spawn sub-agent                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ LLM can invoke via system:shell
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Level 2: CLI Utilities (Sandboxed shell)                       │
│  ─────────────────────────────────────────                      │
│  git       - Version control                                    │
│  curl      - HTTP requests                                      │
│  jq        - JSON processing                                    │
│  grep/sed  - Text processing                                    │
│  python    - Script execution                                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Scripts can use
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Level 3: External APIs & Packages                              │
│  ─────────────────────────────────                              │
│  requests, pandas, numpy, ...                                   │
│  OKX API, OpenAI API, ...                                       │
│  Custom analysis libraries                                      │
└─────────────────────────────────────────────────────────────────┘
```

### Why Hierarchy Matters

**Before (flat tool list):**

```
Available tools: fs:read_file, fs:write_file, fs:list_dir, fs:delete,
web:search, browser:navigate, browser:click, browser:scroll,
git:clone, git:commit, git:push, git:pull, git:diff, git:log,
curl:get, curl:post, jq:filter, ...
(50+ tools in prompt = 2000+ tokens)
```

**After (hierarchical):**

```
Available tools:
- fs:read_file, fs:write_file, fs:list_dir
- web:search
- system:shell (for git, curl, jq, python, etc.)

(10 tools in prompt = 400 tokens)
```

### Shell Tool with Hierarchy

```python
class ShellTool:
    """Level 2 tool for CLI utilities."""

    # Level 2 commands (always available via shell)
    L2_COMMANDS = {
        "git", "curl", "wget", "jq", "grep", "sed", "awk",
        "cat", "head", "tail", "wc", "sort", "uniq",
        "python", "python3", "node", "npm",
    }

    async def call(self, command: str, args: list[str]) -> dict:
        if command not in self.L2_COMMANDS:
            return {"error": f"Command {command} not in L2 allowlist"}

        # Execute in sandbox
        result = await run_sandboxed([command] + args)

        # Offload if output is large
        if len(result.stdout) > 500:
            path = self.offloader.offload(result.stdout, "shell", "log")
            return {
                "exit_code": result.returncode,
                "output_ref": path,
                "summary": f"Command completed. Output saved to {path}",
            }

        return {
            "exit_code": result.returncode,
            "stdout": result.stdout,
            "stderr": result.stderr,
        }
```

---

## 3. Logic Offloading

### The Problem

Complex business logic in LLM prompt:

```
Thought: I need to calculate the RSI indicator.
RSI = 100 - (100 / (1 + RS))
RS = Average Gain / Average Loss
Average Gain = Sum of Gains over past N periods / N
...
(Complex calculations in LLM context = errors + token waste)
```

### The Solution: Script API

```python
# scripts/indicators.py
def calculate_rsi(prices: list[float], period: int = 14) -> float:
    """Calculate RSI indicator."""
    gains = []
    losses = []
    for i in range(1, len(prices)):
        change = prices[i] - prices[i-1]
        gains.append(max(0, change))
        losses.append(max(0, -change))

    avg_gain = sum(gains[-period:]) / period
    avg_loss = sum(losses[-period:]) / period

    if avg_loss == 0:
        return 100

    rs = avg_gain / avg_loss
    return 100 - (100 / (1 + rs))
```

LLM just invokes:

```
Action: {"tool": "python:run_script", "args": {
    "script": "scripts/indicators.py",
    "function": "calculate_rsi",
    "args": {"prices": [45000, 45200, 44800, ...], "period": 14}
}}
```

### Script Tool Implementation

```python
class ScriptTool:
    """Level 3 tool for running external scripts."""

    def __init__(self, scripts_dir: Path):
        self.scripts_dir = scripts_dir

    async def call(
        self,
        script: str,
        function: str,
        args: dict,
    ) -> dict:
        # Validate script exists
        script_path = self.scripts_dir / script
        if not script_path.exists():
            return {"error": f"Script {script} not found"}

        # Run in subprocess for isolation
        result = await run_python_function(script_path, function, args)

        return {
            "success": True,
            "result": result,
        }

async def run_python_function(
    script_path: Path,
    function: str,
    args: dict
) -> Any:
    """Run a Python function in isolated subprocess."""

    code = f"""
import json
import sys
sys.path.insert(0, '{script_path.parent}')
from {script_path.stem} import {function}
result = {function}(**{args!r})
print(json.dumps(result))
"""

    proc = await asyncio.create_subprocess_exec(
        "python", "-c", code,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    stdout, stderr = await proc.communicate()

    if proc.returncode != 0:
        raise RuntimeError(stderr.decode())

    return json.loads(stdout.decode())
```

### Script Library Structure

```
workspace/
├── scripts/
│   ├── indicators.py      # Technical analysis
│   ├── report.py          # Report generation
│   ├── data_clean.py      # Data preprocessing
│   └── api_helpers.py     # API wrappers
│
└── outputs/
    └── ...
```

### Benefits of Logic Offloading

| Aspect     | In-Prompt Logic            | Script Offloading        |
| ---------- | -------------------------- | ------------------------ |
| Token cost | High (logic in every call) | Low (just function call) |
| Accuracy   | LLM may make calc errors   | Deterministic code       |
| Caching    | Cannot cache               | Can cache results        |
| Testing    | Cannot unit test           | Full test coverage       |
| Iteration  | Must re-prompt             | Edit script, rerun       |

---

## Combined Example

A research task with all three offloading types:

```python
# LLM sees minimal tool list (Level 1)
tools = ["web:search", "fs:read_file", "fs:write_file", "python:run_script"]

# Step 1: Search (data offloaded)
Action: {"tool": "web:search", "args": {"query": "Bitcoin technical analysis"}}
Observation: Found 15 results. Saved to workspace/outputs/search_001.json

# Step 2: Retrieve specific results (explicit retrieval)
Action: {"tool": "fs:read_file", "args": {"path": "workspace/outputs/search_001.json"}}
Observation: [JSON with 15 results]

# Step 3: Get price data via shell (tool offloading)
Action: {"tool": "system:shell", "args": {"command": "curl", "args": ["-s", "https://api.example.com/btc/prices"]}}
Observation: Price data saved to workspace/outputs/prices_001.json

# Step 4: Calculate indicators (logic offloading)
Action: {"tool": "python:run_script", "args": {
    "script": "scripts/indicators.py",
    "function": "calculate_all_indicators",
    "args": {"prices_file": "workspace/outputs/prices_001.json"}
}}
Observation: {"rsi": 65.2, "macd": 0.5, "bollinger": {"upper": 46000, "lower": 44000}}

# Step 5: Generate report (logic offloading)
Action: {"tool": "python:run_script", "args": {
    "script": "scripts/report.py",
    "function": "generate_analysis_report",
    "args": {"indicators": {...}, "search_results": "workspace/outputs/search_001.json"}
}}
Observation: Report saved to workspace/reports/btc_analysis_2024_12_05.md
```

**LLM context stayed small** throughout:

- Raw data in files, not prompt
- Complex logic in scripts, not prompt
- Only orchestration in LLM

---

## Implementation Checklist

### P0: Data Offloading

- [ ] `DataOffloader` class with thresholds
- [ ] Integrate with `StepReducer`
- [ ] Reference format in observations

### P1: Tool Hierarchy

- [ ] Define L1/L2/L3 tool categories
- [ ] Update `build_react_system_prompt()` for L1 only
- [ ] Shell tool with L2 allowlist

### P2: Script Offloading

- [ ] `python:run_script` tool
- [ ] Script directory structure
- [ ] Subprocess isolation

---

_See also: `DESIGN.md` for overall architecture, `REDUCTION.md` for step reduction_
