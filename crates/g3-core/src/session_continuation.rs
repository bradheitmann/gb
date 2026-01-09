//! Session continuation support for long-running interactive sessions.
//!
//! This module provides functionality to save and restore session state,
//! allowing users to resume work across multiple g3 invocations.
//!
//! The session continuation uses a symlink-based approach:
//! - `.g3/session` is a symlink pointing to the current session directory
//! - `latest.json` is stored inside each session directory (`.g3/sessions/<session_id>/latest.json`)
//! - Following the symlink gives access to the current session's continuation data

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, error, warn};

/// Version of the session continuation format
const CONTINUATION_VERSION: &str = "1.0";

/// Name of the continuation file within each session directory
const CONTINUATION_FILENAME: &str = "latest.json";

/// Session continuation artifact containing all information needed to resume a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContinuation {
    /// Version of the continuation format
    pub version: String,
    /// Whether this session was running in agent mode
    pub is_agent_mode: bool,
    /// Name of the agent (e.g., "fowler", "pike") if in agent mode
    pub agent_name: Option<String>,
    /// Timestamp when the continuation was saved
    pub created_at: String,
    /// Original session ID
    pub session_id: String,
    /// The last final_output summary
    pub final_output_summary: Option<String>,
    /// Path to the full session log (g3_session_*.json)
    pub session_log_path: String,
    /// Context window usage percentage when saved
    pub context_percentage: f32,
    /// Snapshot of the TODO list content
    pub todo_snapshot: Option<String>,
    /// Working directory where the session was running
    pub working_directory: String,
}

impl SessionContinuation {
    /// Create a new session continuation artifact
    pub fn new(
        is_agent_mode: bool,
        agent_name: Option<String>,
        session_id: String,
        final_output_summary: Option<String>,
        session_log_path: String,
        context_percentage: f32,
        todo_snapshot: Option<String>,
        working_directory: String,
    ) -> Self {
        Self {
            version: CONTINUATION_VERSION.to_string(),
            is_agent_mode,
            agent_name,
            created_at: chrono::Utc::now().to_rfc3339(),
            session_id,
            final_output_summary,
            session_log_path,
            context_percentage,
            todo_snapshot,
            working_directory,
        }
    }

    /// Check if the context can be fully restored (< 80% used)
    pub fn can_restore_full_context(&self) -> bool {
        self.context_percentage < 80.0
    }

    /// Check if this session has incomplete TODO items
    pub fn has_incomplete_todos(&self) -> bool {
        match &self.todo_snapshot {
            Some(todo) => todo.contains("- [ ]"),
            None => false,
        }
    }
}

/// Get the path to the .g3 directory
fn get_g3_dir() -> PathBuf {
    crate::get_g3_dir()
}

/// Get the path to the .g3/session symlink
pub fn get_session_dir() -> PathBuf {
    get_g3_dir().join("session")
}

/// Get the path to the .g3/sessions directory (where all sessions are stored)
fn get_sessions_dir() -> PathBuf {
    get_g3_dir().join("sessions")
}

/// Get the path to a specific session's directory
fn get_session_path(session_id: &str) -> PathBuf {
    get_sessions_dir().join(session_id)
}

/// Get the path to the latest.json continuation file
/// This follows the symlink to get the actual path
pub fn get_latest_continuation_path() -> PathBuf {
    get_session_dir().join(CONTINUATION_FILENAME)
}

/// Ensure the .g3 directory exists (but not the session symlink)
pub fn ensure_session_dir() -> Result<PathBuf> {
    let g3_dir = get_g3_dir();
    if !g3_dir.exists() {
        std::fs::create_dir_all(&g3_dir)?;
        debug!("Created .g3 directory: {:?}", g3_dir);
    }
    Ok(get_session_dir())
}

/// Update the .g3/session symlink to point to the given session directory
fn update_session_symlink(session_id: &str) -> Result<()> {
    let symlink_path = get_session_dir();
    let target_path = get_session_path(session_id);
    
    // Remove existing symlink or directory if it exists
    if symlink_path.exists() || symlink_path.is_symlink() {
        if symlink_path.is_symlink() {
            std::fs::remove_file(&symlink_path)
                .context("Failed to remove existing session symlink")?;
        } else if symlink_path.is_dir() {
            // Migration: if it's an old-style directory, remove it
            std::fs::remove_dir_all(&symlink_path)
                .context("Failed to remove old session directory")?;
            debug!("Migrated old .g3/session directory to symlink");
        }
    }
    
    // Create the symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_path, &symlink_path)
        .context("Failed to create session symlink")?;
    
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&target_path, &symlink_path)
        .context("Failed to create session symlink")?;
    
    debug!("Updated session symlink: {:?} -> {:?}", symlink_path, target_path);
    Ok(())
}

