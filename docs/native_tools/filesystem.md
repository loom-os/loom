# Filesystem Tools

File system tools provide safe file operations within the agent's workspace.

## Security

- **Workspace Isolation**: All paths are relative to the workspace root
- **Path Traversal Protection**: Attempts to access files outside workspace are blocked
- **Human Approval**: Write and delete operations require user confirmation

---

## fs:read_file

Read the contents of a file from the workspace.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | Relative path to the file |

### Returns

```json
{
  "content": "file contents as string"
}
```

### Errors

| Error | Cause |
|-------|-------|
| `NotFound` | File does not exist |
| `PermissionDenied` | Path traversal detected |
| `ExecutionFailed` | File read error (e.g., binary file) |

### Example

```python
result = await ctx.tool("fs:read_file", {"path": "config.json"})
print(result["content"])
```

---

## fs:write_file

Write content to a file in the workspace. Creates parent directories if needed.

> ⚠️ **Requires Human Approval**

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | Relative path to the file |
| `content` | string | Yes | Content to write |

### Returns

```json
{
  "success": true,
  "path": "relative/path/to/file.txt",
  "bytes_written": 1234
}
```

### Errors

| Error | Cause |
|-------|-------|
| `PermissionDenied` | Path traversal or user denied approval |
| `ExecutionFailed` | Write error (e.g., disk full) |

### Example

```python
result = await ctx.tool("fs:write_file", {
    "path": "reports/analysis.md",
    "content": "# Analysis Report\n\nFindings..."
})
print(f"Wrote {result['bytes_written']} bytes")
```

---

## fs:list_dir

List contents of a directory in the workspace.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | No | Relative path (defaults to workspace root) |

### Returns

```json
{
  "entries": [
    {"name": "file.txt", "is_dir": false, "size": 1234},
    {"name": "subdir", "is_dir": true, "size": 0}
  ]
}
```

### Errors

| Error | Cause |
|-------|-------|
| `NotFound` | Directory does not exist |
| `PermissionDenied` | Path traversal detected |

### Example

```python
result = await ctx.tool("fs:list_dir", {"path": "data"})
for entry in result["entries"]:
    kind = "📁" if entry["is_dir"] else "📄"
    print(f"{kind} {entry['name']}")
```

---

## fs:delete

Delete a file or empty directory from the workspace.

> ⚠️ **Requires Human Approval**

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | Relative path to delete |

### Returns

```json
{
  "success": true,
  "path": "deleted/file.txt"
}
```

### Errors

| Error | Cause |
|-------|-------|
| `NotFound` | Path does not exist |
| `PermissionDenied` | Path traversal or user denied approval |
| `ExecutionFailed` | Delete error (e.g., directory not empty) |

### Example

```python
result = await ctx.tool("fs:delete", {"path": "temp/cache.json"})
print(f"Deleted: {result['path']}")
```

---

## Best Practices

1. **Use relative paths**: Always use paths relative to workspace root
2. **Check existence**: Use `fs:list_dir` to verify files exist before reading
3. **Handle errors**: Wrap tool calls in try/except for graceful error handling
4. **Batch operations**: For multiple files, consider using shell commands like `find` or `grep`
