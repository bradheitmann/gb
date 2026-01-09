//! Streaming markdown formatter with tag counting.
//!
//! This module provides a state machine that buffers markdown constructs
//! and emits formatted output as soon as constructs are complete.
//!
//! Design principles:
//! - Raw text streams immediately
//! - Inline constructs (bold, italic, inline code) buffer until closed
//! - Block constructs (code blocks, tables, blockquotes) buffer until complete
//! - Proper delimiter counting handles nested/overlapping markers
//! - Escape sequences are respected

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use termimad::MadSkin;

/// Lazily loaded syntax set for code highlighting.
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Types of markdown delimiters we track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DelimiterKind {
    /// `[` - link text start
    LinkBracket,
    /// `**` - strong/bold
    DoubleStar,
    /// `*` - emphasis/italic  
    SingleStar,
    /// `__` - strong/bold (underscore variant)
    DoubleUnderscore,
    /// `_` - emphasis/italic (underscore variant)
    SingleUnderscore,
    /// `` ` `` - inline code
    Backtick,
    /// `~~` - strikethrough
    DoubleSquiggle,
}

/// Block-level constructs that require multi-line buffering.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockState {
    /// Not in any special block
    None,
    /// In a fenced code block, with optional language
    CodeBlock { lang: Option<String>, fence: String },
    /// In a blockquote (lines starting with >)
    BlockQuote,
    /// In a table (lines with |)
    Table,
}

/// The streaming markdown formatter.
/// 
/// Feed it chunks of text, and it will emit formatted output
/// as soon as markdown constructs are complete.
pub struct StreamingMarkdownFormatter {
    /// Stack of open inline delimiters with their positions in the buffer
    delimiter_stack: Vec<(DelimiterKind, usize)>,
    
    /// Current block-level state
    block_state: BlockState,
    
    /// Whether the previous character was a backslash (for escapes)
    escape_next: bool,
    
    /// Whether the last character added to current_line was escaped
    last_char_escaped: bool,
    
    /// The termimad skin for formatting
    skin: MadSkin,
    
    /// Pending output that's ready to emit
    pending_output: VecDeque<String>,
    
    /// Track if we're at the start of a line (for block detection)
    at_line_start: bool,
    
    /// Track if we just emitted a list bullet and should skip the next space
    skip_next_space: bool,
    
    /// Accumulated lines for block constructs
    block_buffer: Vec<String>,
    
    /// Current line being built
    current_line: String,
}

impl StreamingMarkdownFormatter {
    pub fn new(skin: MadSkin) -> Self {
        Self {
            delimiter_stack: Vec::new(),
            block_state: BlockState::None,
            escape_next: false,
            last_char_escaped: false,
            skin,
            pending_output: VecDeque::new(),
            at_line_start: true,
            skip_next_space: false,
            block_buffer: Vec::new(),
            current_line: String::new(),
        }
    }
    
    /// Process an incoming chunk of text.
    /// Returns formatted output that's ready to display.
    pub fn process(&mut self, chunk: &str) -> String {
        for ch in chunk.chars() {
            self.process_char(ch);
        }
        self.collect_output()
    }
    
    /// Signal end of stream and flush any remaining content.
    pub fn finish(&mut self) -> String {
        // Flush any incomplete constructs as-is
        self.flush_incomplete();
        self.collect_output()
    }
    
    /// Process a single character.
    fn process_char(&mut self, ch: char) {
        // Skip space after list bullet
        if self.skip_next_space {
            self.skip_next_space = false;
            if ch == ' ' {
                return;
            }
        }
        
        // Handle escape sequences
        if self.escape_next {
            self.escape_next = false;
            self.last_char_escaped = true;
            self.current_line.push(ch);
            self.at_line_start = false;
            return;
        }
        
        if ch == '\\' {
            self.escape_next = true;
            self.last_char_escaped = false;
            self.current_line.push(ch);
            self.at_line_start = false;
            return;
        }
        
        // Handle based on current block state
        match &self.block_state {
            BlockState::CodeBlock { .. } => self.process_in_code_block(ch),
            BlockState::BlockQuote => self.process_in_blockquote(ch),
            BlockState::Table => self.process_in_table(ch),
            BlockState::None => self.process_normal(ch),
        }
    }
    
