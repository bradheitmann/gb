//! Path utilities for G3 session and workspace management.
//!
//! This module centralizes all path-related logic for:
//! - TODO file location
//! - Logs directory
//! - Session directories and files
//! - Thinned content storage

use std::path::PathBuf;

/// Environment variable name for workspace path.
/// Used to direct all logs to the workspace directory.
pub const G3_WORKSPACE_PATH_ENV: &str = "G3_WORKSPACE_PATH";

/// Environment variable name for custom TODO file path.
const G3_TODO_PATH_ENV: &str = "G3_TODO_PATH";

/// Get the path to the todo.g3.md file.
///
/// Checks for G3_TODO_PATH environment variable first (used by planning mode),
/// then falls back to todo.g3.md in the current directory.
pub fn get_todo_path() -> PathBuf {
    if let Ok(custom_path) = std::env::var(G3_TODO_PATH_ENV) {
        PathBuf::from(custom_path)
    } else {
        std::env::current_dir().unwrap_or_default().join("todo.g3.md")
    }
}

/// Get the path to the todo.g3.md file for a specific session.
/// Returns .g3/sessions/<session_id>/todo.g3.md
pub fn get_session_todo_path(session_id: &str) -> PathBuf {
    get_session_logs_dir(session_id).join("todo.g3.md")
}

/// Get the path to the logs directory.
///
/// Checks for G3_WORKSPACE_PATH environment variable first (used by planning mode),
/// then falls back to "logs" in the current directory.
pub fn get_logs_dir() -> PathBuf {
    if let Ok(workspace_path) = std::env::var(G3_WORKSPACE_PATH_ENV) {
        PathBuf::from(workspace_path).join("logs")
    } else {
        std::env::current_dir().unwrap_or_default().join("logs")
    }
}

/// Public accessor for the logs directory path (for use by submodules).
/// Alias for `get_logs_dir()` for backward compatibility.
pub fn logs_dir() -> PathBuf {
    get_logs_dir()
}

/// Get the base .g3 directory path.
/// This is the root for all g3 session data in the current workspace.
pub fn get_g3_dir() -> PathBuf {
    if let Ok(workspace_path) = std::env::var(G3_WORKSPACE_PATH_ENV) {
        PathBuf::from(workspace_path).join(".g3")
    } else {
        std::env::current_dir().unwrap_or_default().join(".g3")
    }
}

/// Get the session directory for a specific session ID.
/// Returns .g3/sessions/<session_id>/
pub fn get_session_logs_dir(session_id: &str) -> PathBuf {
    get_g3_dir().join("sessions").join(session_id)
}

/// Ensure the session directory exists for a specific session ID.
/// Creates .g3/sessions/<session_id>/ and subdirectories.
pub fn ensure_session_dir(session_id: &str) -> std::io::Result<PathBuf> {
    let session_dir = get_session_logs_dir(session_id);
    std::fs::create_dir_all(&session_dir)?;

    // Create subdirectories
    std::fs::create_dir_all(session_dir.join("thinned"))?;

    Ok(session_dir)
}

/// Get the thinned content directory for a session.
/// Returns .g3/sessions/<session_id>/thinned/
pub fn get_thinned_dir(session_id: &str) -> PathBuf {
    get_session_logs_dir(session_id).join("thinned")
}

/// Get the path to the session.json file for a session.
/// Returns .g3/sessions/<session_id>/session.json
pub fn get_session_file(session_id: &str) -> PathBuf {
    get_session_logs_dir(session_id).join("session.json")
}

/// Get the path to the context summary file for a session.
/// Returns .g3/sessions/<session_id>/context_summary.txt
pub fn get_context_summary_file(session_id: &str) -> PathBuf {
    get_session_logs_dir(session_id).join("context_summary.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_path_default() {
        // When G3_TODO_PATH is not set, should return current_dir/todo.g3.md
        std::env::remove_var(G3_TODO_PATH_ENV);
        let path = get_todo_path();
        assert!(path.ends_with("todo.g3.md"));
    }

    #[test]
    fn test_session_paths_are_consistent() {
        let session_id = "test-session-123";
        let session_dir = get_session_logs_dir(session_id);
        let thinned_dir = get_thinned_dir(session_id);
        let session_file = get_session_file(session_id);
        let summary_file = get_context_summary_file(session_id);
        let todo_file = get_session_todo_path(session_id);

        // All paths should be under the session directory
        assert!(thinned_dir.starts_with(&session_dir));
        assert!(session_file.starts_with(&session_dir));
        assert!(summary_file.starts_with(&session_dir));
        assert!(todo_file.starts_with(&session_dir));

        // Check expected filenames
        assert!(thinned_dir.ends_with("thinned"));
        assert!(session_file.ends_with("session.json"));
        assert!(summary_file.ends_with("context_summary.txt"));
        assert!(todo_file.ends_with("todo.g3.md"));
    }
}
