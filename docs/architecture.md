# G3 Architecture

**Last updated**: January 2025  
**Source of truth**: Crate structure in `crates/`, `Cargo.toml`, `DESIGN.md`

## Purpose

This document describes the internal architecture of G3, a modular AI coding agent built in Rust. It is intended for developers who want to understand, extend, or maintain the codebase.

## High-Level Overview

G3 follows a **tool-first philosophy**: instead of just providing advice, it actively uses tools to read files, write code, execute commands, and complete tasks autonomously.

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   g3-cli        │    │   g3-core       │    │ g3-providers    │
│                 │    │                 │    │                 │
│ • CLI parsing   │◄──►│ • Agent engine  │◄──►│ • Anthropic     │
│ • Interactive   │    │ • Context mgmt  │    │ • Databricks    │
│ • Retro TUI     │    │ • Tool system   │    │ • OpenAI        │
│ • Autonomous    │    │ • Streaming     │    │ • Embedded      │
│   mode          │    │ • Task exec     │    │   (llama.cpp)   │
│                 │    │ • TODO mgmt     │    │ • OAuth flow    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ g3-execution    │    │   g3-config     │    │  g3-planner     │
│                 │    │                 │    │                 │
│ • Code exec     │    │ • TOML config   │    │ • Requirements  │
│ • Shell cmds    │    │ • Env overrides │    │ • Git ops       │
│ • Streaming     │    │ • Provider      │    │ • Planning      │
│ • Error hdlg    │    │   settings      │    │   workflow      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │              ┌─────────────────┐              │
         │              │ g3-computer-    │              │
         └─────────────►│   control       │◄─────────────┘
                        │ • Mouse/kbd     │
                        │ • Screenshots   │
                        │ • OCR/Vision    │
                        │ • WebDriver     │
                        │ • macOS Ax API  │
                        └─────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐
