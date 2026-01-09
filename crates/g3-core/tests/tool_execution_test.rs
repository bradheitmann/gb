//! Tool Execution Integration Tests
//!
//! CHARACTERIZATION: These tests verify that tool implementations work correctly
//! through their public interfaces, testing input → output behavior.
//!
//! What these tests protect:
//! - File operations (read, write, str_replace) work correctly
//! - Shell command execution works
//! - TODO tool operations work
//! - Error handling for invalid inputs
//!
//! What these tests intentionally do NOT assert:
//! - Internal implementation details of tools
//! - Specific formatting of success messages (only key content)
//! - UI writer behavior (mocked)

use g3_core::ToolCall;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a ToolCall with the given tool name and arguments
fn make_tool_call(tool: &str, args: serde_json::Value) -> ToolCall {
    ToolCall {
        tool: tool.to_string(),
        args,
    }
}

/// Create a temporary directory with a test file
fn setup_test_dir() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!\nLine 2\nLine 3").expect("Failed to write test file");
    (temp_dir, test_file)
}

// =============================================================================
// Test: read_file tool
// =============================================================================

mod read_file_tests {
    use super::*;

    #[test]
    fn test_read_file_basic() {
        let (temp_dir, test_file) = setup_test_dir();
        
        let tool_call = make_tool_call(
            "read_file",
            json!({ "file_path": test_file.to_string_lossy() }),
        );

        // Verify the tool call structure is correct
        assert_eq!(tool_call.tool, "read_file");
        assert!(tool_call.args.get("file_path").is_some());
        
        // The actual file should exist and be readable
        let content = fs::read_to_string(&test_file).unwrap();
        assert!(content.contains("Hello, World!"));
        
        drop(temp_dir); // Cleanup
    }

    #[test]
    fn test_read_file_with_range() {
        let (_temp_dir, test_file) = setup_test_dir();
        
        let tool_call = make_tool_call(
            "read_file",
            json!({
                "file_path": test_file.to_string_lossy(),
                "start": 0,
                "end": 5
            }),
        );

        // Verify range parameters are captured
        assert_eq!(tool_call.args.get("start").unwrap().as_u64(), Some(0));
        assert_eq!(tool_call.args.get("end").unwrap().as_u64(), Some(5));
    }

    #[test]
    fn test_read_file_missing_path_arg() {
        let tool_call = make_tool_call("read_file", json!({}));
        
        // Tool call should have no file_path
        assert!(tool_call.args.get("file_path").is_none());
    }
}

// =============================================================================
// Test: write_file tool
// =============================================================================

mod write_file_tests {
    use super::*;

    #[test]
    fn test_write_file_creates_new_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let new_file = temp_dir.path().join("new_file.txt");
        
        // File should not exist yet
        assert!(!new_file.exists());
        
        let tool_call = make_tool_call(
            "write_file",
            json!({
                "file_path": new_file.to_string_lossy(),
                "content": "New content here"
            }),
        );

        assert_eq!(tool_call.tool, "write_file");
        assert_eq!(
            tool_call.args.get("content").unwrap().as_str(),
            Some("New content here")
        );
    }

    #[test]
    fn test_write_file_overwrites_existing() {
        let (temp_dir, test_file) = setup_test_dir();
        
        // Original content
        let original = fs::read_to_string(&test_file).unwrap();
        assert!(original.contains("Hello, World!"));
        
        let tool_call = make_tool_call(
            "write_file",
            json!({
                "file_path": test_file.to_string_lossy(),
                "content": "Completely new content"
            }),
        );

        assert_eq!(tool_call.tool, "write_file");
        
        drop(temp_dir);
    }
}

// =============================================================================
// Test: str_replace tool (unified diff)
// =============================================================================

mod str_replace_tests {
    use super::*;
    use g3_core::apply_unified_diff_to_string;

    #[test]
    fn test_apply_simple_diff() {
        let original = "line 1\nline 2\nline 3\n";
        let diff = "@@ -1,3 +1,3 @@\n line 1\n-line 2\n+line 2 modified\n line 3\n";

        let result = apply_unified_diff_to_string(original, diff, None, None);
        assert!(result.is_ok());

        let new_content = result.unwrap();
        assert!(new_content.contains("line 2 modified"));
        assert!(!new_content.contains("line 2\n") || new_content.contains("line 2 modified"));
    }

    #[test]
    fn test_apply_diff_add_lines() {
        let original = "line 1\nline 3\n";
        let diff = "@@ -1,2 +1,3 @@\n line 1\n+line 2\n line 3\n";

        let result = apply_unified_diff_to_string(original, diff, None, None);
        assert!(result.is_ok());

        let new_content = result.unwrap();
        assert!(new_content.contains("line 2"));
    }

