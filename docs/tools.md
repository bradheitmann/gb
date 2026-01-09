# G3 Tools Reference

**Last updated**: January 2025  
**Source of truth**: `crates/g3-core/src/tool_definitions.rs`, `crates/g3-core/src/tools/`

## Purpose

This document describes all tools available to the G3 agent. Tools are the primary mechanism by which G3 interacts with the filesystem, executes commands, and automates tasks.

## Tool Categories

| Category | Tools | Enabled By |
|----------|-------|------------|
| **Core** | shell, read_file, write_file, str_replace, final_output, background_process | Always |
| **Images** | read_image, take_screenshot | Always |
| **Task Management** | todo_read, todo_write | Always |
| **Code Intelligence** | code_search, code_coverage | Always |
| **WebDriver** | webdriver_* (12 tools) | `--webdriver` or `--chrome-headless` |
| **Computer Control** | mouse_click, type_text, find_element, list_windows | `computer_control.enabled = true` |

---

## Core Tools

### shell

Execute shell commands.

**Parameters**:
- `command` (string, required): The shell command to execute

**Example**:
```json
{"tool": "shell", "args": {"command": "ls -la"}}
```

**Notes**:
- Commands run in the current working directory
- Output is streamed in real-time
- Both stdout and stderr are captured
- Exit code is reported

---

### background_process

Launch a long-running process in the background.

**Parameters**:
- `name` (string, required): Unique name for the process (e.g., "game_server")
- `command` (string, required): Shell command to execute
- `working_dir` (string, optional): Working directory

**Example**:
```json
{"tool": "background_process", "args": {"name": "dev_server", "command": "npm run dev"}}
```

**Returns**: PID and log file path

**Notes**:
- Process runs independently of the agent
- Logs are captured to a file
- Use `shell` to read logs (`tail`), check status (`ps`), or stop (`kill`)

---

### read_file

Read file contents with optional character range.

**Parameters**:
- `file_path` (string, required): Path to the file
- `start` (integer, optional): Starting character position (0-indexed, inclusive)
- `end` (integer, optional): Ending character position (0-indexed, exclusive)

**Example**:
```json
{"tool": "read_file", "args": {"file_path": "src/main.rs", "start": 0, "end": 1000}}
```

**Notes**:
- Supports tilde expansion (`~`)
- Reports file size and line count

---

### read_image

Read image files for visual analysis by the LLM.

**Parameters**:
- `file_paths` (array of strings, required): Paths to image files

**Example**:
```json
{"tool": "read_image", "args": {"file_paths": ["screenshot.png", "diagram.jpg"]}}
```

**Supported formats**: PNG, JPEG, GIF, WebP

**Notes**:
- Images are sent to the LLM for visual analysis
- Use for inspecting sprites, UI screenshots, diagrams, etc.

---

### write_file

Create or overwrite a file.

**Parameters**:
- `file_path` (string, required): Path to the file
- `content` (string, required): Content to write

**Example**:
```json
{"tool": "write_file", "args": {"file_path": "hello.txt", "content": "Hello, world!"}}
```

**Notes**:
- Creates parent directories if needed
- Overwrites existing files
- Reports bytes written

---

### str_replace

Apply a unified diff to a file.

**Parameters**:
- `file_path` (string, required): Path to the file
- `diff` (string, required): Unified diff with context lines
- `start` (integer, optional): Starting character position to constrain search
- `end` (integer, optional): Ending character position to constrain search

**Example**:
```json
{"tool": "str_replace", "args": {
  "file_path": "src/main.rs",
  "diff": "@@ -10,3 +10,4 @@\n fn main() {\n     println!(\"Hello\");\n+    println!(\"World\");\n }"
}}
```

**Notes**:
- Supports multiple hunks
- Context lines help locate the correct position
- Use `start`/`end` to disambiguate when multiple matches exist
- `---/+++` headers are optional for minimal diffs

---

### final_output

Signal task completion with a summary.

**Parameters**:
- `summary` (string, required): Markdown summary of what was accomplished

**Example**:
```json
{"tool": "final_output", "args": {"summary": "## Completed\n\n- Created user authentication module\n- Added unit tests\n- Updated documentation"}}
```

**Notes**:
- Ends the current task
- Summary is displayed to the user
- In autonomous mode, triggers coach review

---

## Image & Screenshot Tools

### take_screenshot

Capture a screenshot of an application window.

**Parameters**:
- `path` (string, required): Filename for the screenshot
- `window_id` (string, required): Application name (e.g., "Safari", "Terminal")
- `region` (object, optional): `{x, y, width, height}` to capture a region

**Example**:
```json
{"tool": "take_screenshot", "args": {"path": "safari.png", "window_id": "Safari"}}
```

