//! CLI Integration Tests (Blackbox)
//!
//! CHARACTERIZATION: These tests verify the CLI's external behavior through
//! its public interface (command-line arguments and exit codes).
//!
//! What these tests protect:
//! - CLI argument parsing works correctly
//! - Help and version output are available
//! - Invalid arguments produce appropriate errors
//! - Workspace directory handling works
//!
//! What these tests intentionally do NOT assert:
//! - Internal implementation details
//! - Specific error message wording (only that errors occur)
//! - Provider-specific behavior (requires API keys)

use std::process::Command;

/// Get the path to the g3 binary.
/// In test mode, this will be in the target/debug directory.
fn get_g3_binary() -> String {
    // When running tests, the binary is in target/debug/
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove deps
    path.push("g3");
    path.to_string_lossy().to_string()
}

// =============================================================================
// Test: --help flag produces help output
// =============================================================================

#[test]
fn test_help_flag_produces_output() {
    let output = Command::new(get_g3_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute g3 --help");

    // Help should succeed
    assert!(
        output.status.success(),
        "g3 --help should exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain key elements of help output
    assert!(
        stdout.contains("Usage:"),
        "Help output should contain 'Usage:'"
    );
    assert!(
        stdout.contains("Options:"),
        "Help output should contain 'Options:'"
    );
    assert!(
        stdout.contains("--help"),
        "Help output should mention --help flag"
    );
    assert!(
        stdout.contains("--version"),
        "Help output should mention --version flag"
    );
}

#[test]
fn test_short_help_flag() {
    let output = Command::new(get_g3_binary())
        .arg("-h")
        .output()
        .expect("Failed to execute g3 -h");

    assert!(output.status.success(), "g3 -h should exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:"),
        "Short help should also show usage"
    );
}

// =============================================================================
// Test: --version flag produces version output
// =============================================================================

#[test]
fn test_version_flag_produces_output() {
    let output = Command::new(get_g3_binary())
        .arg("--version")
        .output()
        .expect("Failed to execute g3 --version");

    assert!(
        output.status.success(),
        "g3 --version should exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain version number pattern (e.g., "g3 0.1.0")
    assert!(
        stdout.contains("g3") || stdout.contains("0."),
        "Version output should contain program name or version number"
    );
}

#[test]
fn test_short_version_flag() {
    let output = Command::new(get_g3_binary())
        .arg("-V")
        .output()
        .expect("Failed to execute g3 -V");

    assert!(output.status.success(), "g3 -V should exit successfully");
}

// =============================================================================
// Test: Invalid arguments produce errors
// =============================================================================

#[test]
fn test_invalid_flag_produces_error() {
    let output = Command::new(get_g3_binary())
        .arg("--this-flag-does-not-exist")
        .output()
        .expect("Failed to execute g3 with invalid flag");

    // Should fail with non-zero exit code
    assert!(
        !output.status.success(),
        "Invalid flag should cause non-zero exit"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should have some error message
    assert!(
        !stderr.is_empty() || !output.stdout.is_empty(),
        "Should produce some output on invalid flag"
    );
}

// =============================================================================
// Test: Conflicting mode flags
// =============================================================================

#[test]
fn test_agent_conflicts_with_autonomous() {
    // --agent conflicts with --autonomous
    let output = Command::new(get_g3_binary())
        .args(["--agent", "test", "--autonomous"])
        .output()
        .expect("Failed to execute g3 with conflicting flags");

    // Should fail due to conflicting arguments
    assert!(
        !output.status.success(),
        "--agent and --autonomous should conflict"
    );
}

#[test]
fn test_planning_conflicts_with_autonomous() {
    let output = Command::new(get_g3_binary())
        .args(["--planning", "--autonomous"])
        .output()
        .expect("Failed to execute g3 with conflicting flags");

    assert!(
        !output.status.success(),
        "--planning and --autonomous should conflict"
    );
}

// =============================================================================
// Test: Flock mode requires all related flags
// =============================================================================

#[test]
fn test_flock_mode_requires_workspace() {
    let output = Command::new(get_g3_binary())
        .args(["--project", "/tmp/test"])
        .output()
        .expect("Failed to execute g3 with incomplete flock args");

    // Should fail because --flock-workspace and --segments are required
    assert!(
        !output.status.success(),
        "--project without --flock-workspace should fail"
    );
}

#[test]
fn test_flock_mode_requires_segments() {
    let output = Command::new(get_g3_binary())
        .args(["--project", "/tmp/test", "--flock-workspace", "/tmp/ws"])
        .output()
        .expect("Failed to execute g3 with incomplete flock args");

    // Should fail because --segments is required
    assert!(
        !output.status.success(),
        "--project without --segments should fail"
    );
}

// =============================================================================
// Test: Workspace directory option is accepted
// =============================================================================

#[test]
fn test_workspace_option_accepted() {
    // Just verify the option is recognized (don't actually run the agent)
    let output = Command::new(get_g3_binary())
        .args(["--workspace", "/tmp", "--help"])
        .output()
        .expect("Failed to execute g3 with workspace option");

    // --help should still work even with other options
    assert!(
        output.status.success(),
        "--workspace option should be recognized"
    );
}

// =============================================================================
// Test: Config file option is accepted
// =============================================================================

#[test]
fn test_config_option_accepted() {
    let output = Command::new(get_g3_binary())
        .args(["--config", "/nonexistent/config.toml", "--help"])
        .output()
        .expect("Failed to execute g3 with config option");

    // --help should still work
    assert!(
        output.status.success(),
        "--config option should be recognized"
    );
}

// =============================================================================
// Test: Provider override option is accepted
// =============================================================================

#[test]
fn test_provider_option_accepted() {
    let output = Command::new(get_g3_binary())
        .args(["--provider", "anthropic", "--help"])
        .output()
        .expect("Failed to execute g3 with provider option");

    assert!(
        output.status.success(),
        "--provider option should be recognized"
    );
}

// =============================================================================
// Test: Quiet mode option is accepted
// =============================================================================

#[test]
fn test_quiet_option_accepted() {
    let output = Command::new(get_g3_binary())
        .args(["--quiet", "--help"])
        .output()
        .expect("Failed to execute g3 with quiet option");

    assert!(
        output.status.success(),
        "--quiet option should be recognized"
    );
}

// =============================================================================
// Test: Machine mode option is accepted
// =============================================================================

#[test]
fn test_machine_option_accepted() {
    let output = Command::new(get_g3_binary())
        .args(["--machine", "--help"])
        .output()
        .expect("Failed to execute g3 with machine option");

    assert!(
        output.status.success(),
        "--machine option should be recognized"
    );
}
