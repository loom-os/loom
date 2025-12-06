# Offload File Management

## Overview

When context engineering offloads large tool outputs to files, these files are stored in `.loom/cache/` directory. This document explains how to view, manage, and clean up these files.

> 📖 **See also:** [LIFECYCLE.md](LIFECYCLE.md) for the complete 8-phase lifecycle design (creation → retrieval → expiration → GC → archival)

## File Location

Offloaded files are stored in workspace-relative paths:

```
<workspace>/
└── .loom/
    └── cache/
        ├── search/          # Web search results
        │   └── websearch_<timestamp>_<hash>.json
        ├── file_read/       # Large file contents
        │   └── file_<name>_<hash>.txt
        ├── shell_output/    # Command outputs
        │   └── shell_<timestamp>_<hash>.txt
        └── web/             # HTTP responses
            └── web_<url>_<hash>.html
```

## Viewing Offloaded Files

When a file is offloaded, the CLI shows:

```
🔧 Calling tool: web:search
   ✅ Result:
      📄 Offloaded to: .loom/cache/search/websearch_1765003729_8a51eb3e498b367b.json
      💡 Summary: Search 'Google AI pricing' → 23 matches
      📖 View with: cat .loom/cache/search/websearch_1765003729_8a51eb3e498b367b.json
```

### Quick View Commands

```bash
# View most recent offloaded file
ls -lt .loom/cache/*/*.* | head -1 | awk '{print $NF}' | xargs cat

# View specific offloaded file (copy path from CLI output)
cat .loom/cache/search/websearch_1765003729_8a51eb3e498b367b.json

# Pretty print JSON
cat .loom/cache/search/websearch_*.json | jq .

# Search within offloaded files
grep -r "keyword" .loom/cache/

# Count offloaded files
find .loom/cache -type f | wc -l
```

## File Lifecycle

### When Files are Created

Files are created when tool output exceeds thresholds:

- **Size**: > 2KB (default)
- **Lines**: > 50 lines (default)

### File Naming

Files use content-based hashing for deduplication:

```
<category>_<timestamp>_<content_hash>.<ext>
```

Example:

```
websearch_1765003729_8a51eb3e498b367b.json
└─┬─────┘ └────┬─────┘ └───────┬────────┘ └┬┘
  │            │                │            └─ Extension
  │            │                └─ Content hash (first 16 chars)
  │            └─ Unix timestamp
  └─ Category (search, file_read, shell, etc.)
```

## Cleanup Strategies

### Manual Cleanup

```bash
# Remove all cached files
rm -rf .loom/cache/*

# Remove old files (older than 7 days)
find .loom/cache -type f -mtime +7 -delete

# Remove files from specific category
rm -rf .loom/cache/search/*

# Remove large files only (>1MB)
find .loom/cache -type f -size +1M -delete
```

### Automatic Cleanup (Future)

Currently, offloaded files are **not automatically cleaned up**. They persist until manually deleted.

**Planned features**:

- Auto-cleanup on agent restart (configurable)
- Max cache size limit (e.g., 100MB)
- LRU eviction when cache is full
- Per-category retention policies

**Configuration** (coming soon):

```toml
# loom.toml
[agents.chat-assistant.cache]
enabled = true
max_size_mb = 100
max_age_days = 7
cleanup_on_start = false
```

## Workspace Integration

### `.gitignore` Setup

Add to your `.gitignore`:

```gitignore
# Loom cache
.loom/cache/
```

### Workspace Structure

Recommended:

```
my-agent-workspace/
├── .env                    # API keys
├── loom.toml              # Agent config
├── .loom/
│   └── cache/             # Auto-generated, ignored by git
│       └── ...
├── agents/
│   └── my_agent.py
└── workspace/             # User files (committed)
    ├── documents/
    └── reports/
```

## Cache Statistics

### View Cache Size

```bash
# Total cache size
du -sh .loom/cache

# Size by category
du -sh .loom/cache/*/

# File count by category
find .loom/cache -type f | grep -o '[^/]*/[^/]*$' | cut -d/ -f1 | sort | uniq -c
```

### Example Output

```bash
$ du -sh .loom/cache
12M     .loom/cache

$ du -sh .loom/cache/*/
8.2M    .loom/cache/search/
2.1M    .loom/cache/file_read/
1.5M    .loom/cache/shell_output/
```

## Best Practices

### Development

1. **Keep cache during development** for faster iteration
2. **Clean before commit** to avoid accidental commits
3. **Use .gitignore** to prevent cache from being tracked

### Production

1. **Set max cache size** to prevent disk space issues
2. **Monitor cache growth** especially for long-running agents
3. **Implement cleanup strategy** based on usage patterns

### Debugging

1. **Check offloaded files** when tool outputs seem truncated
2. **Verify file paths** are workspace-relative
3. **Inspect content hashes** for deduplication debugging

## Troubleshooting

### "Cannot find offloaded file"

**Symptom**: LLM references offloaded file but file doesn't exist

**Causes**:

1. File was manually deleted
2. Workspace directory changed
3. Cache was cleared

**Solution**:

```bash
# Check if file exists
ls -la .loom/cache/search/websearch_*.json

# Regenerate by rerunning the tool
# (Agent will create new offloaded file)
```

### "Cache directory growing too large"

**Symptom**: `.loom/cache/` exceeds expected size

**Solution**:

```bash
# Find largest files
find .loom/cache -type f -exec du -h {} \; | sort -rh | head -20

# Remove old search results (usually safe)
find .loom/cache/search -mtime +1 -delete

# Nuclear option: clear entire cache
rm -rf .loom/cache/*
```

### "Duplicate files being created"

**Symptom**: Multiple files with same content

**Cause**: Content hash collision (very rare) or timestamp variation

**Solution**:

- This is expected behavior for same content at different times
- Use content hash to identify true duplicates
- Manual deduplication:

```bash
# Find duplicate hashes
find .loom/cache -type f -name "*_*_*.json" | \
  grep -o '_[a-f0-9]\{16\}\.' | \
  sort | uniq -d
```

## API Reference

### OffloadConfig

```python
from loom.context import DataOffloader, OffloadConfig

offloader = DataOffloader(
    workspace_path=Path("workspace"),
    config=OffloadConfig(
        enabled=True,           # Enable offloading
        size_threshold=2048,    # Bytes (2KB)
        line_threshold=50,      # Lines
    )
)
```

### Manual Offload

```python
result = offloader.offload(
    content="large content...",
    category="custom",
    identifier="my_data",
)

if result.offloaded:
    print(f"Saved to: {result.file_path}")
    print(f"Preview: {result.content}")
```

## See Also

- [Context Engineering Design](context/DESIGN.md)
- [Offloading Guide](context/OFFLOADING.md)
- [CLI Guide](CLI_GUIDE.md)
