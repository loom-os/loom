use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Stdio};

/// Validate that a path is safe (relative, no parent directory traversal)
fn validate_path(path: &str) -> ToolResult<()> {
    if path.contains("..") || path.starts_with('/') {
        return Err(ToolError::InvalidArguments(
            "Path must be relative and cannot contain '..'".into(),
        ));
    }
    Ok(())
}

/// Git operations tool for repository management in benchmarks and workflows
///
/// Provides safe git operations with path validation and security checks.
/// All paths must be relative to prevent directory traversal attacks.
///
/// # Examples
///
/// Clone a repository:
/// ```json
/// {
///   "operation": "clone",
///   "url": "https://github.com/user/repo.git",
///   "path": "my-repo",
///   "branch": "main"
/// }
/// ```
///
/// Checkout a commit:
/// ```json
/// {
///   "operation": "checkout",
///   "repo_path": "my-repo",
///   "commit": "abc123"
/// }
/// ```
pub struct GitTool;

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitTool {
    pub fn new() -> Self {
        Self
    }
    /// Clone a git repository
    pub fn clone(url: &str, path: &str, branch: Option<&str>) -> ToolResult<serde_json::Value> {
        // Validate path is safe
        validate_path(path)?;

        let mut cmd = Command::new("git");
        cmd.arg("clone");

        if let Some(b) = branch {
            cmd.arg("--branch").arg(b);
        }

        cmd.arg("--depth").arg("1"); // Shallow clone by default
        cmd.arg(url);
        cmd.arg(path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    Ok(json!({
                        "success": true,
                        "path": path,
                        "message": format!("Cloned repository to {}", path),
                        "output": stdout.to_string()
                    }))
                } else {
                    Err(ToolError::ExecutionFailed(format!(
                        "Git clone failed: {}",
                        stderr
                    )))
                }
            }
            Err(e) => Err(ToolError::ExecutionFailed(format!(
                "Failed to execute git: {}",
                e
            ))),
        }
    }

    /// Checkout a specific commit
    pub fn checkout(repo_path: &str, commit: &str) -> ToolResult<serde_json::Value> {
        // Validate path is safe
        validate_path(repo_path)?;

        let path = Path::new(repo_path);
        if !path.exists() || !path.is_dir() {
            return Err(ToolError::InvalidArguments(format!(
                "Repository path does not exist: {}",
                repo_path
            )));
        }

        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("checkout")
            .arg(commit)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Git checkout failed: {}", e)))?;

        if output.status.success() {
            Ok(json!({
                "success": true,
                "commit": commit,
                "message": format!("Checked out commit {}", commit)
            }))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::ExecutionFailed(format!(
                "Git checkout failed: {}",
                stderr
            )))
        }
    }

    /// Apply a patch file
    pub fn apply_patch(repo_path: &str, patch_content: &str) -> ToolResult<serde_json::Value> {
        // Validate path is safe
        validate_path(repo_path)?;

        let path = Path::new(repo_path);
        if !path.exists() || !path.is_dir() {
            return Err(ToolError::InvalidArguments(format!(
                "Repository path does not exist: {}",
                repo_path
            )));
        }

        // Use system temp directory for patch file
        let temp_dir = std::env::temp_dir();
        let patch_file = temp_dir.join(format!("loom_patch_{}.patch", std::process::id()));

        std::fs::write(&patch_file, patch_content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write patch: {}", e)))?;

        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("apply")
            .arg(&patch_file)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Git apply failed: {}", e)))?;

        // Clean up temp file, log warning if it fails
        if let Err(e) = std::fs::remove_file(&patch_file) {
            eprintln!(
                "Warning: Failed to clean up temp patch file {:?}: {}",
                patch_file, e
            );
        }

        if output.status.success() {
            Ok(json!({
                "success": true,
                "message": "Patch applied successfully"
            }))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::ExecutionFailed(format!(
                "Git apply failed: {}",
                stderr
            )))
        }
    }

    /// Get current commit hash
    pub fn current_commit(repo_path: &str) -> ToolResult<serde_json::Value> {
        // Validate path is safe
        validate_path(repo_path)?;

        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Git rev-parse failed: {}", e)))?;

        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(json!({
                "success": true,
                "commit": commit
            }))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ToolError::ExecutionFailed(format!(
                "Git rev-parse failed: {}",
                stderr
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_rejects_absolute_path() {
        let result = GitTool::clone("https://github.com/test/repo.git", "/tmp/repo", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_clone_rejects_parent_traversal() {
        let result = GitTool::clone("https://github.com/test/repo.git", "../repo", None);
        assert!(result.is_err());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool trait implementation
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> String {
        "git".to_string()
    }

    fn description(&self) -> String {
        "Perform git operations like clone, checkout, and apply patches. All paths must be relative for security.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["clone", "checkout", "apply_patch", "current_commit"],
                    "description": "Git operation to perform"
                },
                "url": {
                    "type": "string",
                    "description": "Repository URL (for clone operation)"
                },
                "path": {
                    "type": "string",
                    "description": "Local path for cloned repository (must be relative)"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch to clone (optional, for clone operation)"
                },
                "repo_path": {
                    "type": "string",
                    "description": "Path to repository (for checkout, apply_patch, current_commit)"
                },
                "commit": {
                    "type": "string",
                    "description": "Commit hash or branch to checkout"
                },
                "patch_content": {
                    "type": "string",
                    "description": "Patch content to apply (for apply_patch)"
                }
            },
            "required": ["operation"],
            "examples": [
                {
                    "operation": "clone",
                    "url": "https://github.com/user/repo.git",
                    "path": "my-repo",
                    "branch": "main"
                },
                {
                    "operation": "checkout",
                    "repo_path": "my-repo",
                    "commit": "abc123def"
                },
                {
                    "operation": "current_commit",
                    "repo_path": "my-repo"
                }
            ]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let operation = arguments["operation"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'operation' field".into()))?;

        match operation {
            "clone" => {
                let url = arguments["url"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' for clone".into()))?;
                let path = arguments["path"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'path' for clone".into())
                })?;
                let branch = arguments["branch"].as_str();

                Self::clone(url, path, branch)
            }
            "checkout" => {
                let repo_path = arguments["repo_path"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'repo_path' for checkout".into())
                })?;
                let commit = arguments["commit"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'commit' for checkout".into())
                })?;

                Self::checkout(repo_path, commit)
            }
            "apply_patch" => {
                let repo_path = arguments["repo_path"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'repo_path' for apply_patch".into())
                })?;
                let patch_content = arguments["patch_content"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'patch_content' for apply_patch".into())
                })?;

                Self::apply_patch(repo_path, patch_content)
            }
            "current_commit" => {
                let repo_path = arguments["repo_path"].as_str().ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'repo_path' for current_commit".into())
                })?;

                Self::current_commit(repo_path)
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown operation '{}'. Supported: clone, checkout, apply_patch, current_commit",
                operation
            ))),
        }
    }
}
