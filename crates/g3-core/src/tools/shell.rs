//! Shell command execution tools.

use anyhow::Result;
use tracing::debug;

use crate::ui_writer::UiWriter;
use crate::utils::resolve_paths_in_shell_command;
use crate::utils::shell_escape_command;
use crate::ToolCall;

use super::executor::ToolContext;

/// Execute the `shell` tool.
pub async fn execute_shell<W: UiWriter>(tool_call: &ToolCall, ctx: &ToolContext<'_, W>) -> Result<String> {
    debug!("Processing shell tool call");
    
    let command = match tool_call.args.get("command").and_then(|v| v.as_str()) {
        Some(cmd) => cmd,
        None => {
            debug!("No command parameter found in args: {:?}", tool_call.args);
            return Ok("❌ Missing command argument".to_string());
        }
    };
    
    debug!("Command string: {}", command);
    // First resolve any file paths with Unicode space fallback (macOS screenshot names)
    let resolved_command = resolve_paths_in_shell_command(command);
    debug!("Resolved command: {}", resolved_command);
    let escaped_command = shell_escape_command(&resolved_command);

    let executor = g3_execution::CodeExecutor::new();

    struct ToolOutputReceiver<'a, W: UiWriter> {
        ui_writer: &'a W,
    }

    impl<'a, W: UiWriter> g3_execution::OutputReceiver for ToolOutputReceiver<'a, W> {
        fn on_output_line(&self, line: &str) {
            self.ui_writer.update_tool_output_line(line);
        }
    }

    let receiver = ToolOutputReceiver {
        ui_writer: ctx.ui_writer,
    };

    debug!(
        "ABOUT TO CALL execute_bash_streaming_in_dir: escaped_command='{}', working_dir={:?}",
        escaped_command, ctx.working_dir
    );

    match executor
        .execute_bash_streaming_in_dir(&escaped_command, &receiver, ctx.working_dir)
        .await
    {
        Ok(result) => {
            if result.success {
                Ok(if result.stdout.is_empty() {
                    "✅ Command executed successfully".to_string()
                } else {
                    result.stdout.trim().to_string()
                })
            } else {
                Ok(format!("❌ Command failed: {}", result.stderr.trim()))
            }
        }
        Err(e) => Ok(format!("❌ Execution error: {}", e)),
    }
}

/// Execute the `background_process` tool.
pub async fn execute_background_process<W: UiWriter>(
    tool_call: &ToolCall,
    ctx: &ToolContext<'_, W>,
) -> Result<String> {
    debug!("Processing background_process tool call");
    
    let name = match tool_call.args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return Ok("❌ Missing 'name' argument".to_string()),
    };

    let command = match tool_call.args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return Ok("❌ Missing 'command' argument".to_string()),
    };

    // Use provided working_dir, or fall back to context working_dir, or current dir
    let work_dir = tool_call
        .args
        .get("working_dir")
        .and_then(|v| v.as_str())
        .map(|s| std::path::PathBuf::from(shellexpand::tilde(s).as_ref()))
        .or_else(|| ctx.working_dir.map(std::path::PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    match ctx.background_process_manager.start(name, command, &work_dir) {
        Ok(info) => Ok(format!(
            "✅ Background process '{}' started\n\n\
            **PID:** {}\n\
            **Log file:** {}\n\
            **Working dir:** {}\n\n\
            To interact with this process, use the shell tool:\n\
            - View logs: `tail -100 {}`\n\
            - Follow logs: `tail -f {}` (blocks until Ctrl+C)\n\
            - Check status: `ps -p {}`\n\
            - Stop process: `kill {}`",
            info.name,
            info.pid,
            info.log_file.display(),
            info.working_dir.display(),
            info.log_file.display(),
            info.log_file.display(),
            info.pid,
            info.pid
        )),
        Err(e) => Ok(format!("❌ Failed to start background process: {}", e)),
    }
}