**Notes**:
- Use `list_windows` first to identify available windows
- Relative paths save to `~/tmp` or `$TMPDIR`
- Uses native screencapture on macOS

---


## Task Management Tools

### todo_read

Read the current TODO list.

**Parameters**: None

**Example**:
```json
{"tool": "todo_read", "args": {}}
```

**Notes**:
- TODO lists are session-scoped
- Stored in `.g3/sessions/<session_id>/todo.g3.md`
- Call at start of multi-step tasks to check for existing plans

---

### todo_write

Create or update the TODO list.

**Parameters**:
- `content` (string, required): TODO list content in markdown checkbox format

**Example**:
```json
{"tool": "todo_write", "args": {"content": "- [ ] Implement feature\n  - [ ] Write tests\n  - [ ] Update docs\n- [x] Setup project"}}
```

**Notes**:
- Replaces entire file content
- Always call `todo_read` first to preserve existing content
- Use `- [ ]` for incomplete, `- [x]` for complete
- Supports nested tasks with indentation

---

## Code Intelligence Tools

### code_search

Syntax-aware code search using tree-sitter.

**Parameters**:
- `searches` (array, required): Array of search objects:
  - `name` (string): Label for this search
  - `query` (string): Tree-sitter query in S-expression format
  - `language` (string): Programming language
  - `paths` (array, optional): Paths to search
  - `context_lines` (integer, optional): Lines of context (0-20)
- `max_concurrency` (integer, optional): Parallel searches (default: 4)
- `max_matches_per_search` (integer, optional): Max matches (default: 500)

**Supported languages**: rust, python, javascript, typescript, go, java, c, cpp, kotlin

**Example**:
```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "functions",
    "query": "(function_item name: (identifier) @name)",
    "language": "rust",
    "context_lines": 2
  }]
}}
```

See [Code Search Guide](CODE_SEARCH.md) for detailed query patterns.

---

### code_coverage

Generate code coverage report using cargo llvm-cov.

**Parameters**: None

**Example**:
```json
{"tool": "code_coverage", "args": {}}
```

**Notes**:
- Runs all tests with coverage instrumentation
- Auto-installs llvm-tools-preview and cargo-llvm-cov if missing
- Returns coverage statistics summary

---

## WebDriver Tools

Enabled with `--webdriver` (Safari) or `--chrome-headless` (Chrome).

### webdriver_start

Start a browser session.

**Example**:
```json
{"tool": "webdriver_start", "args": {}}
```

### webdriver_navigate

Navigate to a URL.

**Parameters**:
- `url` (string, required): URL with protocol (e.g., `https://`)

### webdriver_get_url / webdriver_get_title

Get current URL or page title.

### webdriver_find_element / webdriver_find_elements

Find element(s) by CSS selector.

**Parameters**:
- `selector` (string, required): CSS selector

### webdriver_click

Click an element.

**Parameters**:
- `selector` (string, required): CSS selector

### webdriver_send_keys

Type text into an input.

**Parameters**:
- `selector` (string, required): CSS selector
- `text` (string, required): Text to type
- `clear_first` (boolean, optional): Clear before typing (default: true)

### webdriver_execute_script

Execute JavaScript.

**Parameters**:
- `script` (string, required): JavaScript code (use `return` to return values)

### webdriver_get_page_source

Get rendered HTML.

**Parameters**:
- `max_length` (integer, optional): Max chars to return (default: 10000, 0 for no limit)
- `save_to_file` (string, optional): Save to file instead of returning inline

### webdriver_screenshot

Take browser screenshot.

**Parameters**:
- `path` (string, required): Save path

### webdriver_back / webdriver_forward / webdriver_refresh

Navigation controls.

### webdriver_quit

Close browser and end session.

---



## Computer Control Tools

Enabled with `computer_control.enabled = true` in config.

### mouse_click

Click at coordinates.

**Parameters**:
- `x` (integer, required): X coordinate
- `y` (integer, required): Y coordinate
- `button` (string, optional): "left", "right", "middle"

### type_text

Type text at cursor.

**Parameters**:
- `text` (string, required): Text to type

### find_element

Find UI element by text, role, or attributes.

### list_windows

List all open windows with IDs and titles.

---

## Tool Execution Notes

### Duplicate Detection

G3 prevents accidental duplicate tool calls:
- Only immediately sequential identical calls are blocked
- Text between tool calls resets detection
- Tools can be reused throughout a session

### Error Handling

Tool errors are reported back to the agent, which can:
- Retry with different parameters
- Try an alternative approach
- Report the issue to the user

### Working Directory

Tools execute in:
1. Directory specified by `--codebase-fast-start` if provided
2. Current working directory otherwise

### File Paths

- Tilde expansion (`~`) is supported
- Relative paths are relative to working directory
- Screenshots default to `~/tmp` or `$TMPDIR`
