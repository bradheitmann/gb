//! Background process management for long-running tasks like game servers.
//!
//! This module provides a way to launch processes in the background with:
//! - Automatic log capture to files (stdout/stderr combined)
//! - Named process tracking for easy reference
//! - Process lifecycle management (start, stop via shell)
//!
//! The design is intentionally minimal - only one tool (`background_process`) is exposed.
//! Users can use the regular `shell` tool to:
//! - Read logs: `cat /path/to/logs.txt` or `tail -100 /path/to/logs.txt`
//! - Stop processes: `kill <pid>` or `pkill -f <name>`
//! - Check status: `ps aux | grep <name>`

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Information about a running background process
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// User-provided name for the process
    pub name: String,
    /// The command that was executed
    pub command: String,
    /// Process ID
    pub pid: u32,
    /// Path to the log file (combined stdout/stderr)
    pub log_file: PathBuf,
    /// Timestamp when the process was started
    pub started_at: u64,
    /// Working directory where the process was started
    pub working_dir: PathBuf,
}

/// Manages background processes launched by the agent
#[derive(Debug)]
pub struct BackgroundProcessManager {
    /// Map of process name -> process info
    processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
    /// Map of process name -> child handle (for cleanup)
    children: Arc<Mutex<HashMap<String, Child>>>,
    /// Directory where log files are stored
    log_dir: PathBuf,
}

impl BackgroundProcessManager {
    /// Create a new background process manager
    pub fn new(log_dir: PathBuf) -> Self {
        // Ensure log directory exists
        if let Err(e) = fs::create_dir_all(&log_dir) {
            debug!("Failed to create log directory {:?}: {}", log_dir, e);
        }

        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            children: Arc::new(Mutex::new(HashMap::new())),
            log_dir,
        }
    }

    /// Start a new background process
    ///
    /// # Arguments
    /// * `name` - A unique name for this process (used to reference it later)
    /// * `command` - The shell command to execute
    /// * `working_dir` - The directory to run the command in
    ///
    /// # Returns
    /// ProcessInfo on success, or an error message
    pub fn start(
        &self,
        name: &str,
        command: &str,
        working_dir: &PathBuf,
    ) -> Result<ProcessInfo, String> {
        // Check if a process with this name already exists
        {
            let processes = self.processes.lock().unwrap();
            if processes.contains_key(name) {
                return Err(format!(
                    "A process named '{}' is already running. Stop it first or use a different name.",
                    name
                ));
            }
        }

        // Create log file with timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let log_filename = format!("{}_{}.log", name, timestamp);
        let log_file = self.log_dir.join(&log_filename);

        // Open log file for writing
        let log_handle = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)
            .map_err(|e| format!("Failed to create log file: {}", e))?;

        // Write header to log file
        {
            let mut file = &log_handle;
            writeln!(file, "=== Background Process Log ===").ok();
            writeln!(file, "Name: {}", name).ok();
            writeln!(file, "Command: {}", command).ok();
            writeln!(file, "Working Directory: {:?}", working_dir).ok();
            writeln!(file, "Started: {}", timestamp).ok();
            writeln!(file, "================================\n").ok();
        }

        // Clone the file handle for stderr
        let log_handle_stderr = log_handle
            .try_clone()
            .map_err(|e| format!("Failed to clone log file handle: {}", e))?;

        // Spawn the process
        let child = Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(working_dir)
            .stdout(Stdio::from(log_handle))
            .stderr(Stdio::from(log_handle_stderr))
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        let pid = child.id();

        let info = ProcessInfo {
            name: name.to_string(),
            command: command.to_string(),
            pid,
            log_file: log_file.clone(),
            started_at: timestamp,
            working_dir: working_dir.clone(),
        };

        // Store process info and child handle
        {
            let mut processes = self.processes.lock().unwrap();
            processes.insert(name.to_string(), info.clone());
        }
        {
            let mut children = self.children.lock().unwrap();
            children.insert(name.to_string(), child);
        }

        debug!(
            "Started background process '{}' (PID: {}) with logs at {:?}",
            name, pid, log_file
        );

        Ok(info)
    }

    /// List all tracked background processes
    pub fn list(&self) -> Vec<ProcessInfo> {
        let processes = self.processes.lock().unwrap();
        processes.values().cloned().collect()
    }

    /// Get info about a specific process by name
    pub fn get(&self, name: &str) -> Option<ProcessInfo> {
        let processes = self.processes.lock().unwrap();
        processes.get(name).cloned()
    }

    /// Check if a process is still running
    pub fn is_running(&self, name: &str) -> bool {
        let mut children = self.children.lock().unwrap();
        if let Some(child) = children.get_mut(name) {
            match child.try_wait() {
                Ok(Some(_)) => false, // Process has exited
                Ok(None) => true,     // Still running
                Err(_) => false,      // Error checking, assume not running
            }
        } else {
            false
        }
    }

    /// Remove a process from tracking (call after it has been killed)
    pub fn remove(&self, name: &str) -> Option<ProcessInfo> {
        let info = {
            let mut processes = self.processes.lock().unwrap();
            processes.remove(name)
        };
        {
            let mut children = self.children.lock().unwrap();
            children.remove(name);
        }
        info
    }

    /// Clean up all processes on shutdown
    pub fn cleanup(&self) {
        let mut children = self.children.lock().unwrap();
        for (name, mut child) in children.drain() {
            debug!("Cleaning up background process '{}'", name);
            let _ = child.kill();
        }
    }
}

impl Drop for BackgroundProcessManager {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_start_and_list_process() {
        let temp_dir = std::env::temp_dir().join("g3_bg_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = BackgroundProcessManager::new(temp_dir.clone());

        // Start a simple process that sleeps
        let result = manager.start("test_sleep", "sleep 10", &temp_dir);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.name, "test_sleep");
        assert!(info.pid > 0);
        assert!(info.log_file.exists());

        // List should contain our process
        let list = manager.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "test_sleep");

        // Should be running
        assert!(manager.is_running("test_sleep"));

        // Cleanup
        manager.cleanup();

        // Give it a moment to clean up
        thread::sleep(Duration::from_millis(100));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_duplicate_name_rejected() {
        let temp_dir = std::env::temp_dir().join("g3_bg_test_dup");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = BackgroundProcessManager::new(temp_dir.clone());

        // Start first process
        let result1 = manager.start("my_game", "sleep 10", &temp_dir);
        assert!(result1.is_ok());

        // Try to start another with same name
        let result2 = manager.start("my_game", "sleep 5", &temp_dir);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("already running"));

        // Cleanup
        manager.cleanup();
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
