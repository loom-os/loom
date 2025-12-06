# Context Reduction & Compaction

This document details the **per-step reduction** strategies for Loom's Context Engineering.

> 📖 **See also:** [COMPACTION.md](COMPACTION.md) for long conversation history compression (when >5 steps)

---

## Reduction: Per-Tool Rules

### Principle

> **Keep what's needed to resume; discard what's recoverable.**

Every tool result follows: `{minimal_args, observation, outcome_ref}`

---

## Tool-Specific Reduction Rules

### fs:read_file

```python
# Input
{"path": "config.json"}

# Full result
{"content": "{\n  \"api_key\": \"sk-...\",\n  \"model\": \"gpt-4\",\n  ...1500 chars...}"}

# Reduced
{
    "tool": "fs:read_file",
    "minimal_args": {"path": "config.json"},
    "observation": "Read config.json (1523 bytes, JSON)",
    "outcome_ref": None  # Content stays in LLM if small, or:
    "outcome_ref": "workspace/outputs/read_001.json"  # If large
}
```

**Rules:**

- Keep: path, size, detected type (JSON/text/binary)
- Discard: full content if > 1KB
- Note: Content often needed for reasoning → keep in working memory until compacted

### fs:write_file

```python
# Input
{"path": "report.md", "content": "# Analysis Report\n\n...5000 chars..."}

# Reduced
{
    "tool": "fs:write_file",
    "minimal_args": {"path": "report.md"},
    "observation": "Wrote report.md (5234 bytes)",
    "outcome_ref": None  # Content already on disk
}
```

**Rules:**

- Keep: path, bytes written
- Discard: content (already persisted to disk)
- Note: Never store written content in history

### fs:list_dir

```python
# Input
{"path": "src/"}

# Full result
{"entries": ["main.py", "utils.py", "config/", "tests/", ...20 items...]}

# Reduced
{
    "tool": "fs:list_dir",
    "minimal_args": {"path": "src/"},
    "observation": "Listed src/: 20 items (15 files, 5 dirs)",
    "outcome_ref": "workspace/outputs/list_001.json"  # Full list if > 10 items
}
```

**Rules:**

- Keep: path, count summary
- Discard: full listing if > 10 items
- Note: LLM usually only needs to know "what's there", not full list

### web:search

```python
# Input
{"query": "Bitcoin price prediction 2024", "count": 10}

# Full result
{
    "results": [
        {"title": "...", "url": "...", "snippet": "..."},
        # ...10 results, ~3KB total
    ]
}

# Reduced
{
    "tool": "web:search",
    "minimal_args": {"query": "Bitcoin price prediction 2024"},
    "observation": "Found 10 results for 'Bitcoin price prediction 2024'",
    "outcome_ref": "workspace/outputs/search_001.json"
}
```

**Rules:**

- Keep: query, result count
- Discard: all result details
- Note: If LLM needs specific results, it reads the offloaded file

### browser:navigate

```python
# Input
{"url": "https://example.com/article"}

# Full result
{
    "status": 200,
    "title": "Example Article",
    "text": "...50KB of text...",
    "links": [...100 links...]
}

# Reduced
{
    "tool": "browser:navigate",
    "minimal_args": {"url": "https://example.com/article"},
    "observation": "Visited example.com/article (status=200, 50KB text)",
    "outcome_ref": "workspace/outputs/page_001.json"
}
```

**Rules:**

- Keep: url, status, size
- Discard: page content, links
- Note: Browser content almost always needs offloading

### system:shell

```python
# Input
{"command": "git", "args": ["log", "--oneline", "-20"]}

# Full result
{
    "stdout": "abc123 fix: bug\ndef456 feat: new feature\n...20 lines...",
    "stderr": "",
    "exit_code": 0
}

# Reduced
{
    "tool": "system:shell",
    "minimal_args": {"command": "git log --oneline -20"},
    "observation": "git log: exit 0, 20 lines output",
    "outcome_ref": "workspace/outputs/shell_001.log"  # If output > 500 chars
}
```

**Rules:**

- Keep: command (joined), exit code, output line count
- Discard: stdout/stderr if > 500 chars
- Note: Error output (exit != 0) kept in observation

---

## Compaction: History Compression

> **⚠️ This section provides a simplified overview. See [COMPACTION.md](COMPACTION.md) for complete details.**

When conversations exceed 5 steps, the full per-step reduction above is not enough. We need to compress older steps into ultra-minimal summaries.

### Token Budget

```
Total context: 8192 tokens
├── System prompt + tools: 1500 tokens (fixed)
├── Few-shot examples: 500 tokens (fixed)
├── Compacted history: 500 tokens (variable)
├── Recent steps (full): 2000 tokens (last 3 steps)
├── Current goal: 200 tokens
└── Response buffer: 3492 tokens
```

### Compaction Algorithm

