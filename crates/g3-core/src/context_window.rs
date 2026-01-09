//! Context window management for conversation history and token tracking.
//!
//! This module handles:
//! - Token counting and usage tracking
//! - Conversation history management
//! - Context thinning (reducing context size by saving large tool results to disk)
//! - Summarization triggers

use g3_providers::{Message, MessageRole, Usage};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::paths::get_thinned_dir;
use crate::ToolCall;

/// Scope for context thinning operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinScope {
    /// Process only the first third of messages (incremental thinning)
    FirstThird,
    /// Process all messages (aggressive thinning, aka "skinnify")
    All,
}

impl ThinScope {
    fn label(&self) -> &'static str {
        match self {
            ThinScope::FirstThird => "thinned",
            ThinScope::All => "skinnified",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            ThinScope::FirstThird => "🥒",
            ThinScope::All => "🦴",
        }
    }

    fn file_prefix(&self) -> &'static str {
        match self {
            ThinScope::FirstThird => "leaned",
            ThinScope::All => "skinny",
        }
    }

    fn error_action(&self) -> &'static str {
        match self {
            ThinScope::FirstThird => "thinning",
            ThinScope::All => "skinnifying",
        }
    }
}

/// Represents a modification to be applied to a message
#[derive(Debug)]
enum ThinModification {
    /// Replace the entire message content
    ReplaceContent { index: usize, new_content: String, chars_saved: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    pub used_tokens: u32,
    pub total_tokens: u32,
    pub cumulative_tokens: u32, // Track cumulative tokens across all interactions
    pub conversation_history: Vec<Message>,
    pub last_thinning_percentage: u32, // Track the last percentage at which we thinned
}

impl ContextWindow {
    pub fn new(total_tokens: u32) -> Self {
        Self {
            used_tokens: 0,
            total_tokens,
            cumulative_tokens: 0,
            conversation_history: Vec::new(),
            last_thinning_percentage: 0,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.add_message_with_tokens(message, None);
    }

    /// Add a message with optional token count from the provider
    pub fn add_message_with_tokens(&mut self, message: Message, tokens: Option<u32>) {
        // Skip messages with empty content to avoid API errors
        if message.content.trim().is_empty() {
            warn!("Skipping empty message to avoid API error");
            return;
        }

        // Use provided token count if available, otherwise estimate
        let token_count = tokens.unwrap_or_else(|| Self::estimate_tokens(&message.content));
        self.used_tokens += token_count;
        self.cumulative_tokens += token_count;
        self.conversation_history.push(message);

        debug!(
            "Added message with {} tokens (used: {}/{}, cumulative: {})",
            token_count, self.used_tokens, self.total_tokens, self.cumulative_tokens
        );
    }

    /// Update token usage from provider response
    /// NOTE: This only updates cumulative_tokens (total API usage tracking).
    /// It does NOT update used_tokens because:
    /// 1. prompt_tokens represents the ENTIRE context sent to API (already tracked via add_message)
    /// 2. completion_tokens will be tracked when the assistant message is added via add_message
    /// Adding total_tokens here would cause double/triple counting and break the 80% threshold check.
    pub fn update_usage_from_response(&mut self, usage: &Usage) {
        // Only update cumulative tokens for API usage tracking
        // Do NOT update used_tokens - that's tracked via add_message to avoid double counting
        self.cumulative_tokens += usage.total_tokens;

        debug!(
            "Updated cumulative tokens: {} (used: {}/{}, cumulative: {})",
            usage.total_tokens, self.used_tokens, self.total_tokens, self.cumulative_tokens
        );
    }

    /// More accurate token estimation
    pub fn estimate_tokens(text: &str) -> u32 {
        // Better heuristic:
        // - Average English text: ~4 characters per token
        // - Code/JSON: ~3 characters per token (more symbols)
        // - Add 10% buffer for safety
        let base_estimate = if text.contains("{") || text.contains("```") || text.contains("fn ") {
            (text.len() as f32 / 3.0).ceil() as u32 // Code/JSON
        } else {
            (text.len() as f32 / 4.0).ceil() as u32 // Regular text
        };
        (base_estimate as f32 * 1.1).ceil() as u32 // Add 10% buffer
    }

    pub fn update_usage(&mut self, usage: &Usage) {
        // Deprecated: Use update_usage_from_response instead
        self.update_usage_from_response(usage);
    }

    /// Update cumulative token usage (for streaming) when no provider usage data is available
    /// NOTE: This only updates cumulative_tokens, not used_tokens.
    /// The assistant message will be added via add_message which tracks used_tokens.
    pub fn add_streaming_tokens(&mut self, new_tokens: u32) {
        // Only update cumulative tokens - used_tokens is tracked via add_message
        self.cumulative_tokens += new_tokens;
        debug!(
            "Updated cumulative streaming tokens: {} (used: {}/{}, cumulative: {})",
            new_tokens, self.used_tokens, self.total_tokens, self.cumulative_tokens
        );
    }

    pub fn percentage_used(&self) -> f32 {
        if self.total_tokens == 0 {
            0.0
        } else {
            (self.used_tokens as f32 / self.total_tokens as f32) * 100.0
        }
    }

    /// Clear the conversation history while preserving system messages
    /// Used by /clear command to start fresh
    pub fn clear_conversation(&mut self) {
        // Keep only system messages (system prompt, README, etc.)
        let system_messages: Vec<Message> = self
            .conversation_history
            .iter()
            .filter(|m| matches!(m.role, MessageRole::System))
            .cloned()
            .collect();

        self.conversation_history = system_messages;
        self.used_tokens = self
            .conversation_history
            .iter()
            .map(|m| Self::estimate_tokens(&m.content))
            .sum();
        self.last_thinning_percentage = 0;
    }

    pub fn remaining_tokens(&self) -> u32 {
        self.total_tokens.saturating_sub(self.used_tokens)
    }

    /// Check if we should trigger compaction (at 80% capacity)
    pub fn should_compact(&self) -> bool {
        // Trigger at 80% OR if we're getting close to absolute limits
        // This prevents issues with models that have large contexts but still hit limits
        let percentage_trigger = self.percentage_used() >= 80.0;

        // Also trigger if we're approaching common token limits
        // Most models start having issues around 150k tokens
        let absolute_trigger = self.used_tokens > 150_000;

        percentage_trigger || absolute_trigger
    }

    /// Create a summary request prompt for the current conversation
    pub fn create_summary_prompt(&self) -> String {
        "Please provide a comprehensive summary of our conversation so far. Include:

1. **Main Topic/Goal**: What is the primary task or objective being worked on?
2. **Key Decisions**: What important decisions have been made?
3. **Actions Taken**: What specific actions, commands, or code changes have been completed?
4. **Current State**: What is the current status of the work?
5. **Important Context**: Any critical information, file paths, configurations, or constraints that should be remembered?
6. **Pending Items**: What remains to be done or what was the user's last request?

Format this as a detailed but concise summary that can be used to resume the conversation from scratch while maintaining full context.".to_string()
    }

    /// Reset the context window with a summary
    /// Preserves the original system prompt as the first message
    pub fn reset_with_summary(
        &mut self,
        summary: String,
        latest_user_message: Option<String>,
    ) -> usize {
        // Calculate chars saved (old history minus new summary)
        let old_chars: usize = self
            .conversation_history
            .iter()
            .map(|m| m.content.len())
            .sum();

        // Preserve the original system prompt (first message) and optionally the README (second message)
        let original_system_prompt = self.conversation_history.first().cloned();
        let readme_message = self.conversation_history.get(1).and_then(|msg| {
            if matches!(msg.role, MessageRole::System)
                && (msg.content.contains("Project README")
                    || msg.content.contains("Agent Configuration"))
            {
                Some(msg.clone())
            } else {
                None
            }
        });

        // Clear the conversation history
        self.conversation_history.clear();
        self.used_tokens = 0;

        // Re-add the original system prompt first (critical invariant)
        if let Some(system_prompt) = original_system_prompt {
            self.add_message(system_prompt);
        }

        // Re-add the README message if it existed
        if let Some(readme) = readme_message {
            self.add_message(readme);
        }

        // Add the summary as a system message
        let summary_message = Message::new(
            MessageRole::System,
            format!("Previous conversation summary:\n\n{}", summary),
        );
        self.add_message(summary_message);

        // Add the latest user message if provided
        if let Some(user_msg) = latest_user_message {
            self.add_message(Message::new(MessageRole::User, user_msg));
        }

        let new_chars: usize = self
            .conversation_history
            .iter()
            .map(|m| m.content.len())
            .sum();
        old_chars.saturating_sub(new_chars)
    }

    /// Check if we should trigger context thinning
    /// Triggers at 50%, 60%, 70%, and 80% thresholds
    pub fn should_thin(&self) -> bool {
        let current_percentage = self.percentage_used() as u32;

        // Check if we've crossed a new 10% threshold starting at 50%
        if current_percentage >= 50 {
            let current_threshold = (current_percentage / 10) * 10; // Round down to nearest 10%
            if current_threshold > self.last_thinning_percentage && current_threshold <= 80 {
                return true;
            }
        }

        false
    }

    /// Perform context thinning: scan messages and replace large tool results with file references.
    ///
    /// # Arguments
    /// * `session_id` - If provided, thinned content is saved to .g3/session/<session_id>/thinned/
    /// * `scope` - Controls which messages to process (first third or all)
    ///
    /// # Returns
    /// A tuple of (summary message, chars saved)
    pub fn thin_context_with_scope(
        &mut self,
        session_id: Option<&str>,
        scope: ThinScope,
    ) -> (String, usize) {
        let current_percentage = self.percentage_used() as u32;

        // Only update last_thinning_percentage for incremental thinning
        if scope == ThinScope::FirstThird {
            let current_threshold = (current_percentage / 10) * 10;
            self.last_thinning_percentage = current_threshold;
        }

        // Calculate message range based on scope
        let total_messages = self.conversation_history.len();
        let end_index = match scope {
            ThinScope::FirstThird => (total_messages / 3).max(1),
            ThinScope::All => total_messages,
        };

        // Determine output directory: use session dir if available, otherwise ~/tmp
        let tmp_dir = match Self::resolve_thinned_dir(session_id, scope) {
            Ok(dir) => dir,
            Err(msg) => return (msg, 0),
        };

        // Collect modifications to apply (avoids borrow checker issues)
        let modifications = self.collect_thin_modifications(end_index, &tmp_dir, scope.file_prefix());

        // Count results
        let mut leaned_count = 0;
        let mut tool_call_leaned_count = 0;
        let mut chars_saved = 0;

        // Apply modifications
        for modification in &modifications {
            match modification {
                ThinModification::ReplaceContent { index, new_content, chars_saved: saved } => {
                    if let Some(msg) = self.conversation_history.get_mut(*index) {
                        // Determine if this was a tool result or tool call based on content
                        if msg.content.starts_with("Tool result:") {
                            leaned_count += 1;
                        } else {
                            tool_call_leaned_count += 1;
                        }
                        msg.content = new_content.clone();
                        chars_saved += saved;
                    }
                }
            }
        }

        // Recalculate token usage after thinning
        self.recalculate_tokens();

        // Build result message
        self.build_thin_result_message(
            scope,
            current_percentage,
            leaned_count,
            tool_call_leaned_count,
            chars_saved,
        )
    }

    /// Collect all modifications needed for thinning without mutating
    fn collect_thin_modifications(
        &self,
        end_index: usize,
        tmp_dir: &str,
        file_prefix: &str,
    ) -> Vec<ThinModification> {
        let mut modifications = Vec::new();

        for i in 0..end_index {
            if let Some(message) = self.conversation_history.get(i) {
                // Check if the previous message was a TODO tool call
                let is_todo_result = self.is_todo_tool_result(i);

                // Process User messages that look like tool results
                if matches!(message.role, MessageRole::User)
                    && message.content.starts_with("Tool result:")
                    && !is_todo_result
                    && message.content.len() > 500
                {
                    if let Some(modification) = Self::create_tool_result_modification(
                        &message.content,
                        i,
                        tmp_dir,
                        file_prefix,
                    ) {
                        modifications.push(modification);
                    }
                }

                // Process Assistant messages that contain tool calls with large arguments
                if matches!(message.role, MessageRole::Assistant) {
                    if let Some(modification) = Self::create_tool_call_modification(
                        &message.content,
                        i,
                        tmp_dir,
                        file_prefix,
                    ) {
                        modifications.push(modification);
                    }
                }
            }
        }

        modifications
    }

    /// Backward-compatible wrapper for thin_context (first third only)
    pub fn thin_context(&mut self, session_id: Option<&str>) -> (String, usize) {
        self.thin_context_with_scope(session_id, ThinScope::FirstThird)
    }

    /// Backward-compatible wrapper for thin_context_all (entire history)
    pub fn thin_context_all(&mut self, session_id: Option<&str>) -> (String, usize) {
        self.thin_context_with_scope(session_id, ThinScope::All)
    }

    /// Resolve the directory for storing thinned content
    fn resolve_thinned_dir(session_id: Option<&str>, scope: ThinScope) -> Result<String, String> {
        if let Some(sid) = session_id {
            let thinned_dir = get_thinned_dir(sid);
            if let Err(e) = std::fs::create_dir_all(&thinned_dir) {
                warn!("Failed to create thinned directory: {}", e);
                return Err(format!(
                    "⚠️  Context {} failed: could not create thinned directory",
                    scope.error_action()
                ));
            }
            Ok(thinned_dir.to_string_lossy().to_string())
        } else {
            let fallback_dir = shellexpand::tilde("~/tmp").to_string();
            if let Err(e) = std::fs::create_dir_all(&fallback_dir) {
                warn!("Failed to create ~/tmp directory: {}", e);
                return Err(format!(
                    "⚠️  Context {} failed: could not create ~/tmp directory",
                    scope.error_action()
                ));
            }
            Ok(fallback_dir)
        }
    }

    /// Check if message at index i is a result of a TODO tool call
    fn is_todo_tool_result(&self, i: usize) -> bool {
        if i == 0 {
            return false;
        }

        if let Some(prev_message) = self.conversation_history.get(i - 1) {
            if matches!(prev_message.role, MessageRole::Assistant) {
                return prev_message.content.contains(r#""tool":"todo_read""#)
                    || prev_message.content.contains(r#""tool":"todo_write""#)
                    || prev_message.content.contains(r#""tool": "todo_read""#)
                    || prev_message.content.contains(r#""tool": "todo_write""#);
            }
        }
        false
    }

    /// Create a modification for thinning a tool result message
    fn create_tool_result_modification(
        content: &str,
        index: usize,
        tmp_dir: &str,
        file_prefix: &str,
    ) -> Option<ThinModification> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("{}_tool_result_{}_{}.txt", file_prefix, timestamp, index);
        let file_path = format!("{}/{}", tmp_dir, filename);

        if let Err(e) = std::fs::write(&file_path, content) {
            warn!("Failed to write thinned content to {}: {}", file_path, e);
            return None;
        }

        let original_len = content.len();
        let new_content = format!("Tool result saved to {}", file_path);
        let chars_saved = original_len - new_content.len();

        debug!(
            "Thinned tool result {} ({} chars) to {}",
            index, original_len, file_path
        );

        Some(ThinModification::ReplaceContent {
            index,
            new_content,
            chars_saved,
        })
    }

    /// Create a modification for thinning tool calls in an assistant message
    fn create_tool_call_modification(
        content: &str,
        index: usize,
        tmp_dir: &str,
        file_prefix: &str,
    ) -> Option<ThinModification> {
        // Look for JSON tool call patterns
        let tool_call_start = content
            .find(r#"{"tool":"#)
            .or_else(|| content.find(r#"{ "tool":"#))
            .or_else(|| content.find(r#"{"tool" :"#))
            .or_else(|| content.find(r#"{ "tool" :"#))?;

        let json_portion = &content[tool_call_start..];
        let json_end = Self::find_json_end(json_portion)?;
        let json_str = &json_portion[..=json_end];

        let mut tool_call: ToolCall = serde_json::from_str(json_str).ok()?;
        let mut modified = false;
        let mut chars_saved = 0;

        // Handle write_file tool calls
        if tool_call.tool == "write_file" {
            if let Some((saved, new_args)) =
                Self::thin_write_file_args(&tool_call.args, index, tmp_dir, file_prefix)
            {
                tool_call.args = new_args;
                modified = true;
                chars_saved += saved;
            }
        }

        // Handle str_replace tool calls
        if tool_call.tool == "str_replace" {
            if let Some((saved, new_args)) =
                Self::thin_str_replace_args(&tool_call.args, index, tmp_dir, file_prefix)
            {
                tool_call.args = new_args;
                modified = true;
                chars_saved += saved;
            }
        }

        if !modified {
            return None;
        }

        // Reconstruct the message
        let prefix = &content[..tool_call_start];
        let suffix = &content[tool_call_start + json_str.len()..];
        let new_json = serde_json::to_string(&tool_call).ok()?;
        let new_content = format!("{}{}{}", prefix, new_json, suffix);

        Some(ThinModification::ReplaceContent {
            index,
            new_content,
            chars_saved,
        })
    }

    /// Thin write_file args by saving content to file
    /// Returns (chars_saved, new_args) if thinned
    fn thin_write_file_args(
        args: &serde_json::Value,
        index: usize,
        tmp_dir: &str,
        file_prefix: &str,
    ) -> Option<(usize, serde_json::Value)> {
        let args_obj = args.as_object()?;
        let content_str = args_obj.get("content")?.as_str()?;
        let content_len = content_str.len();

        if content_len <= 500 {
            return None;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("{}_write_file_content_{}_{}.txt", file_prefix, timestamp, index);
        let file_path = format!("{}/{}", tmp_dir, filename);

        if std::fs::write(&file_path, content_str).is_err() {
            return None;
        }

        let mut new_args = args_obj.clone();
        new_args.insert(
            "content".to_string(),
            serde_json::Value::String(format!("<content saved to {}>", file_path)),
        );

        debug!(
            "Thinned write_file content {} ({} chars) to {}",
            index, content_len, file_path
        );

        Some((content_len, serde_json::Value::Object(new_args)))
    }

    /// Thin str_replace args by saving diff to file
    /// Returns (chars_saved, new_args) if thinned
    fn thin_str_replace_args(
        args: &serde_json::Value,
        index: usize,
        tmp_dir: &str,
        file_prefix: &str,
    ) -> Option<(usize, serde_json::Value)> {
        let args_obj = args.as_object()?;
        let diff_str = args_obj.get("diff")?.as_str()?;
        let diff_len = diff_str.len();

        if diff_len <= 500 {
            return None;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("{}_str_replace_diff_{}_{}.txt", file_prefix, timestamp, index);
        let file_path = format!("{}/{}", tmp_dir, filename);

        if std::fs::write(&file_path, diff_str).is_err() {
            return None;
        }

        let mut new_args = args_obj.clone();
        new_args.insert(
            "diff".to_string(),
            serde_json::Value::String(format!("<diff saved to {}>", file_path)),
        );

        debug!(
            "Thinned str_replace diff {} ({} chars) to {}",
            index, diff_len, file_path
        );

        Some((diff_len, serde_json::Value::Object(new_args)))
    }

    /// Build the result message for thinning operations
    fn build_thin_result_message(
        &self,
        scope: ThinScope,
        current_percentage: u32,
        leaned_count: usize,
        tool_call_leaned_count: usize,
        chars_saved: usize,
    ) -> (String, usize) {
        let scope_desc = match scope {
            ThinScope::FirstThird => "",
            ThinScope::All => " across entire history",
        };

        // Nothing was thinned
        if leaned_count == 0 && tool_call_leaned_count == 0 {
            let msg = format!(
                "ℹ Context {} triggered at {}% but no large tool results or tool calls found{}",
                scope.error_action(), current_percentage, scope_desc
            );
            return (msg, 0);
        }

        // Build description of what was thinned
        let what_thinned = match (leaned_count > 0, tool_call_leaned_count > 0) {
            (true, true) => format!("{} tool results + {} tool calls", leaned_count, tool_call_leaned_count),
            (true, false) => format!("{} tool results", leaned_count),
            (false, true) => format!("{} tool calls", tool_call_leaned_count),
            (false, false) => unreachable!(), // handled above
        };

        let msg = format!(
            "{} Context {} at {}%: {}{}, ~{} chars saved",
            scope.emoji(), scope.label(), current_percentage, what_thinned, scope_desc, chars_saved
        );
        (msg, chars_saved)
    }

    /// Recalculate token usage based on current conversation history
    fn recalculate_tokens(&mut self) {
        let mut total = 0;
        for message in &self.conversation_history {
            total += Self::estimate_tokens(&message.content);
        }
        self.used_tokens = total;

        debug!("Recalculated tokens after thinning: {} tokens", total);
    }

    /// Helper function to find the end of a JSON object
    pub fn find_json_end(json_str: &str) -> Option<usize> {
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in json_str.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => escape_next = true,
                '"' if !escape_next => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context_window() {
        let cw = ContextWindow::new(100_000);
        assert_eq!(cw.used_tokens, 0);
        assert_eq!(cw.total_tokens, 100_000);
        assert_eq!(cw.cumulative_tokens, 0);
        assert!(cw.conversation_history.is_empty());
    }

    #[test]
    fn test_percentage_used() {
        let mut cw = ContextWindow::new(100);
        cw.used_tokens = 50;
        assert!((cw.percentage_used() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_remaining_tokens() {
        let mut cw = ContextWindow::new(100);
        cw.used_tokens = 30;
        assert_eq!(cw.remaining_tokens(), 70);
    }

    #[test]
    fn test_should_compact_at_80_percent() {
        let mut cw = ContextWindow::new(100);
        cw.used_tokens = 79;
        assert!(!cw.should_compact());
        cw.used_tokens = 80;
        assert!(cw.should_compact());
    }

    #[test]
    fn test_should_compact_at_absolute_limit() {
        let mut cw = ContextWindow::new(1_000_000);
        cw.used_tokens = 150_001;
        assert!(cw.should_compact());
    }

    #[test]
    fn test_should_thin_thresholds() {
        let mut cw = ContextWindow::new(100);
        
        // Below 50% - should not thin
        cw.used_tokens = 49;
        assert!(!cw.should_thin());
        
        // At 50% - should thin (first time)
        cw.used_tokens = 50;
        assert!(cw.should_thin());
        
        // After thinning at 50%, shouldn't thin again until 60%
        cw.last_thinning_percentage = 50;
        cw.used_tokens = 55;
        assert!(!cw.should_thin());
        
        // At 60% - should thin again
        cw.used_tokens = 60;
        assert!(cw.should_thin());
    }

    #[test]
    fn test_estimate_tokens_regular_text() {
        let text = "Hello world, this is a test.";
        let tokens = ContextWindow::estimate_tokens(text);
        // ~28 chars / 4 * 1.1 = ~8 tokens
        assert!(tokens > 0 && tokens < 20);
    }

    #[test]
    fn test_estimate_tokens_code() {
        let code = "fn main() { println!(\"hello\"); }";
        let tokens = ContextWindow::estimate_tokens(code);
        // Code uses 3 chars per token estimate
        assert!(tokens > 0);
    }

    #[test]
    fn test_find_json_end() {
        assert_eq!(ContextWindow::find_json_end("{}"), Some(1));
        assert_eq!(ContextWindow::find_json_end(r#"{"a": 1}"#), Some(7));
        assert_eq!(ContextWindow::find_json_end(r#"{"a": {"b": 2}}"#), Some(14));
        assert_eq!(ContextWindow::find_json_end("{incomplete"), None);
    }

    #[test]
    fn test_thin_scope_properties() {
        assert_eq!(ThinScope::FirstThird.emoji(), "🥒");
        assert_eq!(ThinScope::All.emoji(), "🦴");
        assert_eq!(ThinScope::FirstThird.label(), "thinned");
        assert_eq!(ThinScope::All.label(), "skinnified");
    }
}