/// Save a session continuation artifact
/// This saves latest.json in the session's directory and updates the symlink
pub fn save_continuation(continuation: &SessionContinuation) -> Result<PathBuf> {
    let session_id = &continuation.session_id;
    let session_path = get_session_path(session_id);
    
    // Ensure the session directory exists
    if !session_path.exists() {
        std::fs::create_dir_all(&session_path)
            .context("Failed to create session directory")?;
    }
    
    // Save latest.json in the session directory
    let latest_path = session_path.join(CONTINUATION_FILENAME);
    let json = serde_json::to_string_pretty(continuation)?;
    std::fs::write(&latest_path, &json)?;
    
    // Update the symlink to point to this session
    update_session_symlink(session_id)?;
    
    debug!("Saved session continuation to {:?}", latest_path);
    Ok(latest_path)
}

/// Load the latest session continuation artifact if it exists
pub fn load_continuation() -> Result<Option<SessionContinuation>> {
    let symlink_path = get_session_dir();
    
    // Check if the symlink exists and is valid
    if !symlink_path.is_symlink() && !symlink_path.exists() {
        debug!("No session symlink found at {:?}", symlink_path);
        return Ok(None);
    }
    
    // If it's a symlink, check if the target exists
    if symlink_path.is_symlink() {
        let target = std::fs::read_link(&symlink_path)?;
        if !target.exists() && !symlink_path.exists() {
            debug!("Session symlink target does not exist: {:?}", target);
            return Ok(None);
        }
    }
    
    let latest_path = symlink_path.join(CONTINUATION_FILENAME);
    
    if !latest_path.exists() {
        debug!("No continuation file found at {:?}", latest_path);
        return Ok(None);
    }
    
    let json = std::fs::read_to_string(&latest_path)?;
    let continuation: SessionContinuation = serde_json::from_str(&json)?;
    
    // Validate version
    if continuation.version != CONTINUATION_VERSION {
        warn!(
            "Continuation version mismatch: expected {}, got {}",
            CONTINUATION_VERSION, continuation.version
        );
    }
    
    debug!("Loaded session continuation from {:?}", latest_path);
    Ok(Some(continuation))
}

/// Clear the session continuation symlink (for /clear command)
/// This only removes the symlink, not the actual session data
pub fn clear_continuation() -> Result<()> {
    let symlink_path = get_session_dir();
    
    if symlink_path.is_symlink() {
        std::fs::remove_file(&symlink_path)?;
        debug!("Removed session symlink: {:?}", symlink_path);
    } else if symlink_path.is_dir() {
        // Handle old-style directory (migration case)
        for entry in std::fs::read_dir(&symlink_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(&path)?;
                debug!("Removed session file: {:?}", path);
            }
        }
        std::fs::remove_dir(&symlink_path)?;
        debug!("Removed old session directory: {:?}", symlink_path);
    }
    
    debug!("Cleared session continuation");
    Ok(())
}

/// Check if a continuation exists and is valid
pub fn has_valid_continuation() -> bool {
    match load_continuation() {
        Ok(Some(continuation)) => {
            // Check if the session log still exists
            let session_log_path = PathBuf::from(&continuation.session_log_path);
            if !session_log_path.exists() {
                warn!("Session log no longer exists: {:?}", session_log_path);
                return false;
            }
            
            // Check if we're in the same working directory
            let current_dir = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            
            if current_dir != continuation.working_directory {
                debug!(
                    "Working directory changed: {} -> {}",
                    continuation.working_directory, current_dir
                );
                // Still valid, but user should be aware
            }
            
            true
        }
        Ok(None) => false,
        Err(e) => {
            error!("Error checking continuation: {}", e);
            false
        }
    }
}

