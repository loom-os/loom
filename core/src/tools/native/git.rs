use serde_json::json;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::tools::error::{ToolError, ToolResult};

/// Git operations tool for repository management
pub struct GitTool;

impl GitTool {
    /// Clone a git repository
    pub fn clone(url: &str, path: &str, branch: Option<&str>) -> ToolResult<serde_json::Value> {
        // Validate path is safe
        if path.contains("..") || path.starts_with('/') {
            return Err(ToolError::InvalidArguments(
                "Path must be relative and cannot contain '..'".into(),
            ));
        }

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
        let path = Path::new(repo_path);
        if !path.exists() || !path.is_dir() {
            return Err(ToolError::InvalidArguments(format!(
                "Repository path does not exist: {}",
                repo_path
            )));
        }

        // Write patch to temp file
        let patch_file = path.join(".loom_temp.patch");
        std::fs::write(&patch_file, patch_content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write patch: {}", e)))?;

        let output = Command::new("git")
            .current_dir(repo_path)
            .arg("apply")
            .arg(&patch_file)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Git apply failed: {}", e)))?;

        // Clean up temp file
        let _ = std::fs::remove_file(&patch_file);

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