    /// Process character in normal (non-block) mode.
    fn process_normal(&mut self, ch: char) {
        // Check for block-level constructs at line start
        if self.at_line_start {
            // Handle - at line start: could be list item or horizontal rule
            // Buffer it and decide later
            if ch == '-' && self.current_line.chars().all(|c| c.is_whitespace() || c == '-') {
                self.current_line.push(ch);
                // Keep buffering - will decide at space or newline
                return;
            }
            
            // If we have buffered a single dash (possibly with leading whitespace) and now see a space, it's a list item
            if ch == ' ' && self.current_line.trim() == "-" {
                // Extract indentation
                let indent: String = self.current_line.chars().take_while(|c| c.is_whitespace()).collect();
                self.current_line.clear();
                if !indent.is_empty() {
                    self.pending_output.push_back(indent);
                }
                self.pending_output.push_back("• ".to_string());
                self.at_line_start = false;
                return;
            }
            
            // Handle ordered lists: digit(s) followed by . at line start
            if ch == '.' && !self.current_line.is_empty() 
                && self.current_line.chars().all(|c| c.is_ascii_digit() || c.is_whitespace())
                && self.current_line.chars().any(|c| c.is_ascii_digit()) {
                // This is an ordered list item like "1." or "  2."
                // Emit the number with period immediately
                self.current_line.push(ch);
                self.current_line.push(' ');
                self.pending_output.push_back(self.current_line.clone());
                self.current_line.clear();
                self.at_line_start = false;
                return;
            }
            
            if ch == '`' {
                self.current_line.push(ch);
                // Check if this might be starting a code fence
                if self.current_line.starts_with("```") {
                    // Don't emit yet - wait for the full fence line
                } else if self.current_line == "`" || self.current_line == "``" {
                    // Might become a fence, keep buffering
                }
                return;
            } else if ch == '>' && self.current_line.is_empty() {
                // Starting a blockquote
                self.block_state = BlockState::BlockQuote;
                self.current_line.push(ch);
                return;
            } else if ch == '|' && self.current_line.is_empty() {
                // Might be starting a table
                self.block_state = BlockState::Table;
                self.current_line.push(ch);
                return;
            } else if ch == '#' && self.current_line.is_empty() {
                // Header - buffer until newline
                self.current_line.push(ch);
                self.at_line_start = false;
                return;
            }
        }
        
        // Handle newlines
        if ch == '\n' {
            self.handle_newline();
            return;
        }
        
        // Check for inline delimiters
        if let Some(delim) = self.check_delimiter(ch) {
            self.at_line_start = false;
            self.handle_delimiter(delim, ch);
        } else if self.at_line_start && ch.is_whitespace() {
            // Keep at_line_start true for leading whitespace (for nested lists)
            self.current_line.push(ch);
            self.last_char_escaped = false;
            // Don't set at_line_start = false yet
        } else {
            self.at_line_start = false;
            self.last_char_escaped = false;
            
            // Check if we can stream immediately:
            // - No open delimiters
            // - Buffer is empty (we've been streaming)
            // - Current char is not a potential delimiter start
            // - Buffer doesn't start with # (header)
            // - Buffer doesn't start with ` (potential code fence)
            // - Buffer doesn't contain unclosed link bracket
            let in_header = self.current_line.starts_with('#');
            let in_potential_fence = self.current_line.starts_with('`');
            // A complete link ends with ) after ](, so buffer until then
            let has_bracket = self.current_line.contains("[");
            let link_complete = self.current_line.contains("](") && self.current_line.ends_with(")");
            let in_potential_link = has_bracket && !link_complete;
            
            if self.delimiter_stack.is_empty() && !in_header && !in_potential_fence 
                && !in_potential_link && !is_potential_delimiter_start(ch) 
            {
                // Stream immediately - but format any buffered content first if needed
                self.current_line.push(ch);
                // Check if buffer has any formatting that needs processing
                let has_formatting = self.current_line.contains(['[', '*', '_', '`', '~']);
                if has_formatting {
                    let formatted = self.format_inline_content(&self.current_line);
                    self.pending_output.push_back(formatted);
                } else {
                    self.pending_output.push_back(self.current_line.clone());
                }
                self.current_line.clear();
            } else {
                self.current_line.push(ch);
            }
        }
    }
    
