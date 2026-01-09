//! Tests for final_output blocking when TODO items are incomplete in autonomous mode
//!
//! This test verifies that:
//! 1. In autonomous mode: final_output rejects completion when there are incomplete TODO items
//! 2. In non-autonomous mode: final_output always succeeds (no TODO check)

use g3_config::Config;
use g3_core::ui_writer::NullUiWriter;
use g3_core::Agent;
use serial_test::serial;
use tempfile::TempDir;

/// Helper to create a test agent in NON-autonomous mode (interactive/chat mode)
async fn create_non_autonomous_agent(temp_dir: &TempDir) -> Agent<NullUiWriter> {
    std::env::set_current_dir(temp_dir.path()).unwrap();
    let config = Config::default();
    // new_with_readme_and_quiet creates a NON-autonomous agent (is_autonomous = false)
    Agent::new_with_readme_and_quiet(config, NullUiWriter, None, true)
        .await
        .unwrap()
}

/// Helper to create a test agent in AUTONOMOUS mode (agent mode)
async fn create_autonomous_agent(temp_dir: &TempDir) -> Agent<NullUiWriter> {
    std::env::set_current_dir(temp_dir.path()).unwrap();
    let config = Config::default();
    // new_autonomous_with_readme_and_quiet creates an AUTONOMOUS agent (is_autonomous = true)
    Agent::new_autonomous_with_readme_and_quiet(config, NullUiWriter, None, true)
        .await
        .unwrap()
}

/// Helper to simulate a tool call
fn create_tool_call(tool: &str, args: serde_json::Value) -> g3_core::ToolCall {
    g3_core::ToolCall {
        tool: tool.to_string(),
        args,
    }
}

// =============================================================================
// AUTONOMOUS MODE TESTS - TODO check IS enforced
// =============================================================================

#[tokio::test]
#[serial]
async fn test_autonomous_final_output_blocked_with_incomplete_todos() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_autonomous_agent(&temp_dir).await;

    // First, write a TODO list with incomplete items
    let todo_content = "- [ ] Phase 1: Setup\n  - [x] Create files\n  - [ ] Configure settings\n- [ ] Phase 2: Implementation";
    let write_args = serde_json::json!({ "content": todo_content });
    let write_call = create_tool_call("todo_write", write_args);
    let write_result = agent.execute_tool(&write_call).await.unwrap();
    assert!(write_result.contains("TODO list updated"), "Expected TODO write to succeed");

    // Now try to call final_output - it should be rejected in autonomous mode
    let final_args = serde_json::json!({ "summary": "Completed phase 1" });
    let final_call = create_tool_call("final_output", final_args);
    let final_result = agent.execute_tool(&final_call).await.unwrap();

    // Verify that final_output was rejected due to incomplete TODOs
    assert!(
        final_result.contains("incomplete TODO"),
        "Expected final_output to be rejected in autonomous mode when TODOs are incomplete. Got: {}",
        final_result
    );
}

#[tokio::test]
#[serial]
async fn test_autonomous_final_output_allowed_with_complete_todos() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_autonomous_agent(&temp_dir).await;

    // Write a TODO list with ALL items complete
    let todo_content = "- [x] Phase 1: Setup\n  - [x] Create files\n  - [x] Configure settings\n- [x] Phase 2: Implementation";
    let write_args = serde_json::json!({ "content": todo_content });
    let write_call = create_tool_call("todo_write", write_args);
    let _write_result = agent.execute_tool(&write_call).await.unwrap();

    // Now try to call final_output - it should succeed
    let final_args = serde_json::json!({ "summary": "All phases completed successfully" });
    let final_call = create_tool_call("final_output", final_args);
    let final_result = agent.execute_tool(&final_call).await.unwrap();

    // Verify that final_output succeeded (returns the summary)
    assert!(
        final_result.contains("All phases completed successfully"),
        "Expected final_output to return the summary in autonomous mode when all TODOs complete. Got: {}",
        final_result
    );
}

