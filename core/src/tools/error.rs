use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ToolResult<T> = Result<T, ToolError>;
