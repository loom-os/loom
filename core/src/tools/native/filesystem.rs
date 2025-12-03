use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

// ─────────────────────────────────────────────────────────────────────────────
// fs:read_file
// ─────────────────────────────────────────────────────────────────────────────

pub struct ReadFileTool {
    workspace_root: PathBuf,
}

impl ReadFileTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> String {
        "fs:read_file".to_string()
    }

    fn description(&self) -> String {
        "Read the contents of a file from the workspace".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file"
                }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let path_str = arguments["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;

        let path = self.workspace_root.join(path_str);

        // Security check: ensure path is within workspace
        if !path.starts_with(&self.workspace_root) {
            return Err(ToolError::PermissionDenied(
                "Path traversal detected".to_string(),
            ));
        }

        if !path.exists() {
            return Err(ToolError::NotFound(format!("File not found: {}", path_str)));
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        Ok(json!({
            "content": content
        }))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// fs:write_file
// ─────────────────────────────────────────────────────────────────────────────

pub struct WriteFileTool {
    workspace_root: PathBuf,
}

impl WriteFileTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> String {
        "fs:write_file".to_string()
    }

    fn description(&self) -> String {
        "Write content to a file in the workspace (creates parent directories if needed)"
            .to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let path_str = arguments["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;
        let content = arguments["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' argument".to_string()))?;

        let path = self.workspace_root.join(path_str);

        // Security check
        if !path.starts_with(&self.workspace_root) {
            return Err(ToolError::PermissionDenied(
                "Path traversal detected".to_string(),
            ));
        }

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create directories: {}", e))
            })?;
        }

        fs::write(&path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(json!({
            "success": true,
            "path": path_str,
            "bytes_written": content.len()
        }))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// fs:list_dir
// ─────────────────────────────────────────────────────────────────────────────

pub struct ListDirTool {
    workspace_root: PathBuf,
}

impl ListDirTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> String {
        "fs:list_dir".to_string()
    }

    fn description(&self) -> String {
        "List files and directories in a workspace path".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the directory (default: workspace root)"
                }
            },
            "required": []
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let path_str = arguments["path"].as_str().unwrap_or(".");
        let path = self.workspace_root.join(path_str);

        // Security check
        if !path.starts_with(&self.workspace_root) {
            return Err(ToolError::PermissionDenied(
                "Path traversal detected".to_string(),
            ));
        }

        if !path.exists() {
            return Err(ToolError::NotFound(format!(
                "Directory not found: {}",
                path_str
            )));
        }

        let mut entries = fs::read_dir(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read directory: {}", e)))?;

        let mut items = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read entry: {}", e)))?
        {
            let metadata = entry.metadata().await.ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

            items.push(json!({
                "name": name,
                "is_dir": is_dir,
                "size": size
            }));
        }

        Ok(json!({
            "path": path_str,
            "entries": items
        }))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// fs:delete
// ─────────────────────────────────────────────────────────────────────────────

pub struct DeleteFileTool {
    workspace_root: PathBuf,
}

impl DeleteFileTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> String {
        "fs:delete".to_string()
    }

    fn description(&self) -> String {
        "Delete a file or empty directory in the workspace".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to the file or directory"
                }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let path_str = arguments["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' argument".to_string()))?;

        let path = self.workspace_root.join(path_str);

        // Security check
        if !path.starts_with(&self.workspace_root) {
            return Err(ToolError::PermissionDenied(
                "Path traversal detected".to_string(),
            ));
        }

        if !path.exists() {
            return Err(ToolError::NotFound(format!("Path not found: {}", path_str)));
        }

        let is_dir = path.is_dir();
        if is_dir {
            fs::remove_dir(&path).await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to remove directory: {}", e))
            })?;
        } else {
            fs::remove_file(&path)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to remove file: {}", e)))?;
        }

        Ok(json!({
            "success": true,
            "path": path_str,
            "was_directory": is_dir
        }))
    }
}
