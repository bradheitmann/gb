//! TODO list management tools.

use anyhow::Result;
use std::io::Write;
use tracing::debug;

use crate::ui_writer::UiWriter;
use crate::ToolCall;

use super::executor::ToolContext;

/// Execute the `todo_read` tool.
pub async fn execute_todo_read<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &mut ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing todo_read tool call");
    let _ = tool_call; // unused but kept for consistency
    
    let todo_path = ctx.get_todo_path();

    if !todo_path.exists() {
        // Also update in-memory content to stay in sync
        let mut todo = ctx.todo_content.write().await;
        *todo = String::new();
        return Ok("📝 TODO list is empty (no todo.g3.md file found)".to_string());
    }

    match std::fs::read_to_string(&todo_path) {
        Ok(content) => {
            // Update in-memory content to stay in sync
            let mut todo = ctx.todo_content.write().await;
            *todo = content.clone();

            // Check for staleness if enabled and we have a requirements SHA
            if ctx.config.agent.check_todo_staleness {
                if let Some(req_sha) = ctx.requirements_sha {
                    if let Some(staleness_result) = check_todo_staleness(&content, req_sha, ctx.ui_writer) {
                        return Ok(staleness_result);
                    }
                }
            }

            if content.trim().is_empty() {
                Ok("📝 TODO list is empty".to_string())
            } else {
                for line in content.lines() {
                    ctx.ui_writer.print_tool_output_line(line);
                }
                Ok(format!("📝 TODO list:\n{}", content))
            }
        }
        Err(e) => Ok(format!("❌ Failed to read TODO.md: {}", e)),
    }
}

/// Execute the `todo_write` tool.
pub async fn execute_todo_write<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &mut ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing todo_write tool call");
    
    let content_str = match tool_call.args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return Ok("❌ Missing content argument".to_string()),
    };

    let char_count = content_str.chars().count();
    let max_chars = std::env::var("G3_TODO_MAX_CHARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50_000);

    if max_chars > 0 && char_count > max_chars {
        return Ok(format!(
            "❌ TODO list too large: {} chars (max: {})",
            char_count, max_chars
        ));
    }

    // Check if all todos are completed (all checkboxes are checked)
    let has_incomplete = content_str
        .lines()
        .any(|line| line.trim().starts_with("- [ ]"));

    // If all todos are complete, delete the file instead of writing
    // EXCEPT in planner mode (G3_TODO_PATH is set) - preserve for rename to completed_todo_*.md
    let in_planner_mode = std::env::var("G3_TODO_PATH").is_ok();
    let todo_path = ctx.get_todo_path();

    if !in_planner_mode
        && !has_incomplete
        && (content_str.contains("- [x]") || content_str.contains("- [X]"))
    {
        if todo_path.exists() {
            match std::fs::remove_file(&todo_path) {
                Ok(_) => {
                    let mut todo = ctx.todo_content.write().await;
                    *todo = String::new();
                    // Show the final completed TODOs before deletion
                    let mut result =
                        String::from("✅ All TODOs completed! Removed todo.g3.md\n\nFinal status:\n");
                    for line in content_str.lines() {
                        ctx.ui_writer.print_tool_output_line(line);
                        result.push_str(line);
                        result.push('\n');
                    }
                    return Ok(result);
                }
                Err(e) => return Ok(format!("❌ Failed to remove todo.g3.md: {}", e)),
            }
        }
    }

    match std::fs::write(&todo_path, content_str) {
        Ok(_) => {
            // Also update in-memory content to stay in sync
            let mut todo = ctx.todo_content.write().await;
            *todo = content_str.to_string();
            // Print the TODO content to the console (inside the tool frame)
            for line in content_str.lines() {
                ctx.ui_writer.print_tool_output_line(line);
            }
            Ok(format!(
                "✅ TODO list updated ({} chars) and saved to todo.g3.md:\n{}",
                char_count, content_str
            ))
        }
        Err(e) => Ok(format!("❌ Failed to write todo.g3.md: {}", e)),
    }
}

/// Check if the TODO list is stale (generated from a different requirements file).
/// Returns Some(message) if staleness was detected and handled, None otherwise.
fn check_todo_staleness<W: UiWriter>(
    content: &str,
    req_sha: &str,
    ui_writer: &W,
) -> Option<String> {
    // Parse the first line for the SHA header
    let first_line = content.lines().next()?;
    
    if !first_line.starts_with("{{Based on the requirements file with SHA256:") {
        return None;
    }

    let parts: Vec<&str> = first_line.split("SHA256:").collect();
    if parts.len() <= 1 {
        return None;
    }

    let todo_sha = parts[1].trim().trim_end_matches("}}").trim();
    if todo_sha == req_sha {
        return None;
    }

    let warning = format!(
        "⚠️ TODO list is stale! It was generated from a different requirements file.\nExpected SHA: {}\nFound SHA:    {}",
        req_sha, todo_sha
    );
    ui_writer.print_context_status(&warning);

    // Beep 6 times
    print!("\x07\x07\x07\x07\x07\x07");
    let _ = std::io::stdout().flush();

    let options = [
        "Ignore and Continue",
        "Mark as Stale",
        "Quit Application",
    ];
    let choice = ui_writer.prompt_user_choice(
        "Requirements have changed! What would you like to do?",
        &options,
    );

    match choice {
        0 => {
            // Ignore and Continue
            ui_writer.print_context_status("⚠️ Ignoring staleness warning.");
            None
        }
        1 => {
            // Mark as Stale
            Some("⚠️ TODO list is stale (requirements changed). Please regenerate the TODO list to match the new requirements.".to_string())
        }
        2 => {
            // Quit Application
            ui_writer.print_context_status("❌ Quitting application as requested.");
            std::process::exit(0);
        }
        _ => None,
    }
}
