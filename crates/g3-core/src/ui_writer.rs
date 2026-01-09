/// Interface for UI output operations
/// This trait abstracts all UI operations to allow different implementations
/// (console, TUI, web, etc.) without coupling the core logic to specific output methods.
pub trait UiWriter: Send + Sync {
    /// Print a simple message
    fn print(&self, message: &str);

    /// Print a message with a newline
    fn println(&self, message: &str);

    /// Print without newline (for progress indicators)
    fn print_inline(&self, message: &str);

    /// Print a system prompt section
    fn print_system_prompt(&self, prompt: &str);

    /// Print a context window status message
    fn print_context_status(&self, message: &str);

    /// Print a context thinning success message with highlight and animation
    fn print_context_thinning(&self, message: &str);

    /// Print a tool execution header
    fn print_tool_header(&self, tool_name: &str, tool_args: Option<&serde_json::Value>);

    /// Print a tool argument
    fn print_tool_arg(&self, key: &str, value: &str);

    /// Print tool output header
    fn print_tool_output_header(&self);

    /// Update the current tool output line (replaces previous line)
    fn update_tool_output_line(&self, line: &str);

    /// Print a tool output line
    fn print_tool_output_line(&self, line: &str);

    /// Print tool output summary (when output is truncated)
    fn print_tool_output_summary(&self, hidden_count: usize);

    /// Print tool execution timing
    fn print_tool_timing(&self, duration_str: &str, tokens_delta: u32, context_percentage: f32);

    /// Print the agent prompt indicator
    fn print_agent_prompt(&self);

    /// Print agent response inline (for streaming)
    fn print_agent_response(&self, content: &str);

    /// Notify that an SSE event was received (including pings)
    fn notify_sse_received(&self);

    /// Flush any buffered output
    fn flush(&self);

    /// Returns true if this UI writer wants full, untruncated output
    /// Default is false (truncate for human readability)
    fn wants_full_output(&self) -> bool {
        false
    }

    /// Prompt the user for a yes/no confirmation
    fn prompt_user_yes_no(&self, message: &str) -> bool;

    /// Prompt the user to choose from a list of options
    /// Returns the index of the selected option
    fn prompt_user_choice(&self, message: &str, options: &[&str]) -> usize;

    /// Print the final output summary with markdown formatting
    /// Shows a spinner while formatting, then renders the markdown
    fn print_final_output(&self, summary: &str);

    /// Filter JSON tool calls from streaming content for display.
    /// This is a UI concern - the raw content should be preserved for logging.
    /// Default implementation passes through unchanged.
    fn filter_json_tool_calls(&self, content: &str) -> String {
        content.to_string()
    }

    /// Reset the JSON tool call filter state.
    /// Called at the start of a new response to clear any partial state.
    /// Default implementation does nothing.
    fn reset_json_filter(&self) {}
    
    /// Finish the streaming markdown formatter and flush any remaining content.
    /// Called at the end of an agent response to emit any buffered markdown.
    /// Also resets the formatter for the next response.
    /// Default implementation does nothing.
    fn finish_streaming_markdown(&self) {}

    /// Set whether the UI is in agent mode.
    /// When in agent mode, tool names may be displayed differently (e.g., different color).
    /// Default implementation does nothing.
    fn set_agent_mode(&self, _is_agent_mode: bool) {}
}

/// A no-op implementation for when UI output is not needed
pub struct NullUiWriter;

impl UiWriter for NullUiWriter {
    fn print(&self, _message: &str) {}
    fn println(&self, _message: &str) {}
    fn print_inline(&self, _message: &str) {}
    fn print_system_prompt(&self, _prompt: &str) {}
    fn print_context_status(&self, _message: &str) {}
    fn print_context_thinning(&self, _message: &str) {}
    fn print_tool_header(&self, _tool_name: &str, _tool_args: Option<&serde_json::Value>) {}
    fn print_tool_arg(&self, _key: &str, _value: &str) {}
    fn print_tool_output_header(&self) {}
    fn update_tool_output_line(&self, _line: &str) {}
    fn print_tool_output_line(&self, _line: &str) {}
    fn print_tool_output_summary(&self, _hidden_count: usize) {}
    fn print_tool_timing(&self, _duration_str: &str, _tokens_delta: u32, _context_percentage: f32) {}
    fn print_agent_prompt(&self) {}
    fn print_agent_response(&self, _content: &str) {}
    fn notify_sse_received(&self) {}
    fn flush(&self) {}
    fn finish_streaming_markdown(&self) {}
    fn wants_full_output(&self) -> bool {
        false
    }
    fn prompt_user_yes_no(&self, _message: &str) -> bool {
        true
    }
    fn prompt_user_choice(&self, _message: &str, _options: &[&str]) -> usize {
        0
    }
    fn print_final_output(&self, _summary: &str) {
        // No-op for null writer
    }
}
