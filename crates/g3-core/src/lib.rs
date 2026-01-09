pub mod context_window;
pub mod background_process;
pub mod code_search;
pub mod error_handling;
pub mod feedback_extraction;
pub mod paths;
pub mod project;
pub mod provider_registration;
pub mod provider_config;
pub mod retry;
pub mod session;
pub mod session_continuation;
pub mod streaming_parser;
pub mod task_result;
pub mod tool_dispatch;
pub mod tool_definitions;
pub mod tools;
pub mod ui_writer;
pub mod streaming;
pub mod utils;
pub mod webdriver_session;

pub use task_result::TaskResult;
pub use retry::{RetryConfig, RetryResult, execute_with_retry, retry_operation};
pub use feedback_extraction::{ExtractedFeedback, FeedbackSource, FeedbackExtractionConfig, extract_coach_feedback};
pub use session_continuation::{SessionContinuation, load_continuation, save_continuation, clear_continuation, has_valid_continuation, get_session_dir, load_context_from_session_log, find_incomplete_agent_session};

// Re-export context window types
pub use context_window::{ContextWindow, ThinScope};

// Export agent prompt generation for CLI use
pub use prompts::get_agent_system_prompt;

#[cfg(test)]
mod task_result_comprehensive_tests;
use crate::ui_writer::UiWriter;

#[cfg(test)]
mod tilde_expansion_tests;

#[cfg(test)]
mod error_handling_test;
mod prompts;

use anyhow::Result;
use g3_config::Config;
use g3_providers::{CacheControl, CompletionRequest, Message, MessageRole, ProviderRegistry};
use prompts::{get_system_prompt_for_native, SYSTEM_PROMPT_FOR_NON_NATIVE_TOOL_USE};
#[allow(unused_imports)]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

// Re-export path utilities for backward compatibility
pub use paths::{
    G3_WORKSPACE_PATH_ENV, ensure_session_dir, get_context_summary_file, get_g3_dir, get_logs_dir,
    get_session_file, get_session_logs_dir, get_session_todo_path, get_thinned_dir, logs_dir,
};
use paths::get_todo_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value, // Should be a JSON object with tool-specific arguments
}


// Re-export WebDriverSession from its own module
pub use webdriver_session::WebDriverSession;

/// Options for fast-start discovery execution
#[derive(Debug, Clone)]
pub struct DiscoveryOptions<'a> {
    pub messages: &'a [Message],
    pub fast_start_path: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub enum StreamState {
    Generating,
    ToolDetected(ToolCall),
    Executing,
    Resuming,
}


// Re-export StreamingToolParser from its own module
pub use streaming_parser::StreamingToolParser;

pub struct Agent<W: UiWriter> {
    providers: ProviderRegistry,
    context_window: ContextWindow,
    thinning_events: Vec<usize>,      // chars saved per thinning event
    pending_90_compaction: bool,      // flag to trigger compaction at 90%
    auto_compact: bool,               // whether to auto-compact at 90% before tool calls
    compaction_events: Vec<usize>,    // chars saved per compaction event
    first_token_times: Vec<Duration>, // time to first token for each completion
    config: Config,
    session_id: Option<String>,
    tool_call_metrics: Vec<(String, Duration, bool)>, // (tool_name, duration, success)
    ui_writer: W,
    is_autonomous: bool,
    quiet: bool,
    computer_controller: Option<Box<dyn g3_computer_control::ComputerController>>,
    todo_content: std::sync::Arc<tokio::sync::RwLock<String>>,
    webdriver_session: std::sync::Arc<
        tokio::sync::RwLock<
            Option<std::sync::Arc<tokio::sync::Mutex<WebDriverSession>>>,
        >,
    >,
    webdriver_process: std::sync::Arc<tokio::sync::RwLock<Option<tokio::process::Child>>>,
    tool_call_count: usize,
    requirements_sha: Option<String>,
    /// Working directory for tool execution (set by --codebase-fast-start)
    working_dir: Option<String>,
    background_process_manager: std::sync::Arc<background_process::BackgroundProcessManager>,
    /// Pending images to attach to the next user message
    pending_images: Vec<g3_providers::ImageContent>,
    /// Whether this agent is running in agent mode (--agent flag)
    is_agent_mode: bool,
    /// Name of the agent if running in agent mode (e.g., "fowler", "pike")
    agent_name: Option<String>,
}

impl<W: UiWriter> Agent<W> {
    /// Minimum tokens for summary requests to avoid API errors when context is nearly full.
    /// This ensures max_tokens is never 0 even when context usage is 90%+.
    const SUMMARY_MIN_TOKENS: u32 = 1000;

    pub async fn new(config: Config, ui_writer: W) -> Result<Self> {
        Self::new_with_mode(config, ui_writer, false, false).await
    }

    pub async fn new_with_readme(
        config: Config,
        ui_writer: W,
        readme_content: Option<String>,
    ) -> Result<Self> {
        Self::new_with_mode_and_readme(config, ui_writer, false, readme_content, false, None).await
    }

    pub async fn new_autonomous_with_readme(
        config: Config,
        ui_writer: W,
        readme_content: Option<String>,
    ) -> Result<Self> {
        Self::new_with_mode_and_readme(config, ui_writer, true, readme_content, false, None).await
    }

    pub async fn new_autonomous(config: Config, ui_writer: W) -> Result<Self> {
        Self::new_with_mode(config, ui_writer, true, false).await
    }

    pub async fn new_with_quiet(config: Config, ui_writer: W, quiet: bool) -> Result<Self> {
        Self::new_with_mode(config, ui_writer, false, quiet).await
    }

    pub async fn new_with_readme_and_quiet(
        config: Config,
        ui_writer: W,
        readme_content: Option<String>,
        quiet: bool,
    ) -> Result<Self> {
        Self::new_with_mode_and_readme(config, ui_writer, false, readme_content, quiet, None).await
    }

    pub async fn new_autonomous_with_readme_and_quiet(
        config: Config,
        ui_writer: W,
        readme_content: Option<String>,
        quiet: bool,
    ) -> Result<Self> {
        Self::new_with_mode_and_readme(config, ui_writer, true, readme_content, quiet, None).await
    }

    /// Create a new agent with a custom system prompt (for agent mode)
    /// The custom_system_prompt replaces the default G3 system prompt entirely
    pub async fn new_with_custom_prompt(
        config: Config,
        ui_writer: W,
        custom_system_prompt: String,
        readme_content: Option<String>,
    ) -> Result<Self> {
        Self::new_with_mode_and_readme(config, ui_writer, false, readme_content, false, Some(custom_system_prompt)).await
    }

    async fn new_with_mode(
        config: Config,
        ui_writer: W,
        is_autonomous: bool,
        quiet: bool,
    ) -> Result<Self> {
        Self::new_with_mode_and_readme(config, ui_writer, is_autonomous, None, quiet, None).await
    }

