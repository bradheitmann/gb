//! Streaming completion logic for the Agent.
//!
//! This module handles the streaming response from LLM providers,
//! including tool call detection, execution, and auto-continue logic.

use crate::context_window::ContextWindow;
use crate::streaming_parser::StreamingToolParser;
use crate::ToolCall;
use g3_providers::{CompletionRequest, MessageRole};
use std::time::{Duration, Instant};
use tracing::{debug, error};

/// Constants for streaming behavior
pub const MAX_ITERATIONS: usize = 400;

/// State tracked across streaming iterations
pub struct StreamingState {
    pub full_response: String,
    pub first_token_time: Option<Duration>,
    pub stream_start: Instant,
    pub iteration_count: usize,
    pub response_started: bool,
    pub any_tool_executed: bool,
    pub auto_summary_attempts: usize,
    pub final_output_called: bool,
    pub turn_accumulated_usage: Option<g3_providers::Usage>,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            full_response: String::new(),
            first_token_time: None,
            stream_start: Instant::now(),
            iteration_count: 0,
            response_started: false,
            any_tool_executed: false,
            auto_summary_attempts: 0,
            final_output_called: false,
            turn_accumulated_usage: None,
        }
    }

    pub fn record_first_token(&mut self) {
        if self.first_token_time.is_none() {
            self.first_token_time = Some(self.stream_start.elapsed());
        }
    }

    pub fn get_ttft(&self) -> Duration {
        self.first_token_time.unwrap_or_else(|| self.stream_start.elapsed())
    }
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::new()
    }
}

/// State tracked within a single streaming iteration
pub struct IterationState {
    pub parser: StreamingToolParser,
    pub current_response: String,
    pub tool_executed: bool,
    pub chunks_received: usize,
    pub raw_chunks: Vec<String>,
    pub accumulated_usage: Option<g3_providers::Usage>,
}

impl IterationState {
    pub fn new() -> Self {
        Self {
            parser: StreamingToolParser::new(),
            current_response: String::new(),
            tool_executed: false,
            chunks_received: 0,
            raw_chunks: Vec::new(),
            accumulated_usage: None,
        }
    }

    /// Store a raw chunk for debugging (limited to first 20 + last few)
    pub fn record_chunk(&mut self, chunk: &g3_providers::CompletionChunk) {
        if self.chunks_received < 20 || chunk.finished {
            self.raw_chunks.push(format!(
                "Chunk #{}: content={:?}, finished={}, tool_calls={:?}",
                self.chunks_received + 1,
                chunk.content,
                chunk.finished,
                chunk.tool_calls
            ));
        } else if self.raw_chunks.len() == 20 {
            self.raw_chunks.push("... (chunks 21+ omitted for brevity) ...".to_string());
        }
        self.chunks_received += 1;
    }
}

impl Default for IterationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Clean LLM-specific tokens from content
pub fn clean_llm_tokens(content: &str) -> String {
    content
        .replace("<|im_end|>", "")
        .replace("</s>", "")
        .replace("[/INST]", "")
        .replace("<</SYS>>", "")
}

/// Format a duration for display
pub fn format_duration(duration: Duration) -> String {
    let total_ms = duration.as_millis();

    if total_ms < 1000 {
        format!("{}ms", total_ms)
    } else if total_ms < 60_000 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        let minutes = total_ms / 60_000;
        let remaining_seconds = (total_ms % 60_000) as f64 / 1000.0;
        format!("{}m {:.1}s", minutes, remaining_seconds)
    }
}

/// Format the timing footer with optional token usage info
pub fn format_timing_footer(
    elapsed: Duration,
    ttft: Duration,
    turn_tokens: Option<u32>,
    context_percentage: f32,
) -> String {
    let timing = format!(
        "⏱️ {} | 💭 {}",
        format_duration(elapsed),
        format_duration(ttft)
    );

    // Add token usage info if available (dimmed)
    if let Some(tokens) = turn_tokens {
        format!(
            "{}  \x1b[2m{} ◉ | {:.0}%\x1b[0m",
            timing, tokens, context_percentage
        )
    } else {
        format!("{}  \x1b[2m{:.0}%\x1b[0m", timing, context_percentage)
    }
}