```python
def compact_history(steps: list[Step], token_budget: int = 500) -> str:
    """Compact old steps to fit token budget."""

    # Keep last 3 steps full
    recent = steps[-3:]
    old = steps[:-3]

    if not old:
        return format_full(recent)

    # Compact old steps
    compact_lines = []
    for step in old:
        line = TEMPLATES[step.tool_name].format(**step.__dict__)
        compact_lines.append(f"• {line}")

    compact_section = "Earlier steps:\n" + "\n".join(compact_lines)

    # Check token budget
    compact_tokens = estimate_tokens(compact_section)
    if compact_tokens > token_budget:
        # Further compress: just count
        compact_section = f"Earlier: {len(old)} steps completed"

    recent_section = format_full(recent)

    return f"{compact_section}\n\n{recent_section}"
```

### Compaction Templates

```python
TEMPLATES = {
    "fs:read_file": "Read {path} ({size})",
    "fs:write_file": "Wrote {path}",
    "fs:list_dir": "Listed {path} ({count} items)",
    "web:search": "Searched '{query}' → {count} results",
    "browser:navigate": "Visited {url}",
    "system:shell": "Ran `{command}` (exit {exit_code})",
    "weather:get": "Checked weather for {location}",
}
```

### Example

**Before compaction (1500 tokens):**

```
Step 1:
Thought: I need to search for Bitcoin price information.
Action: {"tool": "web:search", "args": {"query": "Bitcoin price"}}
Observation: {"results": [{"title": "Bitcoin Price Today", "url": "...", "snippet": "..."}, ...]}

Step 2:
Thought: Let me read the first result.
Action: {"tool": "browser:navigate", "args": {"url": "https://..."}}
Observation: {"status": 200, "text": "...5000 chars..."}

Step 3:
Thought: I should save this analysis.
Action: {"tool": "fs:write_file", "args": {"path": "analysis.md", "content": "..."}}
Observation: {"success": true, "bytes": 1234}

Step 4:
Thought: Now let me check another source.
Action: {"tool": "web:search", "args": {"query": "BTC prediction"}}
Observation: {"results": [...]}

Step 5:
Thought: I'll summarize the findings.
Action: ...
```

**After compaction (400 tokens):**

```
Earlier steps:
• Searched 'Bitcoin price' → 10 results
• Visited coindesk.com/btc (200, 5KB)
• Wrote analysis.md

Recent steps:

Step 4:
Thought: Now let me check another source.
Action: {"tool": "web:search", "args": {"query": "BTC prediction"}}
Observation: Found 8 results. Saved to workspace/outputs/search_002.json

Step 5:
Thought: I'll summarize the findings.
...
```

---

## Offloading Strategy

### When to Offload

| Content Type    | Threshold   | Action                        |
| --------------- | ----------- | ----------------------------- |
| File content    | > 1KB       | Offload to workspace/outputs/ |
| Search results  | > 5 results | Always offload                |
| Shell output    | > 500 chars | Offload to .log               |
| Browser content | Always      | Always offload                |
| Write content   | N/A         | Never store (already on disk) |

### Retrieval Pattern

When LLM needs offloaded content:

```
LLM: I need to see the search results from step 2.

Thought: The results are saved in workspace/outputs/search_001.json
Action: {"tool": "fs:read_file", "args": {"path": "workspace/outputs/search_001.json"}}
```

This is **intentional**: forces LLM to be explicit about what it needs.

---

## Implementation

### StepReducer

```python
class StepReducer:
    """Applies per-tool reduction rules."""

    def __init__(self, offloader: DataOffloader):
        self.offloader = offloader
        self.step_counter = 0

    def reduce(
        self,
        tool_name: str,
        args: dict,
        result: dict
    ) -> Step:
        self.step_counter += 1
        step_id = f"step_{self.step_counter:03d}"

        handler = getattr(self, f"_reduce_{tool_name.replace(':', '_')}", None)
        if handler:
            return handler(step_id, args, result)
        return self._reduce_default(step_id, tool_name, args, result)

    def _reduce_fs_read_file(self, step_id: str, args: dict, result: dict) -> Step:
        content = result.get("content", "")
        size = len(content)

        # Detect type
        file_type = "text"
        if args["path"].endswith(".json"):
            file_type = "JSON"

        # Maybe offload
        outcome_ref = None
        if size > 1024:
            outcome_ref = self.offloader.offload(content, step_id, "txt")

        return Step(
            id=step_id,
            tool_name="fs:read_file",
            minimal_args={"path": args["path"]},
            observation=f"Read {args['path']} ({size} bytes, {file_type})",
            outcome_ref=outcome_ref,
            success=True,
        )
```

### DataOffloader

```python
class DataOffloader:
    """Offloads large data to workspace files."""

    def __init__(self, workspace: Path):
        self.output_dir = workspace / "outputs"
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def offload(self, data: str, step_id: str, ext: str = "json") -> str:
        """Write data to file, return relative path."""
        filename = f"{step_id}.{ext}"
        path = self.output_dir / filename
        path.write_text(data)
        return f"workspace/outputs/{filename}"
```

---

## Metrics

### Token Savings

| Scenario                | Before | After | Savings |
| ----------------------- | ------ | ----- | ------- |
| 5 file reads (2KB each) | 2500   | 200   | 92%     |
| 10 web searches         | 5000   | 300   | 94%     |
| 20 step history         | 8000   | 800   | 90%     |

### Recovery Rate

All offloaded data is 100% recoverable via `fs:read_file`.

---

_See also: `DESIGN.md` for overall architecture_
