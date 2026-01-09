//! Tests for agent session ID generation
//!
//! This test verifies that:
//! 1. Agent mode sessions use the agent name as prefix (e.g., "fowler_<hash>")
//! 2. Different agents get different session IDs even with the same task
//! 3. Regular (non-agent) sessions use the task description as prefix

use g3_config::Config;
use g3_core::ui_writer::NullUiWriter;
use g3_core::Agent;
use serial_test::serial;
use tempfile::TempDir;

/// Helper to create a test agent
async fn create_test_agent(temp_dir: &TempDir) -> Agent<NullUiWriter> {
    std::env::set_current_dir(temp_dir.path()).unwrap();
    let config = Config::default();
    Agent::new_with_readme_and_quiet(config, NullUiWriter, None, true)
        .await
        .unwrap()
}

/// Helper to create a test agent in agent mode
async fn create_agent_mode_agent(temp_dir: &TempDir, agent_name: &str) -> Agent<NullUiWriter> {
    std::env::set_current_dir(temp_dir.path()).unwrap();
    let config = Config::default();
    let mut agent = Agent::new_with_readme_and_quiet(config, NullUiWriter, None, true)
        .await
        .unwrap();
    agent.set_agent_mode(agent_name);
    agent
}

// =============================================================================
// AGENT MODE SESSION ID TESTS
// =============================================================================

#[tokio::test]
#[serial]
async fn test_agent_session_id_uses_agent_name_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_agent_mode_agent(&temp_dir, "fowler").await;

    // Trigger session ID generation
    agent.init_session_id_for_test("Test task");
    
    let session_id = agent.get_session_id();
    assert!(session_id.is_some(), "Session ID should be set after adding a message");
    
    let session_id = session_id.unwrap();
    assert!(
        session_id.starts_with("fowler_"),
        "Agent session ID should start with agent name. Got: {}",
        session_id
    );
}

#[tokio::test]
#[serial]
async fn test_different_agents_get_different_session_ids() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    
    let mut agent1 = create_agent_mode_agent(&temp_dir1, "fowler").await;
    let mut agent2 = create_agent_mode_agent(&temp_dir2, "lamport").await;
    
    // Use the exact same task description for both
    let task = "Begin your analysis and work on the current project.";
    
    agent1.init_session_id_for_test(task);
    agent2.init_session_id_for_test(task);
    
    let session_id1 = agent1.get_session_id().unwrap();
    let session_id2 = agent2.get_session_id().unwrap();
    
    // Session IDs should be different
    assert_ne!(
        session_id1, session_id2,
        "Different agents should get different session IDs even with same task"
    );
    
    // Each should have the correct prefix
    assert!(
        session_id1.starts_with("fowler_"),
        "Fowler session should start with 'fowler_'. Got: {}",
        session_id1
    );
    assert!(
        session_id2.starts_with("lamport_"),
        "Lamport session should start with 'lamport_'. Got: {}",
        session_id2
    );
}

#[tokio::test]
#[serial]
async fn test_regular_session_uses_description_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let mut agent = create_test_agent(&temp_dir).await;
    
    // Add a message with a specific description
    agent.init_session_id_for_test("implement fibonacci function");
    
    let session_id = agent.get_session_id();
    assert!(session_id.is_some(), "Session ID should be set");
    
    let session_id = session_id.unwrap();
    // Regular sessions should use the description (first 5 words, lowercased)
    assert!(
        session_id.starts_with("implement_fibonacci_function_"),
        "Regular session ID should start with description. Got: {}",
        session_id
    );
}

#[tokio::test]
#[serial]
async fn test_same_agent_different_runs_get_different_session_ids() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    
    let mut agent1 = create_agent_mode_agent(&temp_dir1, "fowler").await;
    let mut agent2 = create_agent_mode_agent(&temp_dir2, "fowler").await;
    
    // Same agent, same task
    let task = "Begin your analysis and work on the current project.";
    
    agent1.init_session_id_for_test(task);
    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(1));
    agent2.init_session_id_for_test(task);
    
    let session_id1 = agent1.get_session_id().unwrap();
    let session_id2 = agent2.get_session_id().unwrap();
    
    // Session IDs should be different due to timestamp
    assert_ne!(
        session_id1, session_id2,
        "Same agent running twice should get different session IDs"
    );
    
    // Both should have the same prefix
    assert!(session_id1.starts_with("fowler_"), "Got: {}", session_id1);
    assert!(session_id2.starts_with("fowler_"), "Got: {}", session_id2);
}