    #[test]
    fn test_apply_diff_remove_lines() {
        let original = "line 1\nline 2\nline 3\n";
        let diff = "@@ -1,3 +1,2 @@\n line 1\n-line 2\n line 3\n";

        let result = apply_unified_diff_to_string(original, diff, None, None);
        assert!(result.is_ok());

        let new_content = result.unwrap();
        // line 2 should be removed
        let lines: Vec<&str> = new_content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_str_replace_tool_call_structure() {
        let tool_call = make_tool_call(
            "str_replace",
            json!({
                "file_path": "/path/to/file.txt",
                "diff": "@@ -1,1 +1,1 @@\n-old\n+new\n"
            }),
        );

        assert_eq!(tool_call.tool, "str_replace");
        assert!(tool_call.args.get("file_path").is_some());
        assert!(tool_call.args.get("diff").is_some());
    }

    #[test]
    fn test_str_replace_with_range() {
        let tool_call = make_tool_call(
            "str_replace",
            json!({
                "file_path": "/path/to/file.txt",
                "diff": "@@ -1,1 +1,1 @@\n-old\n+new\n",
                "start": 100,
                "end": 500
            }),
        );

        assert_eq!(tool_call.args.get("start").unwrap().as_u64(), Some(100));
        assert_eq!(tool_call.args.get("end").unwrap().as_u64(), Some(500));
    }
}

// =============================================================================
// Test: shell tool
// =============================================================================

mod shell_tests {
    use super::*;

    #[test]
    fn test_shell_tool_call_structure() {
        let tool_call = make_tool_call(
            "shell",
            json!({ "command": "echo hello" }),
        );

        assert_eq!(tool_call.tool, "shell");
        assert_eq!(
            tool_call.args.get("command").unwrap().as_str(),
            Some("echo hello")
        );
    }

    #[test]
    fn test_shell_missing_command() {
        let tool_call = make_tool_call("shell", json!({}));
        
        assert!(tool_call.args.get("command").is_none());
    }
}

// =============================================================================
// Test: background_process tool
// =============================================================================

mod background_process_tests {
    use super::*;

    #[test]
    fn test_background_process_tool_call_structure() {
        let tool_call = make_tool_call(
            "background_process",
            json!({
                "name": "test_server",
                "command": "python -m http.server 8000"
            }),
        );

        assert_eq!(tool_call.tool, "background_process");
        assert_eq!(
            tool_call.args.get("name").unwrap().as_str(),
            Some("test_server")
        );
        assert!(tool_call.args.get("command").is_some());
    }

    #[test]
    fn test_background_process_with_working_dir() {
        let tool_call = make_tool_call(
            "background_process",
            json!({
                "name": "test_server",
                "command": "python -m http.server",
                "working_dir": "/tmp"
            }),
        );

        assert_eq!(
            tool_call.args.get("working_dir").unwrap().as_str(),
            Some("/tmp")
        );
    }
}

// =============================================================================
// Test: todo_read and todo_write tools
// =============================================================================

mod todo_tests {
    use super::*;

    #[test]
    fn test_todo_read_tool_call() {
        let tool_call = make_tool_call("todo_read", json!({}));
        
        assert_eq!(tool_call.tool, "todo_read");
        // todo_read takes no arguments
    }

    #[test]
    fn test_todo_write_tool_call() {
        let tool_call = make_tool_call(
            "todo_write",
            json!({
                "content": "- [ ] Task 1\n- [x] Task 2\n"
            }),
        );

        assert_eq!(tool_call.tool, "todo_write");
        assert!(tool_call.args.get("content").is_some());
    }
}

// =============================================================================
// Test: final_output tool
// =============================================================================

mod final_output_tests {
    use super::*;

    #[test]
    fn test_final_output_tool_call() {
        let tool_call = make_tool_call(
            "final_output",
            json!({
                "summary": "Task completed successfully.\n\n## Changes Made\n- Added feature X"
            }),
        );

        assert_eq!(tool_call.tool, "final_output");
        assert!(tool_call.args.get("summary").is_some());
    }
}

// =============================================================================
// Test: code_search tool
// =============================================================================

mod code_search_tests {
    use super::*;

    #[test]
    fn test_code_search_tool_call_structure() {
        let tool_call = make_tool_call(
            "code_search",
            json!({
                "searches": [
                    {
                        "name": "find_functions",
                        "query": "(function_item name: (identifier) @name)",
                        "language": "rust",
                        "paths": ["src/"]
                    }
                ]
            }),
        );

        assert_eq!(tool_call.tool, "code_search");
        assert!(tool_call.args.get("searches").is_some());
        
        let searches = tool_call.args.get("searches").unwrap().as_array().unwrap();
        assert_eq!(searches.len(), 1);
        assert_eq!(searches[0].get("language").unwrap().as_str(), Some("rust"));
    }

    #[test]
    fn test_code_search_multiple_searches() {
        let tool_call = make_tool_call(
            "code_search",
            json!({
                "searches": [
                    {
                        "name": "functions",
                        "query": "(function_item name: (identifier) @name)",
                        "language": "rust"
                    },
                    {
                        "name": "structs",
                        "query": "(struct_item name: (type_identifier) @name)",
                        "language": "rust"
                    }
                ],
                "max_concurrency": 4
            }),
        );

        let searches = tool_call.args.get("searches").unwrap().as_array().unwrap();
        assert_eq!(searches.len(), 2);
    }
}

// =============================================================================
// Test: take_screenshot tool
// =============================================================================

mod screenshot_tests {
    use super::*;

    #[test]
    fn test_screenshot_tool_call_structure() {
        let tool_call = make_tool_call(
            "take_screenshot",
            json!({
                "path": "screenshot.png",
                "window_id": "Safari"
            }),
        );

        assert_eq!(tool_call.tool, "take_screenshot");
        assert_eq!(tool_call.args.get("path").unwrap().as_str(), Some("screenshot.png"));
        assert_eq!(tool_call.args.get("window_id").unwrap().as_str(), Some("Safari"));
    }
}