    async fn new_with_mode_and_readme(
        config: Config,
        ui_writer: W,
        is_autonomous: bool,
        readme_content: Option<String>,
        quiet: bool,
        custom_system_prompt: Option<String>,
    ) -> Result<Self> {
        // Register providers using the extracted module
        let providers_to_register = provider_registration::determine_providers_to_register(&config, is_autonomous);
        let providers = provider_registration::register_providers(&config, &providers_to_register).await?;

        // Determine context window size based on active provider
        let mut context_warnings = Vec::new();
        let context_length =
            Self::get_configured_context_length(&config, &providers, &mut context_warnings)?;
        let mut context_window = ContextWindow::new(context_length);

        // Surface any context warnings to the user via UI
        for warning in context_warnings {
            ui_writer.print_context_status(&format!("⚠️ {}", warning));
        }

        // Add system prompt as the FIRST message (before README)
        // This ensures the agent always has proper tool usage instructions
        let provider = providers.get(None)?;
        let provider_has_native_tool_calling = provider.has_native_tool_calling();
        let _ = provider; // Drop provider reference to avoid borrowing issues

        let system_prompt = if let Some(custom_prompt) = custom_system_prompt {
            // Use custom system prompt (for agent mode)
            custom_prompt
        } else {
            // Use default system prompt based on provider capabilities
            if provider_has_native_tool_calling {
                // For native tool calling providers, use a more explicit system prompt
                get_system_prompt_for_native()
            } else {
                // For non-native providers (embedded models), use JSON format instructions
                SYSTEM_PROMPT_FOR_NON_NATIVE_TOOL_USE.to_string()
            }
        };

        let system_message = Message::new(MessageRole::System, system_prompt);
        context_window.add_message(system_message);

        // If README content is provided, add it as a second system message (after the main system prompt)
        if let Some(readme) = readme_content {
            let readme_message = Message::new(MessageRole::System, readme);
            context_window.add_message(readme_message);
        }

        // NOTE: TODO lists are now session-scoped and stored in .g3/sessions/<session_id>/todo.g3.md
        // We don't load any TODO at initialization since we don't have a session_id yet.
        // The agent will use todo_read to load the TODO once a session is established.

        // Initialize computer controller if enabled
        let computer_controller = if config.computer_control.enabled {
            match g3_computer_control::create_controller() {
                Ok(controller) => Some(controller),
                Err(e) => {
                    warn!("Failed to initialize computer control: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            providers,
            context_window,
            auto_compact: config.agent.auto_compact,
            pending_90_compaction: false,
            thinning_events: Vec::new(),
            compaction_events: Vec::new(),
            first_token_times: Vec::new(),
            config,
            session_id: None,
            tool_call_metrics: Vec::new(),
            ui_writer,
            // TODO content starts empty - session-scoped TODOs are loaded via todo_read
            todo_content: std::sync::Arc::new(tokio::sync::RwLock::new(String::new())),
            is_autonomous,
            quiet,
            computer_controller,
            webdriver_session: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            webdriver_process: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
            tool_call_count: 0,
            requirements_sha: None,
            working_dir: None,
            background_process_manager: std::sync::Arc::new(
                background_process::BackgroundProcessManager::new(
                    paths::get_logs_dir().join("background_processes")
                )),
            pending_images: Vec::new(),
            is_agent_mode: false,
            agent_name: None,
        })
    }

    /// Validate that the system prompt is the first message in the conversation history.
    /// This is a critical invariant that must be maintained for proper agent operation.
    ///
    /// # Panics
    /// Panics if:
    /// - The conversation history is empty
    /// - The first message is not a System message
    /// - The first message doesn't contain the system prompt markers
    fn validate_system_prompt_is_first(&self) {
        if self.context_window.conversation_history.is_empty() {
            panic!(
                "FATAL: Conversation history is empty. System prompt must be the first message."
            );
        }

        let first_message = &self.context_window.conversation_history[0];

        if !matches!(first_message.role, MessageRole::System) {
            panic!(
                "FATAL: First message is not a System message. Found: {:?}",
                first_message.role
            );
        }

        // Check for system prompt markers that are present in both standard and agent mode
        // Agent mode replaces the identity line but keeps all other instructions
        let has_tool_instructions = first_message.content.contains("IMPORTANT: You must call tools to achieve goals");
        if !has_tool_instructions {
            panic!("FATAL: First system message does not contain the system prompt. This likely means the README was added before the system prompt.");
        }
    }

    /// Convert cache config string to CacheControl enum
    fn parse_cache_control(cache_config: &str) -> Option<CacheControl> {
        match cache_config {
            "ephemeral" => Some(CacheControl::ephemeral()),
            "5minute" => Some(CacheControl::five_minute()),
            "1hour" => Some(CacheControl::one_hour()),
            _ => {
                warn!(
                    "Invalid cache_config value: '{}'. Valid values are: ephemeral, 5minute, 1hour",
                    cache_config
                );
                None
            }
        }
    }

    /// Count how many cache_control annotations exist in the conversation history
    fn count_cache_controls_in_history(&self) -> usize {
        self.context_window
            .conversation_history
            .iter()
            .filter(|msg| msg.cache_control.is_some())
            .count()
    }

    /// Resolve the max_tokens to use for a given provider, applying fallbacks.
    fn resolve_max_tokens(&self, provider_name: &str) -> u32 {
        provider_config::resolve_max_tokens(&self.config, provider_name)
    }

    /// Get the thinking budget tokens for Anthropic provider, if configured.
    fn get_thinking_budget_tokens(&self, provider_name: &str) -> Option<u32> {
        provider_config::get_thinking_budget_tokens(&self.config, provider_name)
    }

    /// Pre-flight check to validate max_tokens for thinking.budget_tokens constraint.
    fn preflight_validate_max_tokens(&self, provider_name: &str, proposed_max_tokens: u32) -> (u32, bool) {
        provider_config::preflight_validate_max_tokens(&self.config, provider_name, proposed_max_tokens)
    }

    /// Calculate max_tokens for a summary request.
    fn calculate_summary_max_tokens(&self, provider_name: &str) -> (u32, bool) {
        provider_config::calculate_summary_max_tokens(
            &self.config,
            provider_name,
            self.context_window.total_tokens,
            self.context_window.used_tokens,
        )
    }

    /// Apply the fallback sequence to free up context space for thinking budget.
    fn apply_max_tokens_fallback_sequence(&mut self, provider_name: &str, initial_max_tokens: u32, hard_coded_minimum: u32) -> u32 {
        self.apply_fallback_sequence_impl(provider_name, Some(initial_max_tokens), hard_coded_minimum)
    }

    /// Apply the fallback sequence for summary requests to free up context space.
    fn apply_summary_fallback_sequence(&mut self, provider_name: &str) -> u32 {
        self.apply_fallback_sequence_impl(provider_name, None, 5000)
    }

    /// Unified implementation of the fallback sequence for freeing context space.
    /// If `initial_max_tokens` is Some, uses preflight_validate_max_tokens for validation.
    /// If `initial_max_tokens` is None, uses calculate_summary_max_tokens for validation.
    fn apply_fallback_sequence_impl(
        &mut self,
        provider_name: &str,
        initial_max_tokens: Option<u32>,
        hard_coded_minimum: u32,
    ) -> u32 {
        // Initial validation
        let (mut max_tokens, needs_reduction) = match initial_max_tokens {
            Some(initial) => self.preflight_validate_max_tokens(provider_name, initial),
            None => self.calculate_summary_max_tokens(provider_name),
        };

        if !needs_reduction {
            return max_tokens;
        }

        self.ui_writer.print_context_status(
            "⚠️ Context window too full for thinking budget. Applying fallback sequence...\n",
        );

        // Step 1: Try thinnify (first third of context)
        self.ui_writer.print_context_status("🥒 Step 1: Trying thinnify...\n");
        let thin_msg = self.do_thin_context();
        self.ui_writer.print_context_thinning(&thin_msg);

        // Recalculate after thinnify
        let (new_max, still_needs_reduction) = self.recalculate_max_tokens(provider_name, initial_max_tokens.is_some());
        max_tokens = new_max;
        if !still_needs_reduction {
            self.ui_writer.print_context_status("✅ Thinnify resolved capacity issue. Continuing...\n");
            return max_tokens;
        }

        // Step 2: Try skinnify (entire context)
        self.ui_writer.print_context_status("🦴 Step 2: Trying skinnify...\n");
        let skinny_msg = self.do_thin_context_all();
        self.ui_writer.print_context_thinning(&skinny_msg);

        // Recalculate after skinnify
        let (final_max, final_needs_reduction) = self.recalculate_max_tokens(provider_name, initial_max_tokens.is_some());
        if !final_needs_reduction {
            self.ui_writer.print_context_status("✅ Skinnify resolved capacity issue. Continuing...\n");
            return final_max;
        }

        // Step 3: Nothing worked, use hard-coded minimum
        self.ui_writer.print_context_status(&format!(
            "⚠️ Step 3: Context reduction insufficient. Using hard-coded max_tokens={} as last resort...\n",
            hard_coded_minimum
        ));
        hard_coded_minimum
    }

    /// Helper to recalculate max_tokens after context reduction.
    fn recalculate_max_tokens(&self, provider_name: &str, use_preflight: bool) -> (u32, bool) {
        if use_preflight {
            let recalc_max = self.resolve_max_tokens(provider_name);
            self.preflight_validate_max_tokens(provider_name, recalc_max)
        } else {
            self.calculate_summary_max_tokens(provider_name)
        }
    }

    /// Resolve the temperature to use for a given provider, applying fallbacks.
    fn resolve_temperature(&self, provider_name: &str) -> f32 {
        provider_config::resolve_temperature(&self.config, provider_name)
    }

    /// Print provider diagnostics through the UiWriter for visibility
    pub fn print_provider_banner(&self, role_label: &str) {
        if let Ok((provider_name, model)) = self.get_provider_info() {
            let max_tokens = self.resolve_max_tokens(&provider_name);
            let context_len = self.context_window.total_tokens;

            let mut details = vec![
                format!("provider={}", provider_name),
                format!("model={}", model),
                format!("max_tokens={}", max_tokens),
                format!("context_window_length={}", context_len),
            ];

            if let Ok(provider) = self.providers.get(None) {
                details.push(format!(
                    "native_tools={}",
                    if provider.has_native_tool_calling() {
                        "yes"
                    } else {
                        "no"
                    }
                ));
                if provider.supports_cache_control() {
                    details.push("cache_control=yes".to_string());
                }
            }

            self.ui_writer
                .print_context_status(&format!("{}: {}", role_label, details.join(", ")));
        }
    }

    fn get_configured_context_length(
        config: &Config,
        providers: &ProviderRegistry,
        warnings: &mut Vec<String>,
    ) -> Result<u32> {
        // First, check if there's a global max_context_length override in agent config
        if let Some(max_context_length) = config.agent.max_context_length {
            debug!(
                "Using configured agent.max_context_length: {}",
                max_context_length
            );
            return Ok(max_context_length);
        }

        // Get the active provider to determine context length
        let provider = providers.get(None)?;
        let provider_name = provider.name();
        let model_name = provider.model();

        // Parse provider name to get type and config name
        let (provider_type, config_name) = provider_config::parse_provider_ref(provider_name);

        // Use provider-specific context length if available
        let context_length = match provider_type {
            "embedded" | "embedded." => {
                // For embedded models, use the configured context_length or model-specific defaults
                if let Some(embedded_config) = config.providers.embedded.get(config_name) {
                    embedded_config.context_length.unwrap_or_else(|| {
                        // Model-specific defaults for embedded models
                        match &embedded_config.model_type.to_lowercase()[..] {
                            "codellama" => 16384, // CodeLlama supports 16k context
                            "llama" => 4096,      // Base Llama models
                            "mistral" => 8192,    // Mistral models
                            "qwen" => 32768,      // Qwen2.5 supports 32k context
                            _ => 4096,            // Conservative default
                        }
                    })
                } else {
                    config.agent.fallback_default_max_tokens as u32
                }
            }
            "openai" => {
                // OpenAI models have varying context windows
                if let Some(max_tokens) = provider_config::get_max_tokens(config, provider_name) {
                    warnings.push(format!(
                        "Context length falling back to max_tokens ({}) for provider={}",
                        max_tokens, provider_name
                    ));
                    max_tokens
                } else {
                    400000
                }
            }
            "anthropic" => {
                // Claude models have large context windows
                if let Some(max_tokens) = provider_config::get_max_tokens(config, provider_name) {
                    warnings.push(format!(
                        "Context length falling back to max_tokens ({}) for provider={}",
                        max_tokens, provider_name
                    ));
                    max_tokens
                } else {
                    200000
                }
            }
            "databricks" => {
                // Databricks models have varying context windows depending on the model
                if let Some(max_tokens) = provider_config::get_max_tokens(config, provider_name) {
                    warnings.push(format!(
                        "Context length falling back to max_tokens ({}) for provider={}",
                        max_tokens, provider_name
                    ));
                    max_tokens
                } else if model_name.contains("claude") {
                    200000 // Claude models on Databricks have large context windows
                } else if model_name.contains("llama") || model_name.contains("dbrx") {
                    32768 // DBRX supports 32k context
                } else {
                    16384 // Conservative default for other Databricks models
                }
            }
            _ => config.agent.fallback_default_max_tokens as u32,
        };

        debug!(
            "Using context length: {} tokens for provider: {} (model: {})",
            context_length, provider_name, model_name
        );

        Ok(context_length)
    }

    pub fn get_provider_info(&self) -> Result<(String, String)> {
        let provider = self.providers.get(None)?;
        Ok((provider.name().to_string(), provider.model().to_string()))
    }

    /// Get the default LLM provider
    pub fn get_provider(&self) -> Result<&dyn g3_providers::LLMProvider> {
        self.providers.get(None)
    }

    /// Get the current session ID for this agent
    pub fn get_session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub async fn execute_task(
        &mut self,
        description: &str,
        language: Option<&str>,
        _auto_execute: bool,
    ) -> Result<TaskResult> {
        self.execute_task_with_options(description, language, false, false, false, None)
            .await
    }

    pub async fn execute_task_with_options(
        &mut self,
        description: &str,
        language: Option<&str>,
        _auto_execute: bool,
        show_prompt: bool,
        show_code: bool,
        discovery_options: Option<DiscoveryOptions<'_>>,
    ) -> Result<TaskResult> {
        self.execute_task_with_timing(
            description,
            language,
            _auto_execute,
            show_prompt,
            show_code,
            false,
            discovery_options,
        )
        .await
    }

    pub async fn execute_task_with_timing(
        &mut self,
        description: &str,
        language: Option<&str>,
        _auto_execute: bool,
        show_prompt: bool,
        show_code: bool,
        show_timing: bool,
        discovery_options: Option<DiscoveryOptions<'_>>,
    ) -> Result<TaskResult> {
        // Create a cancellation token that never cancels for backward compatibility
        let cancellation_token = CancellationToken::new();
        self.execute_task_with_timing_cancellable(
            description,
            language,
            _auto_execute,
            show_prompt,
            show_code,
            show_timing,
            cancellation_token,
            discovery_options,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute_task_with_timing_cancellable(
        &mut self,
        description: &str,
        _language: Option<&str>,
        _auto_execute: bool,
        show_prompt: bool,
        show_code: bool,
        show_timing: bool,
        cancellation_token: CancellationToken,
        discovery_options: Option<DiscoveryOptions<'_>>,
    ) -> Result<TaskResult> {
        // Execute the task directly without splitting
        self.execute_single_task(
            description,
            show_prompt,
            show_code,
            show_timing,
            cancellation_token,
            discovery_options,
        )
        .await
    }

    async fn execute_single_task(
        &mut self,
        description: &str,
        _show_prompt: bool,
        _show_code: bool,
        show_timing: bool,
        cancellation_token: CancellationToken,
        discovery_options: Option<DiscoveryOptions<'_>>,
    ) -> Result<TaskResult> {
        // Reset the JSON tool call filter state at the start of each new task
        // This prevents the filter from staying in suppression mode between user interactions
        self.ui_writer.reset_json_filter();

        // Validate that the system prompt is the first message (critical invariant)
        self.validate_system_prompt_is_first();

        // Generate session ID based on the initial prompt if this is a new session
        if self.session_id.is_none() {
            self.session_id = Some(self.generate_session_id(description));
        }

        // Add user message to context window
        let mut user_message = {
            // Check if we should use cache control (every 10 tool calls)
            // But only if we haven't already added 4 cache_control annotations
            let provider = self.providers.get(None)?;
            let provider_name = provider.name();
            let (provider_type, config_name) = provider_config::parse_provider_ref(provider_name);
            if let Some(cache_config) = match provider_type {
                "anthropic" => {
                    self.config
                        .providers
                        .anthropic
                        .get(config_name)
                        .and_then(|c| c.cache_config.as_ref())
                        .and_then(|config| Self::parse_cache_control(config))
                }
                _ => None,
            } {
                Message::with_cache_control_validated(
                    MessageRole::User,
                    format!("Task: {}", description),
                    cache_config,
                    provider,
                )
            } else {
                Message::new(MessageRole::User, format!("Task: {}", description))
            }
        };
        
        // Attach any pending images to this user message
        if !self.pending_images.is_empty() {
            user_message.images = std::mem::take(&mut self.pending_images);
        }
        
        self.context_window.add_message(user_message);

        // Execute fast-discovery tool calls if provided (immediately after user message)
        if let Some(ref options) = discovery_options {
            self.ui_writer
                .println("▶️  Playing back discovery commands...");
            // Store the working directory for subsequent tool calls in the streaming loop
            if let Some(path) = options.fast_start_path {
                self.working_dir = Some(path.to_string());
            }
            let provider = self.providers.get(None)?;
            let supports_cache = provider.supports_cache_control();
            let message_count = options.messages.len();

            for (idx, discovery_msg) in options.messages.iter().enumerate() {
                if let Ok(tool_call) = serde_json::from_str::<ToolCall>(&discovery_msg.content) {
                    self.add_message_to_context(discovery_msg.clone());
                    let result = self
                        .execute_tool_call_in_dir(&tool_call, options.fast_start_path)
                        .await
                        .unwrap_or_else(|e| format!("Error: {}", e));

                    // Add cache_control to the last user message if provider supports it (anthropic)
                    let is_last = idx == message_count - 1;
                    let result_message = if supports_cache
                        && is_last
                        && self.count_cache_controls_in_history() < 4
                    {
                        Message::with_cache_control(
                            MessageRole::User,
                            format!("Tool result: {}", result),
                            CacheControl::ephemeral(),
                        )
                    } else {
                        Message::new(MessageRole::User, format!("Tool result: {}", result))
                    };
                    self.add_message_to_context(result_message);
                }
            }
        }

        // Use the complete conversation history for the request
        let messages = self.context_window.conversation_history.clone();

        // Check if provider supports native tool calling and add tools if so
        let provider = self.providers.get(None)?;
        let provider_name = provider.name().to_string();
        let _has_native_tool_calling = provider.has_native_tool_calling();
        let _supports_cache_control = provider.supports_cache_control();
        let tools = if provider.has_native_tool_calling() {
            Some(tool_definitions::create_tool_definitions(
                tool_definitions::ToolConfig::new(
                    self.config.webdriver.enabled,
                    self.config.computer_control.enabled,
                )))
        } else {
            None
        };
        let _ = provider; // Drop the provider reference to avoid borrowing issues

        // Get max_tokens from provider configuration with preflight validation
        // This ensures max_tokens > thinking.budget_tokens for Anthropic with extended thinking
        let initial_max_tokens = self.resolve_max_tokens(&provider_name);
        let max_tokens = Some(self.apply_max_tokens_fallback_sequence(
            &provider_name,
            initial_max_tokens,
            16000, // Hard-coded minimum for main API calls (higher than summary's 5000)
        ));

        let request = CompletionRequest {
            messages,
            max_tokens,
            temperature: Some(self.resolve_temperature(&provider_name)),
            stream: true, // Enable streaming
            tools,
            disable_thinking: false,
        };

        // Time the LLM call with cancellation support and streaming
        let llm_start = Instant::now();
        let result = tokio::select! {
            result = self.stream_completion(request, show_timing) => result,
            _ = cancellation_token.cancelled() => {
                // Save context window on cancellation
                self.save_context_window("cancelled");
                Err(anyhow::anyhow!("Operation cancelled by user"))
            }
        };

        let task_result = match result {
            Ok(result) => result,
            Err(e) => {
                // Save context window on error
                self.save_context_window("error");
                return Err(e);
            }
        };

        let response_content = task_result.response.clone();
        let _llm_duration = llm_start.elapsed();

        // Create a mock usage for now (we'll need to track this during streaming)
        let mock_usage = g3_providers::Usage {
            prompt_tokens: 100,                                   // Estimate
            completion_tokens: response_content.len() as u32 / 4, // Rough estimate
            total_tokens: 100 + (response_content.len() as u32 / 4),
        };

        // Update context window with estimated token usage
        self.context_window.update_usage(&mock_usage);

        // Add assistant response to context window only if not empty
        // This prevents the "Skipping empty message" warning when only tools were executed
        if !response_content.trim().is_empty() {
            let assistant_message = Message::new(MessageRole::Assistant, response_content.clone());
            self.context_window.add_message(assistant_message);
        } else {
            debug!("Assistant response was empty (likely only tool execution), skipping message addition");
        }

        // Save context window at the end of successful interaction
        self.save_context_window("completed");

        // Check if we need to do 90% auto-compaction
        if self.pending_90_compaction {
            self.ui_writer
                .print_context_status("\n⚡ Context window reached 90% - auto-compacting...\n");
            if let Err(e) = self.force_compact().await {
                warn!("Failed to auto-compact at 90%: {}", e);
            } else {
                self.ui_writer.println("");
            }
            self.pending_90_compaction = false;
        }

        // Return the task result which already includes timing if needed
        Ok(task_result)
    }

    /// Generate a session ID based on the initial prompt
    fn generate_session_id(&self, description: &str) -> String {
        session::generate_session_id(description, self.agent_name.as_deref())
    }

    /// Save the entire context window to a per-session file
    fn save_context_window(&self, status: &str) {
        if self.quiet {
            return;
        }
        session::save_context_window(self.session_id.as_deref(), &self.context_window, status);
    }

    /// Write context window summary to file
    /// Format: date&time, token_count, message_id, role, first_100_chars
    fn write_context_window_summary(&self) {
        if self.quiet {
            return;
        }
        if let Some(ref session_id) = self.session_id {
            session::write_context_window_summary(session_id, &self.context_window);
        }
    }

    pub fn get_context_window(&self) -> &ContextWindow {
        &self.context_window
    }

    /// Add a message directly to the context window.
    /// Used for injecting discovery messages before the first LLM turn.
    pub fn add_message_to_context(&mut self, message: Message) {
        self.context_window.add_message(message);
    }

    /// Execute a tool call and return the result.
    /// This is a public wrapper around execute_tool for use by external callers
    /// like the planner's fast-discovery feature.
    pub async fn execute_tool_call(&mut self, tool_call: &ToolCall) -> Result<String> {
        self.execute_tool(tool_call).await
    }

    /// Execute a tool call with an optional working directory (for discovery commands)
    pub async fn execute_tool_call_in_dir(
        &mut self,
        tool_call: &ToolCall,
        working_dir: Option<&str>,
    ) -> Result<String> {
        self.execute_tool_in_dir(tool_call, working_dir).await
    }

    /// Log an error message to the session JSON file as the last message
    /// This is used in autonomous mode to record context length exceeded errors
    pub fn log_error_to_session(
        &self,
        error: &anyhow::Error,
        role: &str,
        forensic_context: Option<String>,
    ) {
        if self.quiet {
            return;
        }
        match &self.session_id {
            Some(id) => session::log_error_to_session(id, error, role, forensic_context),
            None => {
                error!("Cannot log error to session: no session ID");
            }
        }
    }

    /// Manually trigger context compaction regardless of context window size
    /// Returns Ok(true) if compaction was successful, Ok(false) if it failed
    pub async fn force_compact(&mut self) -> Result<bool> {
        debug!("Manual compaction triggered");

        self.ui_writer.print_context_status(&format!(
            "\n🗜️ Manual compaction requested (current usage: {}%)...",
            self.context_window.percentage_used() as u32
        ));

        let provider = self.providers.get(None)?;
        let provider_name = provider.name().to_string();
        let _ = provider; // Release borrow early

        // Apply fallback sequence: thinnify -> skinnify -> hard-coded 5000
        let mut summary_max_tokens = self.apply_summary_fallback_sequence(&provider_name);

        // Apply provider-specific caps
        // For Anthropic with thinking enabled, we need max_tokens > thinking.budget_tokens
        // So we set a higher cap when thinking is configured
        let anthropic_cap = match self.get_thinking_budget_tokens(&provider_name) {
            Some(budget) => (budget + 2000).max(10_000), // At least budget + 2000 for response
            None => 10_000,
        };
        summary_max_tokens = match provider_name.as_str() {
            "anthropic" => summary_max_tokens.min(anthropic_cap),
            "databricks" => summary_max_tokens.min(10_000),
            "embedded" => summary_max_tokens.min(3000),
            _ => summary_max_tokens.min(5000),
        };

        // Ensure minimum floor as defense-in-depth (primary protection is in calculate_summary_max_tokens)
        summary_max_tokens = summary_max_tokens.max(Self::SUMMARY_MIN_TOKENS);

        debug!(
            "Requesting summary with max_tokens: {} (current usage: {} tokens)",
            summary_max_tokens, self.context_window.used_tokens
        );

        // Create summary request with FULL history
        let summary_prompt = self.context_window.create_summary_prompt();

        // Get the full conversation history
        let conversation_text = self
            .context_window
            .conversation_history
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let summary_messages = vec![
            Message::new(
                MessageRole::System,
                "You are a helpful assistant that creates concise summaries.".to_string(),
            ),
            Message::new(
                MessageRole::User,
                format!(
                    "Based on this conversation history, {}\n\nConversation:\n{}",
                    summary_prompt, conversation_text
                ),
            ),
        ];

        let provider = self.providers.get(None)?;

        // Determine if we need to disable thinking mode for this request
        // Anthropic requires: max_tokens > thinking.budget_tokens + 1024
        let disable_thinking = self.get_thinking_budget_tokens(provider.name()).map_or(false, |budget| {
            let minimum_for_thinking = budget + 1024;
            let should_disable = summary_max_tokens <= minimum_for_thinking;
            if should_disable {
                tracing::warn!("Disabling thinking mode for summary: max_tokens ({}) <= minimum_for_thinking ({})", summary_max_tokens, minimum_for_thinking);
            }
            should_disable
        });

        tracing::debug!("Creating summary request: max_tokens={}, disable_thinking={}", summary_max_tokens, disable_thinking);

        let summary_request = CompletionRequest {
            messages: summary_messages,
            max_tokens: Some(summary_max_tokens),
            temperature: Some(self.resolve_temperature(provider.name())),
            stream: false,
            tools: None,
            disable_thinking,
        };

        // Get the summary
        match provider.complete(summary_request).await {
            Ok(summary_response) => {
                self.ui_writer
                    .print_context_status("✅ Context compacted successfully.\n");

                // Get the latest user message to preserve it
                let latest_user_msg = self
                    .context_window
                    .conversation_history
                    .iter()
                    .rev()
                    .find(|m| matches!(m.role, MessageRole::User))
                    .map(|m| m.content.clone());

                // Reset context with summary
                let chars_saved = self
                    .context_window
                    .reset_with_summary(summary_response.content, latest_user_msg);
                self.compaction_events.push(chars_saved);

                Ok(true)
            }
            Err(e) => {
                error!("Failed to create summary: {}", e);
                self.ui_writer.print_context_status(
                    "⚠️ Unable to create summary. Please try again or start a new session.\n",
                );
                Ok(false)
            }
        }
    }

    /// Manually trigger context thinning regardless of thresholds
    pub fn force_thin(&mut self) -> String {
        debug!("Manual context thinning triggered");
        self.do_thin_context()
    }

    /// Manually trigger context thinning for the ENTIRE context window
    /// Unlike force_thin which only processes the first third, this processes all messages
    pub fn force_thin_all(&mut self) -> String {
        debug!("Manual full context skinnifying triggered");
        self.do_thin_context_all()
    }

    /// Internal helper: thin context and track the event
    fn do_thin_context(&mut self) -> String {
        let (message, chars_saved) = self.context_window.thin_context(self.session_id.as_deref());
        self.thinning_events.push(chars_saved);
        message
    }

    /// Internal helper: thin all context and track the event
    fn do_thin_context_all(&mut self) -> String {
        let (message, chars_saved) = self.context_window.thin_context_all(self.session_id.as_deref());
        self.thinning_events.push(chars_saved);
        message
    }

    /// Check if a tool call is a duplicate of the last tool call in the previous assistant message.
    /// Returns Some("DUP IN MSG") if it's a duplicate, None otherwise.
    fn check_duplicate_in_previous_message(&self, tool_call: &ToolCall) -> Option<String> {
        // Helper to check if two tool calls are duplicates
        let are_duplicates = |tc1: &ToolCall, tc2: &ToolCall| -> bool {
            tc1.tool == tc2.tool && tc1.args == tc2.args
        };

        // Find the most recent assistant message
        for msg in self.context_window.conversation_history.iter().rev() {
            if !matches!(msg.role, MessageRole::Assistant) {
                continue;
            }

            let content = &msg.content;

            // Look for the last occurrence of a tool call pattern
            let last_tool_start = content.rfind(r#"{"tool""#)
                .or_else(|| content.rfind(r#"{ "tool""#))?;

            // Find the end of this JSON object
            let end_offset = StreamingToolParser::find_complete_json_object_end(&content[last_tool_start..])?;
            let end_idx = last_tool_start + end_offset + 1;
            let tool_json = &content[last_tool_start..end_idx];

            // Check if there's any non-whitespace text after this tool call
            let text_after = content[end_idx..].trim();
            if !text_after.is_empty() {
                // There's text after the tool call, so it's not a trailing duplicate
                return None;
            }

            // Parse and compare the tool call
            if let Ok(prev_tool) = serde_json::from_str::<ToolCall>(tool_json) {
                if are_duplicates(&prev_tool, tool_call) {
                    return Some("DUP IN MSG".to_string());
                }
            }

            // Only check the most recent assistant message
            break;
        }

        None
    }

    /// Reload README.md and AGENTS.md and replace the first system message
    /// Returns Ok(true) if README was found and reloaded, Ok(false) if no README was present initially
    pub fn reload_readme(&mut self) -> Result<bool> {
        debug!("Manual README reload triggered");

        // Check if the second message in conversation history is a system message with README content
        // (The first message should always be the system prompt)
        let has_readme = self
            .context_window
            .conversation_history
            .get(1) // Check the SECOND message (index 1)
            .map(|m| {
                matches!(m.role, MessageRole::System)
                    && (m.content.contains("Project README")
                        || m.content.contains("Agent Configuration"))
            })
            .unwrap_or(false);

        // Validate that the system prompt is still first
        self.validate_system_prompt_is_first();

        if !has_readme {
            return Ok(false);
        }

        // Try to load README.md and AGENTS.md
        let mut combined_content = String::new();
        let mut found_any = false;

        if let Ok(agents_content) = std::fs::read_to_string("AGENTS.md") {
            combined_content.push_str("# Agent Configuration\n\n");
            combined_content.push_str(&agents_content);
            combined_content.push_str("\n\n");
            found_any = true;
        }

        if let Ok(readme_content) = std::fs::read_to_string("README.md") {
            combined_content.push_str("# Project README\n\n");
            combined_content.push_str(&readme_content);
            found_any = true;
        }

        if found_any {
            // Replace the second message (README) with the new content
            if let Some(first_msg) = self.context_window.conversation_history.get_mut(1) {
                first_msg.content = combined_content;
                debug!("README content reloaded successfully");
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Get detailed context statistics
    pub fn get_stats(&self) -> String {
        let mut stats = String::new();
        use std::time::Duration;

        stats.push_str("\n📊 Context Window Statistics\n");
        stats.push_str(&"=".repeat(60));
        stats.push_str("\n\n");

        // Context window usage
        stats.push_str("🗂️  Context Window:\n");
        stats.push_str(&format!(
            "   • Used Tokens:       {:>10} / {}\n",
            self.context_window.used_tokens, self.context_window.total_tokens
        ));
        stats.push_str(&format!(
            "   • Usage Percentage:  {:>10.1}%\n",
            self.context_window.percentage_used()
        ));
        stats.push_str(&format!(
            "   • Remaining Tokens:  {:>10}\n",
            self.context_window.remaining_tokens()
        ));
        stats.push_str(&format!(
            "   • Cumulative Tokens: {:>10}\n",
            self.context_window.cumulative_tokens
        ));
        stats.push_str(&format!(
            "   • Last Thinning:     {:>10}%\n",
            self.context_window.last_thinning_percentage
        ));
        stats.push('\n');

        // Context optimization metrics
        stats.push_str("🗜️  Context Optimization:\n");
        stats.push_str(&format!(
            "   • Thinning Events:   {:>10}\n",
            self.thinning_events.len()
        ));
        if !self.thinning_events.is_empty() {
            let total_thinned: usize = self.thinning_events.iter().sum();
            let avg_thinned = total_thinned / self.thinning_events.len();
            stats.push_str(&format!("   • Total Chars Saved: {:>10}\n", total_thinned));
            stats.push_str(&format!("   • Avg Chars/Event:   {:>10}\n", avg_thinned));
        }

        stats.push_str(&format!(
            "   • Compactions:       {:>10}\n",
            self.compaction_events.len()
        ));
        if !self.compaction_events.is_empty() {
            let total_compacted: usize = self.compaction_events.iter().sum();
            let avg_compacted = total_compacted / self.compaction_events.len();
            stats.push_str(&format!(
                "   • Total Chars Saved: {:>10}\n",
                total_compacted
            ));
            stats.push_str(&format!("   • Avg Chars/Event:   {:>10}\n", avg_compacted));
        }
        stats.push('\n');

        // Performance metrics
        stats.push_str("⚡ Performance:\n");
        if !self.first_token_times.is_empty() {
            let avg_ttft = self.first_token_times.iter().sum::<Duration>()
                / self.first_token_times.len() as u32;
            let mut sorted_times = self.first_token_times.clone();
            sorted_times.sort();
            let median_ttft = sorted_times[sorted_times.len() / 2];
            stats.push_str(&format!(
                "   • Avg Time to First Token:    {:>6.3}s\n",
                avg_ttft.as_secs_f64()
            ));
            stats.push_str(&format!(
                "   • Median Time to First Token: {:>6.3}s\n",
                median_ttft.as_secs_f64()
            ));
        }
        stats.push('\n');

        // Conversation history
        stats.push_str("💬 Conversation History:\n");
        stats.push_str(&format!(
            "   • Total Messages:    {:>10}\n",
            self.context_window.conversation_history.len()
        ));

        // Count messages by role
        let mut system_count = 0;
        let mut user_count = 0;
        let mut assistant_count = 0;

        for msg in &self.context_window.conversation_history {
            match msg.role {
                MessageRole::System => system_count += 1,
                MessageRole::User => user_count += 1,
                MessageRole::Assistant => assistant_count += 1,
            }
        }

        stats.push_str(&format!("   • System Messages:   {:>10}\n", system_count));
        stats.push_str(&format!("   • User Messages:     {:>10}\n", user_count));
        stats.push_str(&format!(
            "   • Assistant Messages:{:>10}\n",
            assistant_count
        ));
        stats.push('\n');

        // Tool call metrics
        stats.push_str("🔧 Tool Call Metrics:\n");
        stats.push_str(&format!(
            "   • Total Tool Calls:  {:>10}\n",
            self.tool_call_metrics.len()
        ));

        let successful_calls = self
            .tool_call_metrics
            .iter()
            .filter(|(_, _, success)| *success)
            .count();
        let failed_calls = self.tool_call_metrics.len() - successful_calls;

        stats.push_str(&format!(
            "   • Successful:        {:>10}\n",
            successful_calls
        ));
        stats.push_str(&format!("   • Failed:            {:>10}\n", failed_calls));

        if !self.tool_call_metrics.is_empty() {
            let total_duration: Duration = self
                .tool_call_metrics
                .iter()
                .map(|(_, duration, _)| *duration)
                .sum();
            let avg_duration = total_duration / self.tool_call_metrics.len() as u32;

            stats.push_str(&format!(
                "   • Total Duration:    {:>10.2}s\n",
                total_duration.as_secs_f64()
            ));
            stats.push_str(&format!(
                "   • Average Duration:  {:>10.2}s\n",
                avg_duration.as_secs_f64()
            ));
        }
        stats.push('\n');

        // Provider info
        stats.push_str("🔌 Provider:\n");
        if let Ok((provider, model)) = self.get_provider_info() {
            stats.push_str(&format!("   • Provider:          {}\n", provider));
            stats.push_str(&format!("   • Model:             {}\n", model));
        }

        stats.push_str(&"=".repeat(60));
        stats.push('\n');

        stats
    }

    pub fn get_tool_call_metrics(&self) -> &Vec<(String, Duration, bool)> {
        &self.tool_call_metrics
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn set_requirements_sha(&mut self, sha: String) {
        self.requirements_sha = Some(sha);
    }

    /// Save a session continuation artifact
    /// Called when final_output is invoked to enable session resumption
    pub fn save_session_continuation(&self, final_output_summary: Option<String>) {
        use crate::session_continuation::{save_continuation, SessionContinuation};
        
        let session_id = match &self.session_id {
            Some(id) => id.clone(),
            None => {
                debug!("No session ID, skipping continuation save");
                return;
            }
        };
        
        // Get the session log path (now in .g3/sessions/<session_id>/session.json)
        let session_log_path = get_session_file(&session_id);
        
        // Get current TODO content - try session-specific path first, then workspace path
        let session_todo_path = crate::paths::get_session_todo_path(&session_id);
        let todo_snapshot = if session_todo_path.exists() {
            std::fs::read_to_string(&session_todo_path).ok()
        } else {
            // Fall back to workspace TODO path for backwards compatibility
            std::fs::read_to_string(get_todo_path()).ok()
        };
        
        // Get working directory
        let working_directory = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        
        let continuation = SessionContinuation::new(
            self.is_agent_mode,
            self.agent_name.clone(),
            session_id,
            final_output_summary,
            session_log_path.to_string_lossy().to_string(),
            self.context_window.percentage_used(),
            todo_snapshot,
            working_directory,
        );
        
        if let Err(e) = save_continuation(&continuation) {
            error!("Failed to save session continuation: {}", e);
        } else {
            debug!("Saved session continuation artifact");
        }
    }
    
    /// Set agent mode information for session tracking
    /// Called when running with --agent flag to enable agent-specific session resume
    pub fn set_agent_mode(&mut self, agent_name: &str) {
        self.is_agent_mode = true;
        self.agent_name = Some(agent_name.to_string());
        debug!("Agent mode enabled for agent: {}", agent_name);
    }

    /// Initialize session ID manually (primarily for testing).
    /// This allows tests to verify session ID generation without calling execute_task,
    /// which would require an LLM provider.
    pub fn init_session_id_for_test(&mut self, description: &str) {
        if self.session_id.is_none() {
            self.session_id = Some(self.generate_session_id(description));
        }
    }

    /// Clear session state and continuation artifacts (for /clear command)
    pub fn clear_session(&mut self) {
        use crate::session_continuation::clear_continuation;
        
        // Clear the context window (keep system prompt)
        self.context_window.clear_conversation();
        
        // Clear continuation artifacts
        if let Err(e) = clear_continuation() {
            error!("Failed to clear continuation artifacts: {}", e);
        }
        
        debug!("Session cleared");
    }

    /// Restore session from a continuation artifact
    /// Returns true if full context was restored, false if only summary was used
    pub fn restore_from_continuation(
        &mut self,
        continuation: &crate::session_continuation::SessionContinuation,
    ) -> Result<bool> {
        use std::path::PathBuf;
        
        let session_log_path = PathBuf::from(&continuation.session_log_path);
        
        // If context < 80%, try to restore full context
        if continuation.can_restore_full_context() && session_log_path.exists() {
            // Load the session log
            let json = std::fs::read_to_string(&session_log_path)?;
            let session_data: serde_json::Value = serde_json::from_str(&json)?;
            
            // Extract conversation history
            if let Some(context_window) = session_data.get("context_window") {
                if let Some(history) = context_window.get("conversation_history") {
                    if let Some(messages) = history.as_array() {
                        // Clear current conversation (keep system messages)
                        self.context_window.clear_conversation();
                        
                        // Restore messages from session log (skip system messages as they're preserved)
                        for msg in messages {
                            let role_str = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                            
                            let role = match role_str {
                                "system" => continue, // Skip system messages, already preserved
                                "assistant" => MessageRole::Assistant,
                                _ => MessageRole::User,
                            };
                            
                            self.context_window.add_message(Message {
                                role,
                                id: String::new(),
                                images: Vec::new(),
                                content: content.to_string(),
                                cache_control: None,
                            });
                        }
                        
                        debug!("Restored full context from session log");
                        return Ok(true);
                    }
                }
            }
        }
        
        // Fall back to using final_output summary + TODO
        let mut context_msg = String::new();
        if let Some(ref summary) = continuation.final_output_summary {
            context_msg.push_str(&format!("Previous session summary:\n{}\n\n", summary));
        }
        if let Some(ref todo) = continuation.todo_snapshot {
            context_msg.push_str(&format!("Current TODO state:\n{}\n", todo));
        }
        
        if !context_msg.is_empty() {
            self.context_window.add_message(Message {
                role: MessageRole::User,
                id: String::new(),
                images: Vec::new(),
                content: format!("[Session Resumed]\n\n{}", context_msg),
                cache_control: None,
            });
        }
        
        debug!("Restored session from summary");
        Ok(false)
    }

    async fn stream_completion(
        &mut self,
        request: CompletionRequest,
        show_timing: bool,
    ) -> Result<TaskResult> {
        self.stream_completion_with_tools(request, show_timing)
            .await
    }

    /// Create tool definitions for native tool calling providers

    /// Helper method to stream with retry logic
    async fn stream_with_retry(
        &self,
        request: &CompletionRequest,
        error_context: &error_handling::ErrorContext,
    ) -> Result<g3_providers::CompletionStream> {
        use crate::error_handling::{calculate_retry_delay, classify_error, ErrorType};

        let mut attempt = 0;
        let max_attempts = if self.is_autonomous {
            self.config.agent.autonomous_max_retry_attempts
        } else {
            self.config.agent.max_retry_attempts
        };

        loop {
            attempt += 1;
            let provider = self.providers.get(None)?;

            match provider.stream(request.clone()).await {
                Ok(stream) => {
                    if attempt > 1 {
                        debug!("Stream started successfully after {} attempts", attempt);
                    }
                    debug!("Stream started successfully");
                    debug!(
                        "Request had {} messages, tools={}, max_tokens={:?}",
                        request.messages.len(),
                        request.tools.is_some(),
                        request.max_tokens
                    );
                    return Ok(stream);
                }
                Err(e) if attempt < max_attempts => {
                    if matches!(classify_error(&e), ErrorType::Recoverable(_)) {
                        let delay = calculate_retry_delay(attempt, self.is_autonomous);
                        warn!(
                            "Recoverable error on attempt {}/{}: {}. Retrying in {:?}...",
                            attempt, max_attempts, e, delay
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        error_context.clone().log_error(&e);
                        return Err(e);
                    }
                }
                Err(e) => {
                    error_context.clone().log_error(&e);
                    return Err(e);
                }
            }
        }
    }

    async fn stream_completion_with_tools(
        &mut self,
        mut request: CompletionRequest,
        show_timing: bool,
    ) -> Result<TaskResult> {
        use crate::error_handling::ErrorContext;
        use tokio_stream::StreamExt;

        debug!("Starting stream_completion_with_tools");

        let mut full_response = String::new();
        let mut first_token_time: Option<Duration> = None;
        let stream_start = Instant::now();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 400; // Prevent infinite loops
        let mut response_started = false;
        let mut any_tool_executed = false; // Track if ANY tool was executed across all iterations
        let mut auto_summary_attempts = 0; // Track auto-summary prompt attempts
        const MAX_AUTO_SUMMARY_ATTEMPTS: usize = 5; // Limit auto-summary retries (increased from 2 for better recovery)
        let final_output_called = false; // Track if final_output was called
        // Note: Session-level duplicate tracking was removed - we only prevent sequential duplicates (DUP IN CHUNK, DUP IN MSG)
        let mut turn_accumulated_usage: Option<g3_providers::Usage> = None; // Track token usage for timing footer

        // Check if we need to compact before starting
        if self.context_window.should_compact() {
            // First try thinning if we are at capacity, don't call the LLM for compaction (might fail)
            if self.context_window.percentage_used() > 90.0 && self.context_window.should_thin() {
                self.ui_writer.print_context_status(&format!(
                    "\n🥒 Context window at {}%. Trying thinning first...",
                    self.context_window.percentage_used() as u32
                ));

                let thin_summary = self.do_thin_context();
                self.ui_writer.print_context_thinning(&thin_summary);

                // Check if thinning was sufficient
                if !self.context_window.should_compact() {
                    self.ui_writer.print_context_status(
                        "✅ Thinning resolved capacity issue. Continuing...\n",
                    );
                    // Continue with the original request without compaction
                } else {
                    self.ui_writer.print_context_status(
                        "⚠️ Thinning insufficient. Proceeding with compaction...\n",
                    );
                }
            }

            // Only proceed with compaction if still needed after thinning
            if self.context_window.should_compact() {
                // Notify user about compaction
                self.ui_writer.print_context_status(&format!(
                    "\n🗜️ Context window reaching capacity ({}%). Compacting...",
                    self.context_window.percentage_used() as u32
                ));

                let provider = self.providers.get(None)?;
                let provider_name = provider.name().to_string();
                let _ = provider; // Release borrow early

                // Apply fallback sequence: thinnify -> skinnify -> hard-coded 5000
                let mut summary_max_tokens = self.apply_summary_fallback_sequence(&provider_name);

                // Apply provider-specific caps
                // For Anthropic with thinking enabled, we need max_tokens > thinking.budget_tokens
                // So we set a higher cap when thinking is configured
                let anthropic_cap = match self.get_thinking_budget_tokens(&provider_name) {
                    Some(budget) => (budget + 2000).max(10_000), // At least budget + 2000 for response
                    None => 10_000,
                };
                summary_max_tokens = match provider_name.as_str() {
                    "anthropic" => summary_max_tokens.min(anthropic_cap),
                    "databricks" => summary_max_tokens.min(10_000),
                    "embedded" => summary_max_tokens.min(3000),
                    _ => summary_max_tokens.min(5000),
                };

                // Ensure minimum floor as defense-in-depth (primary protection is in calculate_summary_max_tokens)
                summary_max_tokens = summary_max_tokens.max(Self::SUMMARY_MIN_TOKENS);

                debug!(
                    "Requesting summary with max_tokens: {} (current usage: {} tokens)",
                    summary_max_tokens, self.context_window.used_tokens
                );

                // Create summary request with FULL history
                let summary_prompt = self.context_window.create_summary_prompt();

                // Get the full conversation history
                let conversation_text = self
                    .context_window
                    .conversation_history
                    .iter()
                    .map(|m| format!("{:?}: {}", m.role, m.content))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let summary_messages = vec![
                    Message::new(
                        MessageRole::System,
                        "You are a helpful assistant that creates concise summaries.".to_string(),
                    ),
                    Message::new(
                        MessageRole::User,
                        format!(
                            "Based on this conversation history, {}\n\nConversation:\n{}",
                            summary_prompt, conversation_text
                        ),
                    ),
                ];

                let provider = self.providers.get(None)?;

                // Determine if we need to disable thinking mode for this request
                // Anthropic requires: max_tokens > thinking.budget_tokens + 1024
                let disable_thinking = self.get_thinking_budget_tokens(provider.name()).map_or(false, |budget| {
                    let minimum_for_thinking = budget + 1024;
                    let should_disable = summary_max_tokens <= minimum_for_thinking;
                    if should_disable {
                        tracing::warn!("Disabling thinking mode for summary: max_tokens ({}) <= minimum_for_thinking ({})", summary_max_tokens, minimum_for_thinking);
                    }
                    should_disable
                });

                tracing::debug!("Creating auto-summary request: max_tokens={}, disable_thinking={}", summary_max_tokens, disable_thinking);

                let summary_request = CompletionRequest {
                    messages: summary_messages,
                    max_tokens: Some(summary_max_tokens),
                    temperature: Some(self.resolve_temperature(provider.name())),
                    stream: false,
                    tools: None,
                    disable_thinking,
                };

                // Get the summary
                match provider.complete(summary_request).await {
                    Ok(summary_response) => {
                        self.ui_writer.print_context_status(
                            "✅ Context compacted successfully. Continuing...\n",
                        );

                        // Extract the latest user message from the request
                        let latest_user_msg = request
                            .messages
                            .iter()
                            .rev()
                            .find(|m| matches!(m.role, MessageRole::User))
                            .map(|m| m.content.clone());

                        // Reset context with summary
                        let chars_saved = self
                            .context_window
                            .reset_with_summary(summary_response.content, latest_user_msg);
                        self.compaction_events.push(chars_saved);

                        // Update the request with new context
                        request.messages = self.context_window.conversation_history.clone();
                    }
                    Err(e) => {
                        error!("Failed to create summary: {}", e);
                        self.ui_writer.print_context_status("⚠️ Unable to compact context. Consider starting a new session if you continue to see errors.\n");
                        // Don't continue with the original request if compaction failed
                        // as we're likely at token limit
                        return Err(anyhow::anyhow!("Context window at capacity and compaction failed. Please start a new session."));
                    }
                }
            }
        }

        loop {
            iteration_count += 1;
            debug!("Starting iteration {}", iteration_count);
            if iteration_count > MAX_ITERATIONS {
                warn!("Maximum iterations reached, stopping stream");
                break;
            }

            // Add a small delay between iterations to prevent "model busy" errors
            if iteration_count > 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            // Get provider info for logging, then drop it to avoid borrow issues
            let (provider_name, provider_model) = {
                let provider = self.providers.get(None)?;
                (provider.name().to_string(), provider.model().to_string())
            };
            debug!("Got provider: {}", provider_name);

            // Create error context for detailed logging
            let last_prompt = request
                .messages
                .iter()
                .rev()
                .find(|m| matches!(m.role, MessageRole::User))
                .map(|m| m.content.clone())
                .unwrap_or_else(|| "No user message found".to_string());

            let error_context = ErrorContext::new(
                "stream_completion".to_string(),
                provider_name.clone(),
                provider_model.clone(),
                last_prompt,
                self.session_id.clone(),
                self.context_window.used_tokens,
                self.quiet,
            )
            .with_request(
                serde_json::to_string(&request)
                    .unwrap_or_else(|_| "Failed to serialize request".to_string()),
            );

            // Log initial request details
            debug!("Starting stream with provider={}, model={}, messages={}, tools={}, max_tokens={:?}",
                provider_name,
                provider_model,
                request.messages.len(),
                request.tools.is_some(),
                request.max_tokens
            );

            // Try to get stream with retry logic
            let mut stream = match self.stream_with_retry(&request, &error_context).await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to start stream: {}", e);
                    // Additional retry for "busy" errors on subsequent iterations
                    if iteration_count > 1 && e.to_string().contains("busy") {
                        warn!(
                            "Model busy on iteration {}, attempting one more retry in 500ms",
                            iteration_count
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                        match self.stream_with_retry(&request, &error_context).await {
                            Ok(s) => s,
                            Err(e2) => {
                                error!("Failed to start stream after retry: {}", e2);
                                error_context.clone().log_error(&e2);
                                return Err(e2);
                            }
                        }
                    } else {
                        return Err(e);
                    }
                }
            };

            // Write context window summary every time we send messages to LLM
            self.write_context_window_summary();

            let mut parser = StreamingToolParser::new();
            let mut current_response = String::new();
            let mut tool_executed = false;
            let mut chunks_received = 0;
            let mut raw_chunks: Vec<String> = Vec::new(); // Store raw chunks for debugging
            let mut _last_error: Option<String> = None;
            let mut accumulated_usage: Option<g3_providers::Usage> = None;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        // Notify UI about SSE received (including pings)
                        self.ui_writer.notify_sse_received();

                        // Capture usage data if available
                        if let Some(ref usage) = chunk.usage {
                            accumulated_usage = Some(usage.clone());
                            turn_accumulated_usage = Some(usage.clone());
                            debug!(
                                "Received usage data - prompt: {}, completion: {}, total: {}",
                                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                            );
                        }

                        // Store raw chunk for debugging (limit to first 20 and last 5)
                        if chunks_received < 20 || chunk.finished {
                            raw_chunks.push(format!(
                                "Chunk #{}: content={:?}, finished={}, tool_calls={:?}",
                                chunks_received + 1,
                                chunk.content,
                                chunk.finished,
                                chunk.tool_calls
                            ));
                        } else if raw_chunks.len() == 20 {
                            raw_chunks.push("... (chunks 21+ omitted for brevity) ...".to_string());
                        }

                        // Record time to first token
                        if first_token_time.is_none() && !chunk.content.is_empty() {
                            first_token_time = Some(stream_start.elapsed());
                            // Record in agent metrics
                            if let Some(ttft) = first_token_time {
                                self.first_token_times.push(ttft);
                            }
                        }

                        chunks_received += 1;
                        if chunks_received == 1 {
                            debug!(
                                "First chunk received: content_len={}, finished={}",
                                chunk.content.len(),
                                chunk.finished
                            );
                        }

                        // Process chunk with the new parser
                        let completed_tools = parser.process_chunk(&chunk);

                        // Handle completed tool calls - process all if multiple calls enabled
                        // Always process all tool calls - they will be executed after stream ends
                        let tools_to_process: Vec<ToolCall> = completed_tools;

                        // Helper function to check if two tool calls are duplicates
                        let are_duplicates = |tc1: &ToolCall, tc2: &ToolCall| -> bool {
                            tc1.tool == tc2.tool && tc1.args == tc2.args
                        };

                        // De-duplicate tool calls and track duplicates
                        let mut last_tool_in_chunk: Option<ToolCall> = None;
                        let mut deduplicated_tools: Vec<(ToolCall, Option<String>)> = Vec::new();

                        for tool_call in tools_to_process {
                            let mut duplicate_type = None;

                            // Check for IMMEDIATELY SEQUENTIAL duplicate in current chunk
                            // Only the immediately previous tool call counts as a duplicate
                            if let Some(ref last_tool) = last_tool_in_chunk {
                                if are_duplicates(last_tool, &tool_call) {
                                duplicate_type = Some("DUP IN CHUNK".to_string());
                                }
                            } else {
                                // Check for duplicate against previous message
                                duplicate_type = self.check_duplicate_in_previous_message(&tool_call);
                            }

                            // Track the last tool call for sequential duplicate detection
                            last_tool_in_chunk = Some(tool_call.clone());

                            deduplicated_tools.push((tool_call, duplicate_type));
                        }

                        // Process each tool call
                        for (tool_call, duplicate_type) in deduplicated_tools {
                            debug!("Processing completed tool call: {:?}", tool_call);
                            
                            // If it's a duplicate, log it and skip - don't set tool_executed!
                            // Setting tool_executed for duplicates would trigger auto-continue
                            // even when no actual tool execution occurred.
                            if let Some(dup_type) = &duplicate_type {
                                // Log the duplicate with red prefix
                                let prefixed_tool_name =
                                    format!("🟥 {} {}", tool_call.tool, dup_type);
                                let warning_msg = format!(
                                    "⚠️ Duplicate tool call detected ({}): Skipping execution of {} with args {}",
                                    dup_type,
                                    tool_call.tool,
                                    serde_json::to_string(&tool_call.args).unwrap_or_else(|_| "<unserializable>".to_string())
                                );

                                // Log to tool log with red prefix
                                let mut modified_tool_call = tool_call.clone();
                                modified_tool_call.tool = prefixed_tool_name;
                                debug!("{}", warning_msg);

                                // NOTE: Do NOT call parser.reset() here!
                                // Resetting the parser clears the entire text buffer, which would
                                // lose any subsequent (non-duplicate) tool calls that haven't been
                                // processed yet.
                                continue; // Skip execution of duplicate
                            }

                            // Check if we should auto-compact at 90% BEFORE executing the tool
                            // We need to do this before any borrows of self
                            if self.auto_compact && self.context_window.percentage_used() >= 90.0 {
                                // Set flag to trigger compaction after this turn completes
                                // We can't do it now due to borrow checker constraints
                                self.pending_90_compaction = true;
                            }

                            // Check if we should thin the context BEFORE executing the tool
                            if self.context_window.should_thin() {
                                let thin_summary = self.do_thin_context();
                                // Print the thinning summary
                                self.ui_writer.print_context_thinning(&thin_summary);
                            }

                            // Track what we've already displayed before getting new text
                            // This prevents re-displaying old content after tool execution
                            let already_displayed_chars = current_response.chars().count();

                            // Get the text content accumulated so far
                            let text_content = parser.get_text_content();

                            // Clean the content
                            let clean_content = streaming::clean_llm_tokens(&text_content);

                            // Store the raw content BEFORE filtering for the context window log
                            let raw_content_for_log = clean_content.clone();

                            // Filter out JSON tool calls from the display
                            let filtered_content =
                                self.ui_writer.filter_json_tool_calls(&clean_content);
                            let final_display_content = filtered_content.trim();

                            // Display any new content before tool execution
                            // We need to skip what was already shown (tracked in current_response)
                            // but also account for the fact that parser.text_buffer accumulates
                            // across iterations and is never cleared until reset()
                            let new_content =
                                if current_response.len() <= final_display_content.len() {
                                    // Only show content that hasn't been displayed yet
                                    final_display_content
                                        .chars()
                                        .skip(already_displayed_chars)
                                        .collect::<String>()
                                } else {
                                    // Nothing new to display
                                    String::new()
                                };

                            // Don't display text before final_output - it will be in the summary
                            if !new_content.trim().is_empty() && tool_call.tool != "final_output" {
                                #[allow(unused_assignments)]
                                if !response_started {
                                    self.ui_writer.print_agent_prompt();
                                    response_started = true;
                                }
                                self.ui_writer.print_agent_response(&new_content);
                                self.ui_writer.flush();
                                // Update current_response to track what we've displayed
                                current_response.push_str(&new_content);
                            }

                            // Execute the tool with formatted output

                            // Skip printing tool call details for final_output
                            if tool_call.tool != "final_output" {
                                // Finish streaming markdown before showing tool output
                                self.ui_writer.finish_streaming_markdown();

                                // Tool call header
                                self.ui_writer.print_tool_header(&tool_call.tool, Some(&tool_call.args));
                                if let Some(args_obj) = tool_call.args.as_object() {
                                    for (key, value) in args_obj {
                                        let value_str = match value {
                                            serde_json::Value::String(s) => {
                                                if tool_call.tool == "shell" && key == "command" {
                                                    if let Some(first_line) = s.lines().next() {
                                                        if s.lines().count() > 1 {
                                                            format!("{}...", first_line)
                                                        } else {
                                                            first_line.to_string()
                                                        }
                                                    } else {
                                                        s.clone()
                                                    }
                                                } else if s.chars().count() > 100 {
                                                    streaming::truncate_for_display(s, 100)
                                                } else {
                                                    s.clone()
                                                }
                                            }
                                            _ => value.to_string(),
                                        };
                                        self.ui_writer.print_tool_arg(key, &value_str);
                                    }
                                }
                                self.ui_writer.print_tool_output_header();
                            }

                            // Clone working_dir to avoid borrow checker issues
                            let working_dir = self.working_dir.clone();
                            let exec_start = Instant::now();
                            // Add 8-minute timeout for tool execution
                            let tool_result = match tokio::time::timeout(
                                Duration::from_secs(8 * 60), // 8 minutes
                                // Use working_dir if set (from --codebase-fast-start)
                                self.execute_tool_in_dir(&tool_call, working_dir.as_deref()),
                            )
                            .await
                            {
                                Ok(result) => result?,
                                Err(_) => {
                                    warn!("Tool call {} timed out after 8 minutes", tool_call.tool);
                                    "❌ Tool execution timed out after 8 minutes".to_string()
                                }
                            };
                            let exec_duration = exec_start.elapsed();

                            // Track tool call metrics
                            let tool_success = !tool_result.contains("❌");
                            self.tool_call_metrics.push((
                                tool_call.tool.clone(),
                                exec_duration,
                                tool_success,
                            ));

                            // Display tool execution result with proper indentation
                            if tool_call.tool == "final_output" {
                                // For final_output, use the dedicated method that renders markdown
                                // with a spinner animation
                                self.ui_writer.print_final_output(&tool_result);
                            } else {
                                let output_lines: Vec<&str> = tool_result.lines().collect();

                                // Check if UI wants full output (machine mode) or truncated (human mode)
                                let wants_full = self.ui_writer.wants_full_output();

                                const MAX_LINES: usize = 5;
                                const MAX_LINE_WIDTH: usize = 80;
                                let output_len = output_lines.len();

                                // Skip printing for todo tools - they already print their content
                                let is_todo_tool =
                                    tool_call.tool == "todo_read" || tool_call.tool == "todo_write";

                                if !is_todo_tool {
                                    let max_lines_to_show = if wants_full { output_len } else { MAX_LINES };

                                    for (idx, line) in output_lines.iter().enumerate() {
                                        if !wants_full && idx >= max_lines_to_show {
                                            break;
                                        }
                                        let clipped_line = streaming::truncate_line(line, MAX_LINE_WIDTH, !wants_full);
                                        self.ui_writer.update_tool_output_line(&clipped_line);
                                    }

                                    if !wants_full && output_len > MAX_LINES {
                                        self.ui_writer.print_tool_output_summary(output_len);
                                    }
                                }
                            }

                            // Add the tool call and result to the context window using RAW unfiltered content
                            // This ensures the log file contains the true raw content including JSON tool calls
                            let tool_message = if !raw_content_for_log.trim().is_empty() {
                                Message::new(
                                    MessageRole::Assistant,
                                    format!(
                                        "{}\n\n{{\"tool\": \"{}\", \"args\": {}}}",
                                        raw_content_for_log.trim(),
                                        tool_call.tool,
                                        tool_call.args
                                    ),
                                )
                            } else {
                                // No text content before tool call, just include the tool call
                                Message::new(
                                    MessageRole::Assistant,
                                    format!(
                                        "{{\"tool\": \"{}\", \"args\": {}}}",
                                        tool_call.tool, tool_call.args
                                    ),
                                )
                            };
                            let mut result_message = {
                                // Check if we should use cache control (every 10 tool calls)
                                // But only if we haven't already added 4 cache_control annotations
                                if self.tool_call_count > 0
                                    && self.tool_call_count % 10 == 0
                                    && self.count_cache_controls_in_history() < 4
                                {
                                    let provider = self.providers.get(None)?;
                                    let provider_name = provider.name();
                                    let (provider_type, config_name) = provider_config::parse_provider_ref(provider_name);
                                    if let Some(cache_config) = match provider_type {
                                        "anthropic" => {
                                            self.config
                                                .providers
                                                .anthropic
                                                .get(config_name)
                                                .and_then(|c| c.cache_config.as_ref())
                                                .and_then(|config| Self::parse_cache_control(config))
                                        }
                                        _ => None,
                                    } {
                                        Message::with_cache_control_validated(
                                            MessageRole::User,
                                            format!("Tool result: {}", tool_result),
                                            cache_config,
                                            provider,
                                        )
                                    } else {
                                        Message::new(
                                            MessageRole::User,
                                            format!("Tool result: {}", tool_result),
                                        )
                                    }
                                } else {
                                    Message::new(
                                        MessageRole::User,
                                        format!("Tool result: {}", tool_result),
                                    )
                                }
                            };

                            // Attach any pending images to the result message
                            // (images loaded via read_image tool)
                            if !self.pending_images.is_empty() {
                                result_message.images = std::mem::take(&mut self.pending_images);
                            }

                            // Track tokens before adding messages
                            let tokens_before = self.context_window.used_tokens;

                            self.context_window.add_message(tool_message);
                            self.context_window.add_message(result_message);

                            // Check if this was a final_output tool call
                            if tool_call.tool == "final_output" {
                                // Finish the streaming markdown formatter before final_output
                                self.ui_writer.finish_streaming_markdown();

                                // Save context window BEFORE returning so the session log includes final_output
                                self.save_context_window("completed");
                                
                                // The summary was already displayed via print_final_output
                                // Don't add it to full_response to avoid duplicate printing
                                // full_response is intentionally left empty/unchanged
                                let _ttft =
                                    first_token_time.unwrap_or_else(|| stream_start.elapsed());

                                // Add timing if needed
                                let final_response = if show_timing {
                                    format!(
                                        "🕝 {} | 💭 {}",
                                        Self::format_duration(stream_start.elapsed()),
                                        Self::format_duration(_ttft)
                                    )
                                } else {
                                    // Return empty string since content was already displayed
                                    String::new()
                                };

                                return Ok(TaskResult::new(
                                    final_response,
                                    self.context_window.clone(),
                                ));
                            }

                            // Closure marker with timing
                            if tool_call.tool != "final_output" {
                                let tokens_delta = self.context_window.used_tokens.saturating_sub(tokens_before);
                                self.ui_writer
                                    .print_tool_timing(&Self::format_duration(exec_duration),
                                        tokens_delta,
                                        self.context_window.percentage_used());
                                self.ui_writer.print_agent_prompt();
                            }

                            // Update the request with the new context for next iteration
                            request.messages = self.context_window.conversation_history.clone();

                            // Ensure tools are included for native providers in subsequent iterations
                            let provider_for_tools = self.providers.get(None)?;
                            if provider_for_tools.has_native_tool_calling() {
                                request.tools = Some(tool_definitions::create_tool_definitions(
                                    tool_definitions::ToolConfig::new(
                                        self.config.webdriver.enabled,
                                        self.config.computer_control.enabled,
                                    )));
                            }

                            // DO NOT add final_display_content to full_response here!
                            // The content was already displayed during streaming and added to current_response.
                            // Adding it again would cause duplication when the agent message is printed.
                            // The only time we should add to full_response is:
                            // 1. For final_output tool (handled separately)
                            // 2. At the end when no tools were executed (handled in the "no tool executed" branch)

                            tool_executed = true;
                            any_tool_executed = true; // Track across all iterations

                            // Reset auto-continue attempts after successful tool execution
                            // This gives the LLM fresh attempts since it's making progress
                            auto_summary_attempts = 0;


                            // Reset the JSON tool call filter state after each tool execution
                            // This ensures the filter doesn't stay in suppression mode for subsequent streaming content
                            self.ui_writer.reset_json_filter();

                            // Only reset parser if there are no more unexecuted tool calls in the buffer
                            // This handles the case where the LLM emits multiple tool calls in one response
                            if parser.has_unexecuted_tool_call() {
                                debug!("Parser still has unexecuted tool calls, not resetting buffer");
                                // Mark current tool as consumed so we don't re-detect it
                                parser.mark_tool_calls_consumed();
                            } else {
                                // Reset parser for next iteration - this clears the text buffer
                                parser.reset();
                            }

                            // Clear current_response for next iteration to prevent buffered text
                            // from being incorrectly displayed after tool execution
                            current_response.clear();
                            // Reset response_started flag for next iteration
                            response_started = false;

                            // Continue processing - don't break mid-stream
                        } // End of for loop processing each tool call

                        // Note: We no longer break mid-stream after tool execution.
                        // All tool calls are collected and executed after the stream ends.

                        // If no tool calls were completed, continue streaming normally
                        if !tool_executed {
                            let clean_content = streaming::clean_llm_tokens(&chunk.content);

                            if !clean_content.is_empty() {
                                let filtered_content =
                                    self.ui_writer.filter_json_tool_calls(&clean_content);

                                if !filtered_content.is_empty() {
                                    if !response_started {
                                        self.ui_writer.print_agent_prompt();
                                        response_started = true;
                                    }

                                    self.ui_writer.print_agent_response(&filtered_content);
                                    self.ui_writer.flush();
                                    current_response.push_str(&filtered_content);

                                    // Mark parser buffer as consumed up to current position
                                    // This prevents tool-call-like patterns in displayed text
                                    // from triggering false positives in has_unexecuted_tool_call()
                                    parser.mark_tool_calls_consumed();
                                }
                            }
                        }

                        if chunk.finished {
                            debug!("Stream finished: tool_executed={}, current_response_len={}, full_response_len={}, chunks_received={}",
                                tool_executed, current_response.len(), full_response.len(), chunks_received);

                            // Stream finished - check if we should continue or return
                            if !tool_executed {
                                // No tools were executed in this iteration
                                // Check if we got any meaningful response at all
                                // We need to check the parser's text buffer as well, since the LLM
                                // might have responded with text but no final_output tool call
                                let text_content = parser.get_text_content();
                                let has_text_response = !text_content.trim().is_empty()
                                    || !current_response.trim().is_empty();

                                // Don't re-add text from parser buffer if we already displayed it
                                // The parser buffer contains ALL accumulated text, but current_response
                                // already has what was displayed during streaming
                                if current_response.is_empty() && !text_content.trim().is_empty() {
                                    // Only use parser text if we truly have no response
                                    // This should be rare - only if streaming failed to display anything
                                    debug!("Warning: Using parser buffer text as fallback - this may duplicate output");
                                    // Extract only the undisplayed portion from parser buffer
                                    // Parser buffer accumulates across iterations, so we need to be careful
                                    let clean_text = streaming::clean_llm_tokens(&text_content);

                                    let filtered_text =
                                        self.ui_writer.filter_json_tool_calls(&clean_text);

                                    // Only use this if we truly have nothing else
                                    if !filtered_text.trim().is_empty() && full_response.is_empty()
                                    {
                                        debug!(
                                            "Using filtered parser text as last resort: {} chars",
                                            filtered_text.len()
                                        );
                                        // Note: This assignment is currently unused but kept for potential future use
                                        let _ = filtered_text;
                                    }
                                }

                                if !has_text_response && full_response.is_empty() {
                                    streaming::log_stream_error(
                                        iteration_count,
                                        &provider_name,
                                        &provider_model,
                                        chunks_received,
                                        &parser,
                                        &request,
                                        &self.context_window,
                                        self.session_id.as_deref(),
                                        &raw_chunks,
                                    );

                                    // No response received - this is an error condition
                                    warn!("Stream finished without any content or tool calls");
                                    warn!("Chunks received: {}", chunks_received);
                                    return Err(anyhow::anyhow!(
                                        "No response received from the model. The model may be experiencing issues or the request may have been malformed."
                                    ));
                                }

                                // If tools were executed in previous iterations but final_output wasn't called,
                                // break to let the outer loop's auto-continue logic handle it
                                if any_tool_executed && !final_output_called {
                                    debug!("Tools were executed but final_output not called - breaking to auto-continue");
                                    // NOTE: We intentionally do NOT set full_response here.
                                    // The content was already displayed during streaming.
                                    // Setting full_response would cause duplication when the
                                    // function eventually returns.
                                    // Context window is updated separately via add_message().
                                    break;
                                }

                                // Set full_response to current_response (don't append)
                                // current_response already contains everything that was displayed
                                // Don't set full_response here - it would duplicate the output
                                // The text was already displayed during streaming
                                // Return empty string to avoid duplication
                                full_response = String::new();

                                // Finish the streaming markdown formatter before returning
                                self.ui_writer.finish_streaming_markdown();

                                // Save context window BEFORE returning
                                self.save_context_window("completed");
                                let _ttft =
                                    first_token_time.unwrap_or_else(|| stream_start.elapsed());

                                // Add timing if needed
                                let final_response = if show_timing {
                                    let turn_tokens = turn_accumulated_usage.as_ref().map(|u| u.total_tokens);
                                    let timing_footer = Self::format_timing_footer(
                                        stream_start.elapsed(),
                                        _ttft,
                                        turn_tokens,
                                        self.context_window.percentage_used(),
                                    );
                                    format!(
                                        "{}\n\n{}",
                                        full_response,
                                        timing_footer
                                    )
                                } else {
                                    full_response
                                };

                                return Ok(TaskResult::new(
                                    final_response,
                                    self.context_window.clone(),
                                ));
                            }
                            break; // Tool was executed, break to continue outer loop
                        }
                    }
                    Err(e) => {
                        // Capture detailed streaming error information
                        let error_msg = e.to_string();
                        let error_details = format!(
                            "Streaming error at chunk {}: {}",
                            chunks_received + 1,
                            error_msg
                        );

                        error!("Error type: {}", std::any::type_name_of_val(&e));
                        error!("Parser state at error: text_buffer_len={}, has_incomplete={}, message_stopped={}",
                            parser.text_buffer_len(), parser.has_incomplete_tool_call(), parser.is_message_stopped());

                        // Store the error for potential logging later
                        _last_error = Some(error_details.clone());

                        // Check if this is a recoverable connection error
                        let is_connection_error = streaming::is_connection_error(&error_msg);

                        if is_connection_error {
                            warn!(
                                "Connection error at chunk {}, treating as end of stream",
                                chunks_received + 1
                            );
                            // If we have any content or tool calls, treat this as a graceful end
                            if chunks_received > 0
                                && (!parser.get_text_content().is_empty()
                                    || parser.has_unexecuted_tool_call())
                            {
                                warn!("Stream terminated unexpectedly but we have content, continuing");
                                break; // Break to process what we have
                            }
                        }

                        if tool_executed {
                            error!("{}", error_details);
                            warn!("Stream error after tool execution, attempting to continue");
                            break; // Break to outer loop to start new stream
                        } else {
                            // Log raw chunks before failing
                            error!("Fatal streaming error. Raw chunks received before error:");
                            for chunk_str in raw_chunks.iter().take(10) {
                                error!("  {}", chunk_str);
                            }
                            return Err(e);
                        }
                    }
                }
            }

            // Update context window with actual usage if available
            if let Some(usage) = accumulated_usage {
                debug!("Updating context window with actual usage from stream");
                self.context_window.update_usage_from_response(&usage);
            } else {
                // Fall back to estimation if no usage data was provided
                debug!("No usage data from stream, using estimation");
                let estimated_tokens = ContextWindow::estimate_tokens(&current_response);
                self.context_window.add_streaming_tokens(estimated_tokens);
            }

            // If we get here and no tool was executed, we're done
            if !tool_executed {
                // IMPORTANT: Do NOT add parser text_content here!
                // The text has already been displayed during streaming via current_response.
                // The parser buffer accumulates ALL text and would cause duplication.
                debug!("Stream completed without tool execution. Response already displayed during streaming.");
                debug!(
                    "Current response length: {}, Full response length: {}",
                    current_response.len(),
                    full_response.len()
                );

                let has_response = !current_response.is_empty() || !full_response.is_empty();

                // Check if the response is essentially empty (just whitespace or timing lines)
                // This detects cases where the LLM outputs nothing substantive
                let response_text = if !current_response.is_empty() {
                    &current_response
                } else {
                    &full_response
                };
                let is_empty_response = streaming::is_empty_response(response_text);

                // Check if there's an incomplete tool call in the buffer
                let has_incomplete_tool_call = parser.has_incomplete_tool_call();

                // Check if there's a complete but unexecuted tool call in the buffer
                let has_unexecuted_tool_call = parser.has_unexecuted_tool_call();

                // Log when we detect unexecuted or incomplete tool calls for debugging
                if has_incomplete_tool_call {
                    debug!("Detected incomplete tool call in buffer (buffer_len={}, consumed_up_to={})",
                        parser.text_buffer_len(), parser.text_buffer_len());
                }
                if has_unexecuted_tool_call {
                    debug!("Detected unexecuted tool call in buffer - this may indicate a parsing issue");
                    warn!("Unexecuted tool call detected in buffer after stream ended");
                }

                // Auto-continue if tools were executed but final_output was never called
                // OR if the LLM emitted an incomplete tool call (truncated JSON)
                // OR if the LLM emitted a complete tool call that wasn't executed
                // This ensures we don't return control when the LLM clearly intended to call a tool
                // Note: We removed the redundant condition (any_tool_executed && is_empty_response)
                // because it's already covered by (any_tool_executed && !final_output_called)
                // Auto-continue is only enabled in autonomous mode - in interactive mode,
                // the user may be asking questions and we should return control to them
                let should_auto_continue = self.is_autonomous && ((any_tool_executed && !final_output_called) 
                    || has_incomplete_tool_call 
                    || has_unexecuted_tool_call);
                if should_auto_continue {
                    if auto_summary_attempts < MAX_AUTO_SUMMARY_ATTEMPTS {
                        auto_summary_attempts += 1;
                        if has_incomplete_tool_call {
                            warn!(
                                "LLM emitted incomplete tool call ({} iterations, auto-continue attempt {}/{})",
                                iteration_count, auto_summary_attempts, MAX_AUTO_SUMMARY_ATTEMPTS
                            );
                            self.ui_writer.print_context_status(
                                "\n🔄 Model emitted incomplete tool call. Auto-continuing...\n"
                            );
                        } else if has_unexecuted_tool_call {
                            warn!(
                                "LLM emitted unexecuted tool call ({} iterations, auto-continue attempt {}/{})",
                                iteration_count, auto_summary_attempts, MAX_AUTO_SUMMARY_ATTEMPTS
                            );
                            self.ui_writer.print_context_status(
                                "\n🔄 Model emitted tool call that wasn't executed. Auto-continuing...\n"
                            );
                        } else if is_empty_response {
                            warn!(
                                "LLM emitted empty/trivial response ({} iterations, auto-continue attempt {}/{})",
                                iteration_count, auto_summary_attempts, MAX_AUTO_SUMMARY_ATTEMPTS
                            );
                            self.ui_writer.print_context_status(
                                "\n🔄 Model emitted empty response. Auto-continuing...\n"
                            );
                        } else {
                            warn!(
                                "LLM stopped without calling final_output after executing tools ({} iterations, auto-continue attempt {}/{})",
                                iteration_count, auto_summary_attempts, MAX_AUTO_SUMMARY_ATTEMPTS
                            );
                            self.ui_writer.print_context_status(
                                "\n🔄 Model stopped without calling final_output. Auto-continuing...\n"
                            );
                        }
                        
                        // Add any text response to context before prompting for continuation
                        if has_response {
                            let response_text = if !current_response.is_empty() {
                                current_response.clone()
                            } else {
                                full_response.clone()
                            };
                            if !response_text.trim().is_empty() {
                                let assistant_msg = Message::new(
                                    MessageRole::Assistant,
                                    response_text.trim().to_string(),
                                );
                                self.context_window.add_message(assistant_msg);
                            }
                        }
                        
                        // Add a follow-up message asking for continuation
                        let continue_prompt = if has_incomplete_tool_call {
                            Message::new(
                                MessageRole::User,
                                "Your previous response was cut off mid-tool-call. Please complete the tool call and continue.".to_string(),
                            )
                        } else {
                            Message::new(
                                MessageRole::User,
                                "Please continue until you are done. You **MUST** call `final_output` with a summary when done.".to_string(),
                            )
                        };
                        self.context_window.add_message(continue_prompt);
                        request.messages = self.context_window.conversation_history.clone();
                        
                        // Continue the loop
                        continue;
                    } else {
                        // Max attempts reached, give up gracefully
                        warn!(
                            "Max auto-continue attempts ({}) reached after {} iterations. Conditions: any_tool_executed={}, final_output_called={}, has_incomplete={}, has_unexecuted={}, is_empty_response={}",
                            MAX_AUTO_SUMMARY_ATTEMPTS,
                            iteration_count,
                            any_tool_executed,
                            final_output_called,
                            has_incomplete_tool_call,
                            has_unexecuted_tool_call,
                            is_empty_response
                        );
                        self.ui_writer.print_agent_response(
                            &format!("\n⚠️ The model stopped without calling final_output after {} auto-continue attempts.\n", MAX_AUTO_SUMMARY_ATTEMPTS)
                        );
                    }
                } else if has_response {
                    // Only set full_response if it's empty (first iteration without tools)
                    // This prevents duplication when the agent responds without calling final_output
                    // NOTE: We intentionally do NOT set full_response here anymore.
                    // The content was already displayed during streaming via print_agent_response().
                    // Setting full_response would cause the CLI to print it again.
                    // We only need full_response for the context window (handled separately).
                    debug!(
                        "Response already streamed, not setting full_response. current_response: {} chars",
                        current_response.len()
                    );
                }

                let _ttft = first_token_time.unwrap_or_else(|| stream_start.elapsed());

                // Add the RAW unfiltered response to context window before returning
                // This ensures the log contains the true raw content including any JSON
                if !full_response.trim().is_empty() {
                    // Get the raw text from the parser (before filtering)
                    let raw_text = parser.get_text_content();
                    let raw_clean = streaming::clean_llm_tokens(&raw_text);

                    if !raw_clean.trim().is_empty() {
                        let assistant_message = Message::new(MessageRole::Assistant, raw_clean);
                        self.context_window.add_message(assistant_message);
                    }
                }

                // Save context window BEFORE returning
                self.save_context_window("completed");
                
                // Add timing if needed
                let final_response = if show_timing {
                    let turn_tokens = turn_accumulated_usage.as_ref().map(|u| u.total_tokens);
                    let timing_footer = Self::format_timing_footer(
                        stream_start.elapsed(),
                        _ttft,
                        turn_tokens,
                        self.context_window.percentage_used(),
                    );
                    format!(
                        "{}\n\n{}",
                        full_response,
                        timing_footer
                    )
                } else {
                    full_response
                };

                return Ok(TaskResult::new(final_response, self.context_window.clone()));
            }

            // Continue the loop to start a new stream with updated context
        }

        // If we exit the loop due to max iterations
        let _ttft = first_token_time.unwrap_or_else(|| stream_start.elapsed());

        // Add timing if needed
        let final_response = if show_timing {
            let turn_tokens = turn_accumulated_usage.as_ref().map(|u| u.total_tokens);
            let timing_footer = Self::format_timing_footer(
                stream_start.elapsed(),
                _ttft,
                turn_tokens,
                self.context_window.percentage_used(),
            );
            format!(
                "{}\n\n{}",
                full_response,
                timing_footer
            )
        } else {
            full_response
        };

        Ok(TaskResult::new(final_response, self.context_window.clone()))
    }

    pub async fn execute_tool(&mut self, tool_call: &ToolCall) -> Result<String> {
        // Increment tool call count
        self.tool_call_count += 1;
        self.execute_tool_in_dir(tool_call, None).await
    }

    /// Execute a tool with an optional working directory (for discovery commands)
    pub async fn execute_tool_in_dir(
        &mut self,
        tool_call: &ToolCall,
        working_dir: Option<&str>,
    ) -> Result<String> {
        // Only increment tool call count if not already incremented by execute_tool
        if working_dir.is_some() {
            self.tool_call_count += 1;
        }

        let result = self.execute_tool_inner_in_dir(tool_call, working_dir).await;
        let log_str = match &result {
            Ok(s) => s.clone(),
            Err(e) => format!("ERROR: {}", e),
        };
        debug!("Tool {} completed: {}", tool_call.tool, &log_str.chars().take(100).collect::<String>());
        result
    }


    async fn execute_tool_inner_in_dir(
        &mut self,
        tool_call: &ToolCall,
        working_dir: Option<&str>,
    ) -> Result<String> {
        debug!("=== EXECUTING TOOL ===");
        debug!("Tool name: {}", tool_call.tool);
        debug!(
            "Working directory passed to execute_tool_inner_in_dir: {:?}",
            working_dir
        );
        debug!("Tool args (raw): {:?}", tool_call.args);
        debug!(
            "Tool args (JSON): {}",
            serde_json::to_string(&tool_call.args)
                .unwrap_or_else(|_| "failed to serialize".to_string())
        );
        debug!("======================");

        // Create tool context for dispatch
        let mut ctx = tools::executor::ToolContext {
            config: &self.config,
            ui_writer: &self.ui_writer,
            session_id: self.session_id.as_deref(),
            working_dir,
            computer_controller: self.computer_controller.as_ref(),
            webdriver_session: &self.webdriver_session,
            webdriver_process: &self.webdriver_process,
            background_process_manager: &self.background_process_manager,
            todo_content: &self.todo_content,
            pending_images: &mut self.pending_images,
            is_autonomous: self.is_autonomous,
            requirements_sha: self.requirements_sha.as_deref(),
            context_total_tokens: self.context_window.total_tokens,
            context_used_tokens: self.context_window.used_tokens,
        };

        // Dispatch to the appropriate tool handler
        let result = tool_dispatch::dispatch_tool(tool_call, &mut ctx).await?;

        // Handle special case: final_output needs to save session continuation
        if tool_call.tool == "final_output" {
            let summary = tool_call.args.get("summary").and_then(|v| v.as_str());
            self.save_session_continuation(summary.map(|s| s.to_string()));
        }

        Ok(result)
    }


    fn format_duration(duration: Duration) -> String {
        streaming::format_duration(duration)
    }

    fn format_timing_footer(
        elapsed: Duration,
        ttft: Duration,
        turn_tokens: Option<u32>,
        context_percentage: f32,
    ) -> String {
        streaming::format_timing_footer(elapsed, ttft, turn_tokens, context_percentage)
    }
}


// Re-export utility functions
pub use utils::apply_unified_diff_to_string;

// Implement Drop to clean up safaridriver process
impl<W: UiWriter> Drop for Agent<W> {
    fn drop(&mut self) {
        // Validate system prompt invariant on drop (agent exit)
        // This catches any bugs where the conversation history was corrupted during execution
        if !self.context_window.conversation_history.is_empty() {
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.validate_system_prompt_is_first();
            })) {
                eprintln!(
                    "\n⚠️  FATAL ERROR ON EXIT: System prompt validation failed: {:?}",
                    e
                );
            }
        }

        // Try to kill safaridriver process if it's still running
        // We need to use try_lock since we can't await in Drop
        if let Ok(mut process_guard) = self.webdriver_process.try_write() {
            if let Some(process) = process_guard.take() {
                // Use blocking kill since we can't await in Drop
                // This is a best-effort cleanup
                let _ = std::process::Command::new("kill")
                    .arg("-9")
                    .arg(process.id().unwrap_or(0).to_string())
                    .output();

                debug!("Attempted to clean up safaridriver process on Agent drop");
            }
        }
    }
}
