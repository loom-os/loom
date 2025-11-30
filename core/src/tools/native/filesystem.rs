use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

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