│ g3-ensembles    │    │   g3-console    │
│                 │    │                 │
│ • Flock mode    │    │ • Web console   │
│ • Multi-agent   │    │ • Process mgmt  │
│ • Parallel dev  │    │ • Log viewing   │
└─────────────────┘    └─────────────────┘
```

## Workspace Structure

G3 is organized as a Rust workspace with 9 crates:

```
g3/
├── src/main.rs                   # Entry point (delegates to g3-cli)
├── crates/
│   ├── g3-cli/                   # Command-line interface and TUI
│   ├── g3-core/                  # Core agent engine and tools
│   ├── g3-providers/             # LLM provider abstractions
│   ├── g3-config/                # Configuration management
│   ├── g3-execution/             # Code execution engine
│   ├── g3-computer-control/      # Computer automation
│   ├── g3-planner/               # Planning mode workflow
│   ├── g3-ensembles/             # Multi-agent (flock) mode
│   └── g3-console/               # Web monitoring console
├── agents/                       # Agent persona definitions
├── logs/                         # Session logs (auto-created)
└── g3-plan/                      # Planning artifacts
```

## Crate Responsibilities

### g3-core (Central Hub)

**Location**: `crates/g3-core/`  
**Purpose**: Core agent engine, tool system, and orchestration logic

Key modules:
- `lib.rs` - Main `Agent` struct and orchestration (~3400 lines)
- `context_window.rs` - Token tracking and context management
- `streaming_parser.rs` - Real-time LLM response parsing
- `tool_definitions.rs` - JSON schema definitions for all tools
- `tool_dispatch.rs` - Routes tool calls to implementations
- `tools/` - Tool implementations (file ops, shell, vision, webdriver, etc.)
- `error_handling.rs` - Error classification and recovery
- `retry.rs` - Retry logic with exponential backoff
- `prompts.rs` - System prompt generation
- `code_search/` - Tree-sitter based code search

**Key types**:
- `Agent<W: UiWriter>` - Main agent struct, generic over UI output
- `ContextWindow` - Manages conversation history and token limits
- `StreamingToolParser` - Parses streaming LLM responses for tool calls
- `ToolCall` - Represents a tool invocation

### g3-providers (LLM Abstraction)

**Location**: `crates/g3-providers/`  
**Purpose**: Unified interface for multiple LLM backends

Key modules:
- `lib.rs` - `LLMProvider` trait and `ProviderRegistry`
- `anthropic.rs` - Anthropic Claude API (~51k chars)
- `databricks.rs` - Databricks Foundation Models (~58k chars)
- `openai.rs` - OpenAI and compatible APIs (~18k chars)
- `embedded.rs` - Local models via llama.cpp (~34k chars)
- `oauth.rs` - OAuth authentication flow

**Key traits**:
```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream>;
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn has_native_tool_calling(&self) -> bool;
    fn supports_cache_control(&self) -> bool;
    fn max_tokens(&self) -> u32;
    fn temperature(&self) -> f32;
}
```

### g3-cli (User Interface)

**Location**: `crates/g3-cli/`  
**Purpose**: Command-line interface, TUI, and execution modes

Key modules:
- `lib.rs` - Main CLI logic and execution modes (~112k chars)
- `retro_tui.rs` - Full-screen retro terminal UI (~63k chars)
- `filter_json.rs` - JSON tool call filtering for display
- `ui_writer_impl.rs` - Console output implementation
- `theme.rs` - Color themes for retro mode

**Execution modes**:
1. **Single-shot**: `g3 "task description"` - Execute one task and exit
2. **Interactive**: `g3` - REPL-style conversation (default)
3. **Autonomous**: `g3 --autonomous` - Coach-player feedback loop
4. **Accumulative**: Default interactive mode with autonomous runs
5. **Planning**: `g3 --planning` - Requirements-driven development
6. **Retro TUI**: `g3 --retro` - Full-screen terminal interface

### g3-config (Configuration)

**Location**: `crates/g3-config/`  
**Purpose**: TOML-based configuration management

Key structures:
- `Config` - Root configuration
- `ProvidersConfig` - Provider settings with named configs
- `AgentConfig` - Agent behavior settings
- `WebDriverConfig` - Browser automation settings
- `MacAxConfig` - macOS Accessibility API settings

**Configuration hierarchy** (highest priority last):
1. Default configuration
2. `~/.config/g3/config.toml`
3. `./g3.toml`
4. Environment variables (`G3_*`)
5. CLI arguments

### g3-execution (Code Execution)

**Location**: `crates/g3-execution/`  
**Purpose**: Safe execution of shell commands and scripts

Features:
- Streaming output capture
- Exit code tracking
- Async execution via Tokio
- Error handling and formatting

### g3-computer-control (Automation)

**Location**: `crates/g3-computer-control/`  
**Purpose**: Cross-platform computer control and automation

Key modules:
- `platform/` - Platform-specific implementations (macOS, Linux, Windows)
- `webdriver/` - Safari and Chrome WebDriver integration
- `ocr/` - Text extraction (Tesseract, Apple Vision)

**Platform support**:
- **macOS**: Core Graphics, Cocoa, screencapture, Vision framework
- **Linux**: X11/Xtest for input
- **Windows**: Win32 APIs

### g3-planner (Planning Mode)

**Location**: `crates/g3-planner/`  
**Purpose**: Requirements-driven development workflow

Key modules:
- `planner.rs` - Main planning state machine (~40k chars)
- `state.rs` - Planning state management
- `git.rs` - Git operations
- `code_explore.rs` - Codebase exploration
- `llm.rs` - LLM interactions for planning
- `history.rs` - Planning history tracking

**Workflow**:
1. Write requirements in `<codepath>/g3-plan/new_requirements.md`
2. LLM refines requirements
3. Requirements renamed to `current_requirements.md`
4. Coach/player loop implements
5. Files archived with timestamps
6. Git commit with LLM-generated message

### g3-ensembles (Multi-Agent)

**Location**: `crates/g3-ensembles/`  
**Purpose**: Parallel multi-agent development (Flock mode)

Key modules:
- `flock.rs` - Flock orchestration (~43k chars)
- `status.rs` - Agent status tracking

Flock mode enables parallel development by spawning multiple agent instances working on different parts of a project.

### g3-console (Web Console)

**Location**: `crates/g3-console/`  
**Purpose**: Web-based monitoring and control

Key modules:
- `main.rs` - Axum web server
- `api/` - REST API endpoints
- `process/` - Process detection and control
- `logs.rs` - Log parsing and streaming

## Data Flow

### Request Flow

```
User Input
    │
    ▼
