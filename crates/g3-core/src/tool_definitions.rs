//! Tool definitions for the agent's available tools.
//!
//! This module contains the JSON schema definitions for all tools that can be
//! used by the agent when interacting with LLM providers that support native
//! tool calling.

use g3_providers::Tool;
use serde_json::json;

/// Configuration for which optional tool sets to enable
#[derive(Debug, Clone, Copy, Default)]
pub struct ToolConfig {
    pub webdriver: bool,
    pub computer_control: bool,
}

impl ToolConfig {
    pub fn new(webdriver: bool, computer_control: bool) -> Self {
        Self {
            webdriver,
            computer_control,
        }
    }
}

/// Create tool definitions for native tool calling providers.
///
/// Returns a vector of Tool definitions that describe the available tools
/// and their input schemas.
pub fn create_tool_definitions(config: ToolConfig) -> Vec<Tool> {
    let mut tools = create_core_tools();

    if config.webdriver {
        tools.extend(create_webdriver_tools());
    }

    tools
}

/// Create the core tools that are always available
fn create_core_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "shell".to_string(),
            description: "Execute shell commands".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        },
        Tool {
            name: "background_process".to_string(),
            description: "Launch a long-running process in the background (e.g., game servers, dev servers). The process runs independently and logs are captured to a file. Use the regular 'shell' tool to read logs (cat/tail), check status (ps), or stop the process (kill). Returns the PID and log file path.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "A unique name for this process (e.g., 'game_server', 'my_app'). Used to identify the process and its log file."
                    },
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute in the background"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Optional working directory. Defaults to current directory if not specified."
                    }
                },
                "required": ["name", "command"]
            }),
        },
        Tool {
            name: "read_file".to_string(),
            description: "Read the contents of a file. Optionally read a specific character range.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to read"
                    },
                    "start": {
                        "type": "integer",
                        "description": "Starting character position (0-indexed, inclusive). If omitted, reads from beginning."
                    },
                    "end": {
                        "type": "integer",
                        "description": "Ending character position (0-indexed, EXCLUSIVE). If omitted, reads to end of file."
                    }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "read_image".to_string(),
            description: "Read one or more image files and send them to the LLM for visual analysis. Supports PNG, JPEG, GIF, and WebP formats. Use this when you need to visually inspect images (e.g., find sprites, analyze UI, read diagrams). The images will be included in your next response for analysis.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of paths to image files to read"
                    }
                },
                "required": ["file_paths"]
            }),
        },
        Tool {
            name: "write_file".to_string(),
            description: "Write content to a file (creates or overwrites). You MUST provide all arguments".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            }),
        },
        Tool {
            name: "str_replace".to_string(),
            description: "Apply a unified diff to a file. Supports multiple hunks and context lines. Optionally constrain the search to a [start, end) character range (0-indexed; end is EXCLUSIVE). Useful to disambiguate matches or limit scope in large files.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to edit"
                    },
                    "diff": {
                        "type": "string",
                        "description": "A unified diff showing what to replace. Supports @@ hunk headers, context lines, and multiple hunks (---/+++ headers optional for minimal diffs)."
                    },
                    "start": {
                        "type": "integer",
                        "description": "Starting character position in the file (0-indexed, inclusive). If omitted, searches from beginning."
                    },
                    "end": {
                        "type": "integer",
                        "description": "Ending character position in the file (0-indexed, EXCLUSIVE - character at this position is NOT included). If omitted, searches to end of file."
                    }
                },
                "required": ["file_path", "diff"]
            }),
        },
        Tool {
            name: "final_output".to_string(),
            description: "Signal task completion with a detailed summary".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "A detailed summary in markdown of what was accomplished"
                    }
                },
                "required": ["summary"]
            }),
        },
        Tool {
            name: "take_screenshot".to_string(),
            description: "Capture a screenshot of a specific application window. You MUST specify the window_id parameter with the application name (e.g., 'Safari', 'Terminal', 'Google Chrome'). The tool will automatically use the native screencapture command with the application's window ID for a clean capture. Use list_windows first to identify available windows.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Filename for the screenshot (e.g., 'safari.png'). If a relative path is provided, the screenshot will be saved to ~/tmp or $TMPDIR. Use an absolute path to save elsewhere."
                    },
                    "window_id": {
                        "type": "string",
                        "description": "REQUIRED: Application name to capture (e.g., 'Safari', 'Terminal', 'Google Chrome'). The tool will capture the frontmost window of that application using its native window ID."
                    },
                    "region": {
                        "type": "object",
                        "properties": {
                            "x": {"type": "integer"},
                            "y": {"type": "integer"},
                            "width": {"type": "integer"},
                            "height": {"type": "integer"}
                        }
                    }
                },
                "required": ["path", "window_id"]
            }),
        },
        Tool {
            name: "todo_read".to_string(),
            description: "Read your current TODO list from todo.g3.md file in the session directory. Shows what tasks are planned and their status. Call this at the start of multi-step tasks to check for existing plans, and during execution to review progress before updating. TODO lists are scoped to the current session.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "todo_write".to_string(),
            description: "Create or update your TODO list in todo.g3.md file with a complete task plan. Use markdown checkboxes: - [ ] for pending, - [x] for complete. This tool replaces the entire file content, so always call todo_read first to preserve existing content. Essential for multi-step tasks. TODO lists are scoped to the current session.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The TODO list content to save. Use markdown checkbox format: - [ ] for incomplete tasks, - [x] for completed tasks. Support nested tasks with indentation."
                    }
                },
                "required": ["content"]
            }),
        },
        Tool {
            name: "code_coverage".to_string(),
            description: "Generate a code coverage report for the entire workspace using cargo llvm-cov. This runs all tests with coverage instrumentation and returns a summary of coverage statistics. Requires llvm-tools-preview and cargo-llvm-cov to be installed (they will be auto-installed if missing).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "code_search".to_string(),
            description: "Syntax-aware code search that understands code structure, not just text. Finds actual functions, classes, methods, and other code constructs - ignores matches in comments and strings. Much more accurate than grep for code searches. Supports batch searches (up to 20 parallel) with structured results and context lines. Languages: Rust, Python, JavaScript, TypeScript, Go, Java, C, C++, Kotlin. Uses tree-sitter query syntax.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "searches": {
                        "type": "array",
                        "maxItems": 20,
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "Label for this search." },
                                "query": { "type": "string", "description": "tree-sitter query in S-expression format (e.g., \"(function_item name: (identifier) @name)\")" },
                                "language": { "type": "string", "enum": ["rust", "python", "javascript", "typescript", "go", "java", "c", "cpp", "kotlin"], "description": "Programming language to search." },
                                "paths": { "type": "array", "items": { "type": "string" }, "description": "Paths/dirs to search. Defaults to current dir if empty." },
                                "context_lines": { "type": "integer", "minimum": 0, "maximum": 20, "default": 0, "description": "Lines of context to include around each match." }
                            },
                            "required": ["name", "query", "language"]
                        }
                    },
                    "max_concurrency": { "type": "integer", "minimum": 1, "default": 4 },
                    "max_matches_per_search": { "type": "integer", "minimum": 1, "default": 500 }
                },
                "required": ["searches"]
            }),
        },
    ]
}