/// Log detailed error information when stream produces no content
pub fn log_stream_error(
    iteration_count: usize,
    provider_name: &str,
    provider_model: &str,
    chunks_received: usize,
    parser: &StreamingToolParser,
    request: &CompletionRequest,
    context_window: &ContextWindow,
    session_id: Option<&str>,
    raw_chunks: &[String],
) {
    error!("=== STREAM ERROR: No content or tool calls received ===");
    error!("Iteration: {}/{}", iteration_count, MAX_ITERATIONS);
    error!("Provider: {} (model: {})", provider_name, provider_model);
    error!("Chunks received: {}", chunks_received);
    
    error!("Parser state:");
    error!("  - Text buffer length: {}", parser.text_buffer_len());
    error!("  - Text buffer content: {:?}", parser.get_text_content());
    error!("  - Has incomplete tool call: {}", parser.has_incomplete_tool_call());
    error!("  - Message stopped: {}", parser.is_message_stopped());
    error!("  - In JSON tool call: {}", parser.is_in_json_tool_call());
    error!("  - JSON tool start: {:?}", parser.json_tool_start_position());
    
    error!("Request details:");
    error!("  - Messages count: {}", request.messages.len());
    error!("  - Has tools: {}", request.tools.is_some());
    error!("  - Max tokens: {:?}", request.max_tokens);
    error!("  - Temperature: {:?}", request.temperature);
    error!("  - Stream: {}", request.stream);

    error!("Raw chunks received ({} total):", chunks_received);
    for (i, chunk_str) in raw_chunks.iter().take(25).enumerate() {
        error!("  [{}] {}", i, chunk_str);
    }

    match serde_json::to_string_pretty(request) {
        Ok(json) => {
            error!("(turn on DEBUG logging for the raw JSON request)");
            debug!("Full request JSON:\n{}", json);
        }
        Err(e) => error!("Failed to serialize request: {}", e),
    }

    if let Some(last_user_msg) = request
        .messages
        .iter()
        .rev()
        .find(|m| matches!(m.role, MessageRole::User))
    {
        let truncated = if last_user_msg.content.len() > 500 {
            format!("{}... (truncated)", &last_user_msg.content[..500])
        } else {
            last_user_msg.content.clone()
        };
        error!("Last user message: {}", truncated);
    }

    error!("Context window state:");
    error!(
        "  - Used tokens: {}/{}",
        context_window.used_tokens, context_window.total_tokens
    );
    error!("  - Percentage used: {:.1}%", context_window.percentage_used());
    error!(
        "  - Conversation history length: {}",
        context_window.conversation_history.len()
    );

    error!("Session ID: {:?}", session_id);
    error!("=== END STREAM ERROR ===");
}

/// Truncate a string value for display, respecting UTF-8 boundaries
pub fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.char_indices().take(max_len).map(|(_, c)| c).collect();
        format!("{}...", truncated)
    }
}

/// Truncate a line for tool output display
pub fn truncate_line(line: &str, max_width: usize, should_truncate: bool) -> String {
    if !should_truncate {
        line.to_string()
    } else if line.chars().count() <= max_width {
        line.to_string()
    } else {
        let truncated: String = line.chars().take(max_width.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Check if two tool calls are duplicates (same tool and args)
pub fn are_tool_calls_duplicate(tc1: &ToolCall, tc2: &ToolCall) -> bool {
    tc1.tool == tc2.tool && tc1.args == tc2.args
}

/// Determine if a response is essentially empty (whitespace or timing only)
pub fn is_empty_response(response: &str) -> bool {
    response.trim().is_empty()
        || response
            .lines()
            .all(|line| line.trim().is_empty() || line.trim().starts_with("⏱️"))
}

/// Check if an error is a recoverable connection error
pub fn is_connection_error(error_msg: &str) -> bool {
    error_msg.contains("unexpected EOF")
        || error_msg.contains("connection")
        || error_msg.contains("chunk size line")
        || error_msg.contains("body error")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_llm_tokens() {
        assert_eq!(clean_llm_tokens("hello<|im_end|>"), "hello");
        assert_eq!(clean_llm_tokens("test</s>more"), "testmore");
        assert_eq!(clean_llm_tokens("[/INST]response"), "response");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.5s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30.0s");
    }

    #[test]
    fn test_truncate_for_display() {
        assert_eq!(truncate_for_display("short", 10), "short");
        assert_eq!(truncate_for_display("this is long", 5), "this ...");
    }

    #[test]
    fn test_is_empty_response() {
        assert!(is_empty_response(""));
        assert!(is_empty_response("   \n  "));
        assert!(is_empty_response("⏱️ 1.5s"));
        assert!(!is_empty_response("actual content"));
    }

    #[test]
    fn test_is_connection_error() {
        assert!(is_connection_error("unexpected EOF during read"));
        assert!(is_connection_error("connection reset"));
        assert!(!is_connection_error("invalid JSON"));
    }
}