/// Load the full context window from a session log file
pub fn load_context_from_session_log(session_log_path: &Path) -> Result<Option<serde_json::Value>> {
    if !session_log_path.exists() {
        return Ok(None);
    }
    
    let json = std::fs::read_to_string(session_log_path)?;
    let session_data: serde_json::Value = serde_json::from_str(&json)?;
    
    Ok(Some(session_data))
}

/// Find an incomplete agent session for the given agent name.
/// Returns the most recent session that:
/// 1. Was running in agent mode with the matching agent name
/// 2. Has incomplete TODO items (contains "- [ ]")
/// 3. Is in the same working directory
pub fn find_incomplete_agent_session(agent_name: &str) -> Result<Option<SessionContinuation>> {
    let sessions_dir = get_sessions_dir();
    
    if !sessions_dir.exists() {
        debug!("Sessions directory does not exist: {:?}", sessions_dir);
        return Ok(None);
    }
    
    let current_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    
    let mut candidates: Vec<SessionContinuation> = Vec::new();
    
    // Scan all session directories
    for entry in std::fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if !path.is_dir() {
            continue;
        }
        
        // Check for latest.json in this session directory
        let latest_path = path.join(CONTINUATION_FILENAME);
        if !latest_path.exists() {
            continue;
        }
        
        // Try to load the continuation
        let json = match std::fs::read_to_string(&latest_path) {
            Ok(j) => j,
            Err(_) => continue,
        };
        
        let continuation: SessionContinuation = match serde_json::from_str(&json) {
            Ok(c) => c,
            Err(_) => continue, // Skip sessions with old format
        };
        
        // Check if this is an agent mode session with matching name
        if !continuation.is_agent_mode {
            continue;
        }
        
        if continuation.agent_name.as_deref() != Some(agent_name) {
            continue;
        }
        
        // Check if in same working directory
        if continuation.working_directory != current_dir {
            continue;
        }
        
        // Check if has incomplete TODOs (either in snapshot or in the actual file)
        let has_incomplete = if continuation.has_incomplete_todos() {
            true
        } else if continuation.todo_snapshot.is_none() {
            // Fallback: check the actual todo.g3.md file in the session directory
            // This handles sessions created before todo_snapshot was properly saved
            let todo_file_path = path.join("todo.g3.md");
            if todo_file_path.exists() {
                std::fs::read_to_string(&todo_file_path)
                    .map(|content| content.contains("- [ ]"))
                    .unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        };
        
        if has_incomplete {
            candidates.push(continuation);
        }
    }
    
    // Sort by created_at descending and return the most recent
    candidates.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(candidates.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_continuation_creation() {
        let continuation = SessionContinuation::new(
            false,
            None,
            "test_session_123".to_string(),
            Some("Task completed successfully".to_string()),
            "/path/to/session.json".to_string(),
            45.0,
            Some("- [x] Task 1\n- [ ] Task 2".to_string()),
            "/home/user/project".to_string(),
        );
        
        assert_eq!(continuation.version, CONTINUATION_VERSION);
        assert_eq!(continuation.session_id, "test_session_123");
        assert!(continuation.can_restore_full_context());
    }

    #[test]
    fn test_can_restore_full_context() {
        let mut continuation = SessionContinuation::new(
            false,
            None,
            "test".to_string(),
            None,
            "path".to_string(),
            50.0,
            None,
            ".".to_string(),
        );
        
        assert!(continuation.can_restore_full_context()); // 50% < 80%
        
        continuation.context_percentage = 80.0;
        assert!(!continuation.can_restore_full_context()); // 80% >= 80%
        
        continuation.context_percentage = 95.0;
        assert!(!continuation.can_restore_full_context()); // 95% >= 80%
    }

    #[test]
    fn test_has_incomplete_todos() {
        let mut continuation = SessionContinuation::new(
            true,
            Some("fowler".to_string()),
            "test".to_string(),
            None,
            "path".to_string(),
            50.0,
            Some("- [x] Done\n- [ ] Not done".to_string()),
            ".".to_string(),
        );
        
        assert!(continuation.has_incomplete_todos());
        
        continuation.todo_snapshot = Some("- [x] All done".to_string());
        assert!(!continuation.has_incomplete_todos());
        
        continuation.todo_snapshot = None;
        assert!(!continuation.has_incomplete_todos());
    }
}