/// Create WebDriver browser automation tools
fn create_webdriver_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "webdriver_start".to_string(),
            description: "Start a Safari WebDriver session for browser automation. Must be called before any other webdriver tools. Requires Safari's 'Allow Remote Automation' to be enabled in Develop menu.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "webdriver_navigate".to_string(),
            description: "Navigate to a URL in the browser".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to navigate to (must include protocol, e.g., https://)"
                    }
                },
                "required": ["url"]
            }),
        },
        Tool {
            name: "webdriver_get_url".to_string(),
            description: "Get the current URL of the browser".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "webdriver_get_title".to_string(),
            description: "Get the title of the current page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "webdriver_find_element".to_string(),
            description: "Find an element on the page by CSS selector and return its text content".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to find the element (e.g., 'h1', '.class-name', '#id')"
                    }
                },
                "required": ["selector"]
            }),
        },
        Tool {
            name: "webdriver_find_elements".to_string(),
            description: "Find all elements matching a CSS selector and return their text content".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to find elements"
                    }
                },
                "required": ["selector"]
            }),
        },
        Tool {
            name: "webdriver_click".to_string(),
            description: "Click an element on the page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to click"
                    }
                },
                "required": ["selector"]
            }),
        },
        Tool {
            name: "webdriver_send_keys".to_string(),
            description: "Type text into an input element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the input element"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type into the element"
                    },
                    "clear_first": {
                        "type": "boolean",
                        "description": "Whether to clear the element before typing (default: true)"
                    }
                },
                "required": ["selector", "text"]
            }),
        },
        Tool {
            name: "webdriver_execute_script".to_string(),
            description: "Execute JavaScript code in the browser and return the result".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "script": {
                        "type": "string",
                        "description": "JavaScript code to execute (use 'return' to return a value)"
                    }
                },
                "required": ["script"]
            }),
        },
        Tool {
            name: "webdriver_get_page_source".to_string(),
            description: "Get the rendered HTML source of the current page. Returns the current DOM state after JavaScript execution.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "max_length": {
                        "type": "integer",
                        "description": "Maximum length of HTML to return (default: 10000, use 0 for no truncation)"
                    },
                    "save_to_file": {
                        "type": "string",
                        "description": "Optional file path to save the HTML instead of returning it inline"
                    }
                },
                "required": []
            }),
        },
        Tool {
            name: "webdriver_screenshot".to_string(),
            description: "Take a screenshot of the browser window".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path where to save the screenshot (e.g., '/tmp/screenshot.png')"
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "webdriver_back".to_string(),
            description: "Navigate back in browser history".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "webdriver_forward".to_string(),
            description: "Navigate forward in browser history".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "webdriver_refresh".to_string(),
            description: "Refresh the current page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "webdriver_quit".to_string(),
            description: "Close the browser and end the WebDriver session".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_tools_count() {
        let tools = create_core_tools();
        // Should have the core tools: shell, background_process, read_file, read_image,
        // write_file, str_replace, final_output, take_screenshot,
        // todo_read, todo_write, code_coverage, code_search (12 total)
        assert_eq!(tools.len(), 12);
    }

    #[test]
    fn test_webdriver_tools_count() {
        let tools = create_webdriver_tools();
        // 15 webdriver tools
        assert_eq!(tools.len(), 15);
    }

    #[test]
    fn test_create_tool_definitions_core_only() {
        let config = ToolConfig::default();
        let tools = create_tool_definitions(config);
        assert_eq!(tools.len(), 12);
    }

    #[test]
    fn test_create_tool_definitions_all_enabled() {
        let config = ToolConfig::new(true, true);
        let tools = create_tool_definitions(config);
        // 12 core + 15 webdriver = 27
        assert_eq!(tools.len(), 27);
    }

    #[test]
    fn test_tool_has_required_fields() {
        let tools = create_core_tools();
        for tool in tools {
            assert!(!tool.name.is_empty(), "Tool name should not be empty");
            assert!(!tool.description.is_empty(), "Tool description should not be empty");
            assert!(tool.input_schema.is_object(), "Tool input_schema should be an object");
        }
    }
}