┌─────────────┐
│  g3-cli     │  Parse input, determine mode
└─────────────┘
    │
    ▼
┌─────────────┐
│  g3-core    │  Add to context window
│  Agent      │  Build completion request
└─────────────┘
    │
    ▼
┌─────────────┐
│ g3-providers│  Send to LLM provider
│ Registry    │  Stream response
└─────────────┘
    │
    ▼
┌─────────────┐
│  g3-core    │  Parse streaming response
│  Parser     │  Detect tool calls
└─────────────┘
    │
    ▼
┌─────────────┐
│  g3-core    │  Execute tools
│  Tools      │  Return results
└─────────────┘
    │
    ▼
┌─────────────┐
│  g3-core    │  Add results to context
│  Agent      │  Continue or complete
└─────────────┘
```

### Context Window Management

The `ContextWindow` struct manages conversation history with intelligent token tracking:

1. **Token Tracking**: Monitors usage as percentage of provider's context limit
2. **Context Thinning**: At 50%, 60%, 70%, 80% thresholds, replaces large tool results with file references
3. **Auto-Compaction**: At 80% capacity, triggers conversation compaction
4. **Provider Adaptation**: Adjusts to different model context windows (4k to 200k+ tokens)

## Error Handling

G3 implements comprehensive error handling:

1. **Error Classification**: Distinguishes recoverable vs non-recoverable errors
2. **Automatic Retry**: Exponential backoff with jitter for:
   - Rate limits (HTTP 429)
   - Network errors
   - Server errors (HTTP 5xx)
   - Timeouts
3. **Error Logging**: Detailed logs saved to `logs/errors/`
4. **Graceful Degradation**: Continues when possible, fails gracefully when not

## Session Management

Sessions are tracked in `.g3/sessions/<session_id>/`:
- `session.json` - Full conversation history and metadata
- `todo.g3.md` - Session-scoped TODO list
- Context summaries and thinned content

Legacy logs are stored in `logs/g3_session_*.json`.

## Extension Points

### Adding a New Tool

1. Add tool definition in `g3-core/src/tool_definitions.rs`
2. Implement handler in `g3-core/src/tools/`
3. Add dispatch case in `g3-core/src/tool_dispatch.rs`
4. Update system prompt if needed in `g3-core/src/prompts.rs`

### Adding a New Provider

1. Implement `LLMProvider` trait in `g3-providers/src/`
2. Add configuration struct in `g3-config/src/lib.rs`
3. Register provider in `g3-core/src/lib.rs` (in `new_with_mode_and_readme`)
4. Update documentation

### Adding a New Execution Mode

1. Add CLI arguments in `g3-cli/src/lib.rs`
2. Implement mode logic in the CLI
3. May require new agent methods in `g3-core`

## Key Files for Understanding

Start reading here:

1. `src/main.rs` - Entry point (trivial, delegates to g3-cli)
2. `crates/g3-cli/src/lib.rs` - CLI and execution modes
3. `crates/g3-core/src/lib.rs` - Agent implementation
4. `crates/g3-providers/src/lib.rs` - Provider trait and registry
5. `crates/g3-core/src/tool_definitions.rs` - Available tools
6. `crates/g3-config/src/lib.rs` - Configuration structures
7. `DESIGN.md` - Original design document

## Dependencies

Key external dependencies:

- **tokio**: Async runtime
- **reqwest**: HTTP client for API calls
- **serde/serde_json**: Serialization
- **clap**: CLI argument parsing
- **tree-sitter**: Syntax-aware code search
- **llama_cpp**: Local model inference (with Metal acceleration)
- **fantoccini**: WebDriver client
- **axum**: Web framework (for g3-console)
