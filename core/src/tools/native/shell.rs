use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

pub struct ShellTool {
    allowed_commands: Vec<String>,
}

impl ShellTool {
    pub fn new(allowed_commands: Vec<String>) -> Self {
        Self { allowed_commands }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> String {
        "system:shell".to_string()
    }

    fn description(&self) -> String {
        "Execute a shell command".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Arguments for the command"
                }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let command_name = arguments["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command'".to_string()))?;

        let args = arguments["args"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !self.allowed_commands.contains(&command_name.to_string()) {
            return Err(ToolError::PermissionDenied(format!(
                "Command '{}' is not allowed",
                command_name
            )));
        }

        let output = Command::new(command_name)
            .args(&args)
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code()
        }))
    }
}
