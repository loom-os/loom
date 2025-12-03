//! Unit tests for filesystem tools

use loom_core::tools::native::{DeleteFileTool, ListDirTool, ReadFileTool, WriteFileTool};
use loom_core::tools::Tool;
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

fn create_temp_workspace() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

#[tokio::test]
async fn test_write_file_creates_file() {
    let workspace = create_temp_workspace();
    let tool = WriteFileTool::new(workspace.path().to_path_buf());

    let result = tool
        .call(json!({
            "path": "test.txt",
            "content": "Hello, World!"
        }))
        .await;

    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["success"], true);
    assert_eq!(value["bytes_written"], 13);

    // Verify file exists
    let content = fs::read_to_string(workspace.path().join("test.txt"))
        .await
        .unwrap();
    assert_eq!(content, "Hello, World!");
}

#[tokio::test]
async fn test_write_file_creates_parent_dirs() {
    let workspace = create_temp_workspace();
    let tool = WriteFileTool::new(workspace.path().to_path_buf());

    let result = tool
        .call(json!({
            "path": "reports/2024/test.md",
            "content": "# Report"
        }))
        .await;

    assert!(result.is_ok());

    // Verify nested file exists
    let content = fs::read_to_string(workspace.path().join("reports/2024/test.md"))
        .await
        .unwrap();
    assert_eq!(content, "# Report");
}

#[tokio::test]
async fn test_list_dir_lists_files() {
    let workspace = create_temp_workspace();

    // Create some files
    fs::write(workspace.path().join("a.txt"), "a")
        .await
        .unwrap();
    fs::write(workspace.path().join("b.txt"), "b")
        .await
        .unwrap();
    fs::create_dir(workspace.path().join("subdir"))
        .await
        .unwrap();

    let tool = ListDirTool::new(workspace.path().to_path_buf());
    let result = tool.call(json!({})).await;

    assert!(result.is_ok());
    let value = result.unwrap();
    let entries = value["entries"].as_array().unwrap();

    assert_eq!(entries.len(), 3);

    // Check we have both files and the directory
    let names: Vec<&str> = entries
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"a.txt"));
    assert!(names.contains(&"b.txt"));
    assert!(names.contains(&"subdir"));
}

#[tokio::test]
async fn test_delete_file() {
    let workspace = create_temp_workspace();

    // Create a file
    let file_path = workspace.path().join("to_delete.txt");
    fs::write(&file_path, "delete me").await.unwrap();
    assert!(file_path.exists());

    let tool = DeleteFileTool::new(workspace.path().to_path_buf());
    let result = tool.call(json!({"path": "to_delete.txt"})).await;

    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["success"], true);
    assert_eq!(value["was_directory"], false);

    // Verify file is gone
    assert!(!file_path.exists());
}

#[tokio::test]
async fn test_delete_empty_dir() {
    let workspace = create_temp_workspace();

    // Create an empty directory
    let dir_path = workspace.path().join("empty_dir");
    fs::create_dir(&dir_path).await.unwrap();
    assert!(dir_path.exists());

    let tool = DeleteFileTool::new(workspace.path().to_path_buf());
    let result = tool.call(json!({"path": "empty_dir"})).await;

    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["success"], true);
    assert_eq!(value["was_directory"], true);

    // Verify directory is gone
    assert!(!dir_path.exists());
}

#[tokio::test]
async fn test_path_traversal_blocked() {
    let workspace = create_temp_workspace();
    let write_tool = WriteFileTool::new(workspace.path().to_path_buf());

    // Try to write outside workspace
    let result = write_tool
        .call(json!({
            "path": "../../../etc/passwd",
            "content": "hacked"
        }))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_write_roundtrip() {
    let workspace = create_temp_workspace();
    let write_tool = WriteFileTool::new(workspace.path().to_path_buf());
    let read_tool = ReadFileTool::new(workspace.path().to_path_buf());

    // Write
    let content = "测试中文内容\nLine 2\nLine 3";
    write_tool
        .call(json!({
            "path": "test.txt",
            "content": content
        }))
        .await
        .unwrap();

    // Read back
    let result = read_tool.call(json!({"path": "test.txt"})).await.unwrap();
    assert_eq!(result["content"].as_str().unwrap(), content);
}