#[tokio::test]
#[serial]
async fn test_autonomous_final_output_allowed_with_no_todos() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_autonomous_agent(&temp_dir).await;

    // Don't create any TODO list - final_output should still work
    let final_args = serde_json::json!({ "summary": "Simple task completed" });
    let final_call = create_tool_call("final_output", final_args);
    let final_result = agent.execute_tool(&final_call).await.unwrap();

    // Verify that final_output succeeded
    assert!(
        final_result.contains("Simple task completed"),
        "Expected final_output to return the summary when no TODOs exist. Got: {}",
        final_result
    );
}

#[tokio::test]
#[serial]
async fn test_autonomous_final_output_blocked_with_mixed_todos() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_autonomous_agent(&temp_dir).await;

    // Write a TODO list with some complete and some incomplete items
    let todo_content = "- [x] Phase 1: Setup\n- [ ] Phase 2: Implementation\n- [x] Phase 3: Testing";
    let write_args = serde_json::json!({ "content": todo_content });
    let write_call = create_tool_call("todo_write", write_args);
    let _write_result = agent.execute_tool(&write_call).await.unwrap();

    // Try to call final_output - should be rejected
    let final_args = serde_json::json!({ "summary": "Done with phases 1 and 3" });
    let final_call = create_tool_call("final_output", final_args);
    let final_result = agent.execute_tool(&final_call).await.unwrap();

    // Verify rejection
    assert!(
        final_result.contains("incomplete TODO"),
        "Expected final_output to be rejected with mixed TODOs in autonomous mode. Got: {}",
        final_result
    );
}

// =============================================================================
// NON-AUTONOMOUS MODE TESTS - TODO check is NOT enforced
// =============================================================================

#[tokio::test]
#[serial]
async fn test_non_autonomous_final_output_allowed_with_incomplete_todos() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_non_autonomous_agent(&temp_dir).await;

    // Write a TODO list with incomplete items
    let todo_content = "- [ ] Phase 1: Setup\n  - [x] Create files\n  - [ ] Configure settings\n- [ ] Phase 2: Implementation";
    let write_args = serde_json::json!({ "content": todo_content });
    let write_call = create_tool_call("todo_write", write_args);
    let write_result = agent.execute_tool(&write_call).await.unwrap();
    assert!(write_result.contains("TODO list updated"), "Expected TODO write to succeed");

    // In non-autonomous mode, final_output should succeed even with incomplete TODOs
    let final_args = serde_json::json!({ "summary": "Partial completion is fine in interactive mode" });
    let final_call = create_tool_call("final_output", final_args);
    let final_result = agent.execute_tool(&final_call).await.unwrap();

    // Verify that final_output succeeded (returns the summary, not a rejection)
    assert!(
        final_result.contains("Partial completion is fine in interactive mode"),
        "Expected final_output to succeed in non-autonomous mode even with incomplete TODOs. Got: {}",
        final_result
    );
    assert!(
        !final_result.contains("incomplete TODO"),
        "Expected NO rejection message in non-autonomous mode. Got: {}",
        final_result
    );
}

#[tokio::test]
#[serial]
async fn test_non_autonomous_final_output_allowed_with_mixed_todos() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_non_autonomous_agent(&temp_dir).await;

    // Write a TODO list with mixed complete/incomplete items
    let todo_content = "- [x] Phase 1: Setup\n- [ ] Phase 2: Implementation\n- [x] Phase 3: Testing";
    let write_args = serde_json::json!({ "content": todo_content });
    let write_call = create_tool_call("todo_write", write_args);
    let _write_result = agent.execute_tool(&write_call).await.unwrap();

    // In non-autonomous mode, final_output should succeed
    let final_args = serde_json::json!({ "summary": "Interactive mode allows partial completion" });
    let final_call = create_tool_call("final_output", final_args);
    let final_result = agent.execute_tool(&final_call).await.unwrap();

    // Verify success
    assert!(
        final_result.contains("Interactive mode allows partial completion"),
        "Expected final_output to succeed in non-autonomous mode. Got: {}",
        final_result
    );
}