    /// Check if current char (possibly with lookahead in buffer) forms a delimiter.
    fn check_delimiter(&self, ch: char) -> Option<DelimiterKind> {
        let last_char = self.current_line.chars().last();
        
        // If the last character was escaped, it can't be part of a delimiter
        if self.last_char_escaped {
            return None;
        }
        
        match ch {
            '*' => {
                if last_char == Some('*') {
                    Some(DelimiterKind::DoubleStar)
                } else {
                    None // Will check on next char
                }
            }
            '_' => {
                if last_char == Some('_') {
                    Some(DelimiterKind::DoubleUnderscore)
                } else {
                    None
                }
            }
            '`' => Some(DelimiterKind::Backtick),
            '~' => {
                if last_char == Some('~') {
                    Some(DelimiterKind::DoubleSquiggle)
                } else {
                    None
                }
            }
            '[' => Some(DelimiterKind::LinkBracket),
            ']' => {
                // Only treat as closing if we have an open bracket
                if self.delimiter_stack.iter().any(|(d, _)| *d == DelimiterKind::LinkBracket) {
                    Some(DelimiterKind::LinkBracket)
                } else {
                    None
                }
            }
            _ => {
                // Check if previous char was a single delimiter
                // But make sure it's not part of a double delimiter (e.g., ** or __)
                let second_last = if self.current_line.len() >= 2 {
                    self.current_line.chars().rev().nth(1)
                } else {
                    None
                };
                
                match last_char {
                    Some('*') => {
                        // Previous * was a single star only if char before it wasn't also *
                        if second_last != Some('*') {
                            Some(DelimiterKind::SingleStar)
                        } else {
                            None
                        }
                    }
                    Some('_') => {
                        if second_last != Some('_') {
                            Some(DelimiterKind::SingleUnderscore)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        }
    }
    
    /// Handle a detected delimiter.
    fn handle_delimiter(&mut self, delim: DelimiterKind, ch: char) {
        // Don't modify the buffer - we want to preserve raw markdown
        // for regex-based formatting in format_inline_content
        
        // Check if this closes an existing delimiter
        if let Some(pos) = self.find_matching_open_delimiter(delim) {
            // Close the delimiter - the content is complete
            self.delimiter_stack.truncate(pos);
            self.current_line.push(ch);
            self.last_char_escaped = false;
            
            // If stack is now empty AND we're not inside a potential link, emit
            // A potential link is indicated by an unclosed '[' in the buffer
            // that hasn't been followed by '](' yet
            let in_potential_link = self.current_line.contains('[') 
                && !self.current_line.contains("](")
                && !self.current_line.ends_with(')');
            
            // Don't emit yet if this could be a horizontal rule (all asterisks/dashes/underscores)
            // We need to wait for newline to know for sure
            let could_be_hr = self.current_line.chars().all(|c| c == '*' || c == '-' || c == '_')
                && self.current_line.len() >= 2;  // At least ** or -- or __
            
            if self.delimiter_stack.is_empty() && !in_potential_link && !could_be_hr {
                self.emit_formatted_inline();
            }
        } else {
            // Open a new delimiter
            let pos = self.current_line.len();
            self.delimiter_stack.push((delim, pos));
            self.current_line.push(ch);
            self.last_char_escaped = false;
        }
    }
    
    /// Find a matching open delimiter in the stack.
    fn find_matching_open_delimiter(&self, delim: DelimiterKind) -> Option<usize> {
        // Search from the end (most recent) to find matching delimiter
        for (i, (d, _)) in self.delimiter_stack.iter().enumerate().rev() {
            if *d == delim {
                return Some(i);
            }
        }
        None
    }
    
    /// Handle a newline character.
    fn handle_newline(&mut self) {
        // Check if we were building a code fence
        if self.current_line.starts_with("```") {
            let lang = self.current_line[3..].trim().to_string();
            let lang = if lang.is_empty() { None } else { Some(lang) };
            self.block_state = BlockState::CodeBlock {
                lang,
                fence: "```".to_string(),
            };
            self.current_line.clear();
            self.at_line_start = true;
            return;
        }
        
        self.current_line.push('\n');
        
        // Always emit the line at newline, even if there are unclosed delimiters
        // This handles cases like unclosed inline code at end of line
        // The format_inline_content function will handle unclosed delimiters gracefully
        self.emit_formatted_inline();
        
        self.at_line_start = true;
    }
    
    /// Process character while in a code block.
    fn process_in_code_block(&mut self, ch: char) {
        if ch == '\n' {
            // Check if this line closes the code block
            // Only close if the fence is at the start of the line with at most 3 spaces
            // of indentation (per CommonMark spec). This prevents content like "    ```"
            // (4+ spaces, which is code indentation) from closing the block.
            let trimmed = self.current_line.trim_start();
            let leading_spaces = self.current_line.len() - trimmed.len();
            if trimmed == "```" && leading_spaces <= 3 {
                // Emit the entire code block
                self.emit_code_block();
                self.block_state = BlockState::None;
                self.current_line.clear();
            } else {
                self.block_buffer.push(self.current_line.clone());
                self.current_line.clear();
            }
            self.at_line_start = true;
        } else {
            self.current_line.push(ch);
            self.at_line_start = false;
        }
    }
    
    /// Process character while in a blockquote.
    fn process_in_blockquote(&mut self, ch: char) {
        if ch == '\n' {
            self.block_buffer.push(self.current_line.clone());
            self.current_line.clear();
            self.at_line_start = true;
        } else if self.at_line_start && ch != '>' && !ch.is_whitespace() {
            // Line doesn't start with > - blockquote ended
            self.emit_blockquote();
            self.block_state = BlockState::None;
            self.current_line.push(ch);
            self.at_line_start = false;
        } else {
            self.current_line.push(ch);
            self.at_line_start = false;
        }
    }
    
    /// Process character while in a table.
    fn process_in_table(&mut self, ch: char) {
        if ch == '\n' {
            self.block_buffer.push(self.current_line.clone());
            self.current_line.clear();
            self.at_line_start = true;
        } else if self.at_line_start && ch != '|' && !ch.is_whitespace() {
            // Line doesn't start with | - table ended
            self.emit_table();
            self.block_state = BlockState::None;
            self.current_line.push(ch);
            self.at_line_start = false;
        } else {
            self.current_line.push(ch);
            self.at_line_start = false;
        }
    }
    
    /// Emit formatted inline content.
    fn emit_formatted_inline(&mut self) {
        if self.current_line.is_empty() {
            return;
        }
        
        let line = &self.current_line;
        
        // Check for headers
        if line.starts_with('#') {
            let formatted = self.format_header(line);
            self.pending_output.push_back(formatted);
            self.current_line.clear();
            self.delimiter_stack.clear();
            return;
        }
        
        // Check for horizontal rule (---, ***, ___) - only if nothing else emitted on this line
        // This prevents "****" from being treated as "***" + "*" horizontal rule
        if self.pending_output.is_empty() || self.pending_output.back().map(|s| s.ends_with('\n')).unwrap_or(true) {
            let trimmed = line.trim();
            // Must be exactly 3+ of the same character, not mixed
            let is_hr = (trimmed == "---" || trimmed == "***" || trimmed == "___")
                || (trimmed.len() >= 3 && trimmed.chars().all(|c| c == '-'))
                || (trimmed.len() >= 3 && trimmed.chars().all(|c| c == '_'));
            if is_hr {
                // Emit a horizontal rule
                self.pending_output.push_back("\x1b[2m────────────────────────────────────────\x1b[0m\n".to_string());
                self.current_line.clear();
                self.delimiter_stack.clear();
                return;
            }
        }
        
        // Format inline content (bold, italic, code, strikethrough, links)
        let formatted = self.format_inline_content(line);
        self.pending_output.push_back(formatted);
        self.current_line.clear();
        self.delimiter_stack.clear();
    }
    
    /// Format a header line.
    fn format_header(&self, line: &str) -> String {
        let mut level = 0;
        let mut chars = line.chars().peekable();
        
        // Count # characters
        while chars.peek() == Some(&'#') {
            level += 1;
            chars.next();
        }
        
        // Skip whitespace after #
        while chars.peek().map(|c| c.is_whitespace() && *c != '\n').unwrap_or(false) {
            chars.next();
        }
        
        let content: String = chars.collect();
        let content = content.trim_end();
        
        // Format based on level (magenta, bold for h1/h2)
        match level {
            1 => format!("\x1b[1;35m{}\x1b[0m\n", content),  // Bold magenta
            2 => format!("\x1b[35m{}\x1b[0m\n", content),    // Magenta
            _ => format!("\x1b[35m{}\x1b[0m\n", content),    // Magenta for h3+
        }
    }
    
    
    /// Format inline content with bold, italic, code, strikethrough, and links.
    fn format_inline_content(&self, line: &str) -> String {
        // Use regex-based replacement for inline formatting
        let mut result = line.to_string();
        
        // First, handle escaped characters: \* \_ \` \[ \] \~
        // Replace with placeholder that doesn't contain the original char
        // Use different codes for each: *=1, _=2, `=3, [=4, ]=5, ~=6
        let escape_re = regex::Regex::new(r"\\\*").unwrap();
        result = escape_re.replace_all(&result, "\x00E1\x00").to_string();
        let escape_re = regex::Regex::new(r"\\_").unwrap();
        result = escape_re.replace_all(&result, "\x00E2\x00").to_string();
        let escape_re = regex::Regex::new(r"\\`").unwrap();
        result = escape_re.replace_all(&result, "\x00E3\x00").to_string();
        let escape_re = regex::Regex::new(r"\\\[").unwrap();
        result = escape_re.replace_all(&result, "\x00E4\x00").to_string();
        let escape_re = regex::Regex::new(r"\\\]").unwrap();
        result = escape_re.replace_all(&result, "\x00E5\x00").to_string();
        let escape_re = regex::Regex::new(r"\\~").unwrap();
        result = escape_re.replace_all(&result, "\x00E6\x00").to_string();
        
        // Process links [text](url) -> text (in cyan, underlined)  
        // Allow any characters inside the brackets including backticks
        let link_re = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        result = link_re.replace_all(&result, |caps: &regex::Captures| {
            let text = &caps[1];
            // Format any inline code within the link text
            let formatted_text = format_inline_code_only(text);
            format!("\x1b[36;4m{}\x1b[0m", formatted_text)
        }).to_string();
        
        // Process inline code `code` -> code (in orange)
        let code_re = regex::Regex::new(r"`([^`]+)`").unwrap();
        result = code_re.replace_all(&result, |caps: &regex::Captures| {
            let code = &caps[1];
            format!("\x1b[38;2;216;177;114m{}\x1b[0m", code)
        }).to_string();
        
        // Handle unclosed inline code at end of line: `code without closing backtick
        // This renders the content after the backtick in orange and removes the backtick
        let unclosed_code_re = regex::Regex::new(r"`([^`]+)$").unwrap();
        result = unclosed_code_re.replace_all(&result, |caps: &regex::Captures| {
            let code = &caps[1];
            format!("\x1b[38;2;216;177;114m{}\x1b[0m", code)
        }).to_string();
        
        // Process strikethrough ~~text~~ -> text (with strikethrough)
        let strike_re = regex::Regex::new(r"~~([^~]+)~~").unwrap();
        result = strike_re.replace_all(&result, |caps: &regex::Captures| {
            let text = &caps[1];
            format!("\x1b[9m{}\x1b[0m", text)
        }).to_string();
        
        // Process italic *text* -> text (in cyan italic)
        // Handle italic with potential nested bold: *italic with **bold** inside*
        // We need to be careful not to match ** as italic delimiters
        // Must be processed BEFORE bold so we can detect ** inside *...*
        result = process_italic_with_nested_bold(&result);
        
        // Process bold **text** -> text (in green bold)
        // Allow any characters inside including single asterisks for nested italic
        let bold_re = regex::Regex::new(r"\*\*(.+?)\*\*").unwrap();
        result = bold_re.replace_all(&result, |caps: &regex::Captures| {
            let text = &caps[1];
            // Process nested italic within bold
            let inner = format_nested_italic(text);
            format!("\x1b[1;32m{}\x1b[0m", inner)
        }).to_string();
        
        // Restore escaped characters (remove the placeholder markers)
        result = result.replace("\x00E1\x00", "*");
        result = result.replace("\x00E2\x00", "_");
        result = result.replace("\x00E3\x00", "`");
        result = result.replace("\x00E4\x00", "[");
        result = result.replace("\x00E5\x00", "]");
        result = result.replace("\x00E6\x00", "~");
        
        result
    }
    fn emit_code_block(&mut self) {
        let lang = if let BlockState::CodeBlock { lang, .. } = &self.block_state {
            lang.clone()
        } else {
            None
        };
        
        // Emit language label
        if let Some(ref l) = lang {
            self.pending_output
                .push_back(format!("\x1b[2;3m{}\x1b[0m\n", l));
        }
        
        // Highlight the code
        let code = self.block_buffer.join("\n");
        let highlighted = highlight_code(&code, lang.as_deref());
        self.pending_output.push_back(highlighted);
        self.pending_output.push_back("\n".to_string());
        
        self.block_buffer.clear();
    }
    
    /// Emit a complete blockquote.
    fn emit_blockquote(&mut self) {
        let content = self.block_buffer.join("\n");
        let formatted = format!("{}", self.skin.term_text(&content));
        self.pending_output.push_back(formatted);
        self.block_buffer.clear();
    }
    
    /// Emit a complete table.
    fn emit_table(&mut self) {
        let content = self.block_buffer.join("\n");
        let formatted = format!("{}", self.skin.term_text(&content));
        self.pending_output.push_back(formatted);
        self.block_buffer.clear();
    }
    
    /// Flush any incomplete constructs.
    fn flush_incomplete(&mut self) {
        // Emit any remaining block content
        match &self.block_state {
            BlockState::CodeBlock { .. } => {
                // Unclosed code block - emit as-is
                if !self.block_buffer.is_empty() || !self.current_line.is_empty() {
                    if !self.current_line.is_empty() {
                        self.block_buffer.push(self.current_line.clone());
                    }
                    self.emit_code_block();
                }
            }
            BlockState::BlockQuote => {
                if !self.current_line.is_empty() {
                    self.block_buffer.push(self.current_line.clone());
                }
                if !self.block_buffer.is_empty() {
                    self.emit_blockquote();
                }
            }
            BlockState::Table => {
                if !self.current_line.is_empty() {
                    self.block_buffer.push(self.current_line.clone());
                }
                if !self.block_buffer.is_empty() {
                    self.emit_table();
                }
            }
            BlockState::None => {}
        }
        
        self.block_state = BlockState::None;
        
        // Emit any remaining inline content
        if !self.current_line.is_empty() {
            // Even with unclosed delimiters, emit what we have
            let formatted = self.format_inline_content(&self.current_line.clone());
            self.pending_output.push_back(formatted);
            self.current_line.clear();
        }
        
        self.delimiter_stack.clear();
    }
    
    /// Collect all pending output into a single string.
    fn collect_output(&mut self) -> String {
        let mut output = String::new();
        while let Some(s) = self.pending_output.pop_front() {
            output.push_str(&s);
        }
        output
    }
}

/// Format only inline code within text (used for nested formatting in links)
fn format_inline_code_only(text: &str) -> String {
    let code_re = regex::Regex::new(r"`([^`]+)`").unwrap();
    code_re.replace_all(text, |caps: &regex::Captures| {
        let code = &caps[1];
        format!("\x1b[38;2;216;177;114m{}\x1b[0m", code)
    }).to_string()
}

/// Format nested italic within bold text
fn format_nested_italic(text: &str) -> String {
    let italic_re = regex::Regex::new(r"\*([^*]+)\*").unwrap();
    italic_re.replace_all(text, |caps: &regex::Captures| {
        let inner = &caps[1];
        format!("\x1b[3;36m{}\x1b[0m\x1b[1;32m", inner)  // italic, then restore bold
    }).to_string()
}

/// Format nested bold within italic text
fn format_nested_bold(text: &str) -> String {
    let bold_re = regex::Regex::new(r"\*\*(.+?)\*\*").unwrap();
    bold_re.replace_all(text, |caps: &regex::Captures| {
        let inner = &caps[1];
        format!("\x1b[1;32m{}\x1b[0m\x1b[3;36m", inner)  // bold, then restore italic
    }).to_string()
}

/// Process italic text that may contain nested bold
/// Matches *text* where the * is not part of **
fn process_italic_with_nested_bold(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        // Check for single * (not **)
        if chars[i] == '*' && (i + 1 >= chars.len() || chars[i + 1] != '*') 
            && (i == 0 || chars[i - 1] != '*') 
        {
            // Found opening single *, look for closing single *
            let start = i + 1;
            let mut end = None;
            let mut j = start;
            
            while j < chars.len() {
                if chars[j] == '*' && (j + 1 >= chars.len() || chars[j + 1] != '*')
                    && (j == 0 || chars[j - 1] != '*')
                {
                    end = Some(j);
                    break;
                }
                j += 1;
            }
            
            if let Some(end_pos) = end {
                // Found matching closing *, format as italic
                let inner: String = chars[start..end_pos].iter().collect();
                // Process nested bold within the italic content
                let formatted_inner = format_nested_bold(&inner);
                result.push_str(&format!("\x1b[3;36m{}\x1b[0m", formatted_inner));
                i = end_pos + 1;
            } else {
                // No closing *, just output the *
                result.push(chars[i]);
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    
    result
}

/// Check if a character could start a markdown delimiter
fn is_potential_delimiter_start(ch: char) -> bool {
    matches!(ch, '*' | '_' | '`' | '~' | '[' | ']' | '#')
}

/// Highlight code with syntect.
fn highlight_code(code: &str, lang: Option<&str>) -> String {
    // Map language aliases to syntect-recognized names
    let normalized_lang = lang.map(|l| match l.to_lowercase().as_str() {
        // Lisp family - syntect's "Lisp" syntax handles these well
        "racket" | "rkt" => "lisp",
        "elisp" | "emacs-lisp" => "lisp",
        "scheme" => "lisp",
        "common-lisp" | "cl" => "lisp",
        // Other common aliases
        "shell" | "sh" => "bash",
        "zsh" => "bash",
        "dockerfile" => "bash",
        _ => l,
    });

    let syntax = lang
        .and_then(|_| normalized_lang.and_then(|l| SYNTAX_SET.find_syntax_by_token(l)))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
    
    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);
    
    let mut output = String::new();
    
    for line in LinesWithEndings::from(code) {
        match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(ranges) => {
                output.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
            }
            Err(_) => {
                output.push_str(line);
            }
        }
    }
    
    output.push_str("\x1b[0m");
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn make_formatter() -> StreamingMarkdownFormatter {
        let skin = MadSkin::default();
        StreamingMarkdownFormatter::new(skin)
    }
    
    #[test]
    fn test_plain_text_streams_immediately() {
        let mut fmt = make_formatter();
        let output = fmt.process("hello world\n");
        assert!(!output.is_empty());
        assert!(output.contains("hello world"));
    }
    
    #[test]
    fn test_bold_buffers_until_closed() {
        let mut fmt = make_formatter();
        
        // Open bold - should buffer
        let output1 = fmt.process("**bold");
        assert!(output1.is_empty(), "Should buffer until closed");
        
        // Close bold - should emit
        let output2 = fmt.process("**\n");
        assert!(!output2.is_empty(), "Should emit when closed");
    }
    
    #[test]
    fn test_code_block_buffers() {
        let mut fmt = make_formatter();
        
        // Start code block
        let o1 = fmt.process("```rust\n");
        assert!(o1.is_empty(), "Code fence should buffer");
        
        // Code content
        let o2 = fmt.process("fn main() {}\n");
        assert!(o2.is_empty(), "Code content should buffer");
        
        // Close code block
        let o3 = fmt.process("```\n");
        assert!(!o3.is_empty(), "Should emit on close");
        assert!(o3.contains("\x1b["), "Should have ANSI codes");
    }
    
    #[test]
    fn test_escape_sequences() {
        let mut fmt = make_formatter();
        
        // Escaped asterisks should not start bold
        let output = fmt.process("\\*not bold\\*\n");
        assert!(!output.is_empty());
        // The backslashes and asterisks should pass through
    }
    
    #[test]
    fn test_nested_delimiters() {
        let mut fmt = make_formatter();
        
        // **bold *italic* still bold**
        let output = fmt.process("**bold *italic* still bold**\n");
        assert!(!output.is_empty());
    }
    
    #[test]
    fn test_inline_code() {
        let mut fmt = make_formatter();
        
        let output = fmt.process("use `code` here\n");
        assert!(!output.is_empty());
    }
    
    #[test]
    fn test_finish_flushes_incomplete() {
        let mut fmt = make_formatter();
        
        // Unclosed bold
        let o1 = fmt.process("**unclosed bold");
        assert!(o1.is_empty());
        
        // Finish should flush
        let o2 = fmt.finish();
        assert!(!o2.is_empty());
        assert!(o2.contains("unclosed bold"));
    }
}
