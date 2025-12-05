# Shell Tool

Execute shell commands with safety controls.

## Security Model

The shell tool uses an **allowlist** approach:
- **Allowed commands**: Execute immediately without prompts
- **Unknown commands**: Require human approval via interactive prompt

## system:shell

Execute a shell command.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | The command to execute |
| `args` | array | No | Arguments for the command |

### Returns

```json
{
  "stdout": "command output",
  "stderr": "error output if any",
  "exit_code": 0
}
```

### Errors

| Error | Cause |
|-------|-------|
| `PermissionDenied` | Command not in allowlist and user denied |
| `ExecutionFailed` | Command execution failed |

### Example

```python
# List files
result = await ctx.tool("system:shell", {
    "command": "ls",
    "args": ["-la", "src/"]
})
print(result["stdout"])

# Search in files
result = await ctx.tool("system:shell", {
    "command": "grep",
    "args": ["-r", "TODO", "."]
})
```

---

## Allowed Commands

The following commands are pre-approved and execute without user confirmation:

### File Listing & Navigation
| Command | Description |
|---------|-------------|
| `ls` | List directory contents |
| `pwd` | Print working directory |
| `find` | Search for files |
| `which` | Locate a command |
| `whereis` | Locate binary, source, manual |
| `file` | Determine file type |
| `stat` | Display file status |
| `realpath` | Print resolved path |
| `readlink` | Print symlink target |
| `basename` | Strip directory from path |
| `dirname` | Strip filename from path |

### File Content Reading
| Command | Description |
|---------|-------------|
| `cat` | Concatenate and print files |
| `head` | Output first lines |
| `tail` | Output last lines |
| `less` | View file contents |
| `more` | View file contents |
| `wc` | Word, line, byte counts |

### Text Search & Processing
| Command | Description |
|---------|-------------|
| `grep` | Search text patterns |
| `awk` | Pattern scanning |
| `sed` | Stream editor |
| `sort` | Sort lines |
| `uniq` | Report/filter repeated lines |
| `cut` | Remove sections from lines |
| `tr` | Translate characters |
| `diff` | Compare files |

### System Info (Read-only)
| Command | Description |
|---------|-------------|
| `echo` | Display text |
| `date` | Display date/time |
| `whoami` | Print username |
| `hostname` | Print hostname |
| `uname` | Print system info |
| `env` | Print environment |
| `printenv` | Print environment variables |
| `df` | Disk free space |
| `du` | Disk usage |
| `free` | Memory usage |
| `uptime` | System uptime |
| `ps` | Process status |
| `top` | Process monitor |
| `htop` | Interactive process viewer |

### Network Info (Read-only)
| Command | Description |
|---------|-------------|
| `ping` | Test network connectivity |
| `curl` | Transfer data from URL |
| `wget` | Download files |
| `nslookup` | DNS lookup |
| `dig` | DNS lookup |
| `host` | DNS lookup |
| `ifconfig` | Network interfaces |
| `ip` | Network configuration |
| `netstat` | Network statistics |
| `ss` | Socket statistics |

### Development Tools
| Command | Description |
|---------|-------------|
| `git` | Version control |
| `python` | Python interpreter |
| `python3` | Python 3 interpreter |
| `node` | Node.js |
| `npm` | Node package manager |
| `cargo` | Rust package manager |
| `rustc` | Rust compiler |
| `make` | Build automation |
| `cmake` | Build system generator |

---

## Human-in-the-Loop

Commands not in the allowlist trigger a permission prompt:

```
⚠️  Permission Required
Tool: system:shell
Args: {'command': 'rm', 'args': ['-rf', 'temp/']}
Reason: Command 'rm' is not allowed
──────────────────────────────────────────────────
Allow this action? [y/N]:
```

If approved, the command executes via Python's subprocess (bypassing the Rust sandbox).

---

## Best Practices

1. **Prefer allowed commands**: Use `grep`, `find`, `cat` instead of custom scripts
2. **Use arguments array**: Pass args separately for proper escaping
3. **Check exit codes**: Non-zero exit codes indicate errors
4. **Handle stderr**: Important error messages appear in stderr
5. **Avoid destructive commands**: `rm`, `mv`, `chmod` require approval

### Common Patterns

```python
# Find Python files
result = await ctx.tool("system:shell", {
    "command": "find",
    "args": [".", "-name", "*.py", "-type", "f"]
})

# Count lines of code
result = await ctx.tool("system:shell", {
    "command": "wc",
    "args": ["-l", "src/*.py"]
})

# Git status
result = await ctx.tool("system:shell", {
    "command": "git",
    "args": ["status", "--short"]
})

# Check disk usage
result = await ctx.tool("system:shell", {
    "command": "du",
    "args": ["-sh", "."]
})
```
