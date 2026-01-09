//! Provider configuration resolution.
//!
//! This module handles resolving provider-specific configuration values
//! like max_tokens, temperature, and thinking budget tokens from the
//! hierarchical config structure.

use g3_config::Config;
use tracing::warn;

/// Minimum tokens for summary requests to avoid API errors when context is nearly full.
pub const SUMMARY_MIN_TOKENS: u32 = 1000;

/// Parse a provider reference into (provider_type, config_name).
/// Format: "provider_type.config_name" (e.g., "anthropic.default")
/// Falls back to (provider_name, "default") for simple names.
pub fn parse_provider_ref(provider_name: &str) -> (&str, &str) {
    let parts: Vec<&str> = provider_name.split('.').collect();
    if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        (provider_name, "default")
    }
}

/// Get the configured max_tokens for a provider from config.
pub fn get_max_tokens(config: &Config, provider_name: &str) -> Option<u32> {
    let (provider_type, config_name) = parse_provider_ref(provider_name);
    
    match provider_type {
        "anthropic" => config.providers.anthropic.get(config_name)?.max_tokens,
        "openai" => config.providers.openai.get(config_name)?.max_tokens,
        "databricks" => config.providers.databricks.get(config_name)?.max_tokens,
        "embedded" => config.providers.embedded.get(config_name)?.max_tokens,
        _ => None,
    }
}

/// Get the configured temperature for a provider from config.
pub fn get_temperature(config: &Config, provider_name: &str) -> Option<f32> {
    let (provider_type, config_name) = parse_provider_ref(provider_name);
    
    match provider_type {
        "anthropic" => config.providers.anthropic.get(config_name)?.temperature,
        "openai" => config.providers.openai.get(config_name)?.temperature,
        "databricks" => config.providers.databricks.get(config_name)?.temperature,
        "embedded" => config.providers.embedded.get(config_name)?.temperature,
        _ => None,
    }
}

/// Get the thinking budget tokens for Anthropic provider, if configured.
pub fn get_thinking_budget_tokens(config: &Config, provider_name: &str) -> Option<u32> {
    let (provider_type, config_name) = parse_provider_ref(provider_name);
    
    // Only Anthropic has thinking_budget_tokens
    if provider_type != "anthropic" {
        return None;
    }
    
    config.providers.anthropic
        .get(config_name)
        .and_then(|c| c.thinking_budget_tokens)
}

/// Resolve the max_tokens to use for a given provider, applying fallbacks.
pub fn resolve_max_tokens(config: &Config, provider_name: &str) -> u32 {
    let (provider_type, _) = parse_provider_ref(provider_name);
    
    let base = match provider_type {
        "databricks" => get_max_tokens(config, provider_name)
            .or(Some(config.agent.fallback_default_max_tokens as u32))
            .unwrap_or(32000),
        _ => get_max_tokens(config, provider_name)
            .or(Some(config.agent.fallback_default_max_tokens as u32))
            .unwrap_or(16000),
    };
    
    // For Anthropic with thinking enabled, ensure max_tokens is sufficient
    // Anthropic requires: max_tokens > thinking.budget_tokens
    if provider_type == "anthropic" {
        if let Some(budget) = get_thinking_budget_tokens(config, provider_name) {
            let minimum_for_thinking = budget + 1024;
            return base.max(minimum_for_thinking);
        }
    }
    
    base
}

/// Resolve the temperature to use for a given provider, applying fallbacks.
pub fn resolve_temperature(config: &Config, provider_name: &str) -> f32 {
    let (provider_type, _) = parse_provider_ref(provider_name);
    
    match provider_type {
        "databricks" => get_temperature(config, provider_name).unwrap_or(0.1),
        _ => get_temperature(config, provider_name).unwrap_or(0.1),
    }
}

/// Pre-flight check to validate and adjust max_tokens for the thinking.budget_tokens constraint.
/// Returns the adjusted max_tokens that satisfies: max_tokens > thinking.budget_tokens
/// Also returns whether we need to apply fallback actions (thinnify/skinnify).
///
/// Returns: (adjusted_max_tokens, needs_context_reduction)
pub fn preflight_validate_max_tokens(
    config: &Config,
    provider_name: &str,
    proposed_max_tokens: u32,
) -> (u32, bool) {
    let (provider_type, _) = parse_provider_ref(provider_name);
    
    // Only applies to Anthropic provider
    if provider_type != "anthropic" {
        return (proposed_max_tokens, false);
    }

    let budget_tokens = match get_thinking_budget_tokens(config, provider_name) {
        Some(budget) => budget,
        None => return (proposed_max_tokens, false), // No thinking enabled
    };

    // Anthropic requires: max_tokens > budget_tokens
    // We add a minimum output buffer of 1024 tokens for actual response content
    let minimum_required = budget_tokens + 1024;

    if proposed_max_tokens >= minimum_required {
        // We have enough headroom
        (proposed_max_tokens, false)
    } else {
        // max_tokens is too low - need to either adjust or reduce context
        warn!(
            "max_tokens ({}) is below required minimum ({}) for thinking.budget_tokens ({}). Context reduction needed.",
            proposed_max_tokens, minimum_required, budget_tokens
        );
        // Return the minimum required, but flag that we need context reduction
        (minimum_required, true)
    }
}

/// Calculate max_tokens for a summary request, ensuring it satisfies the thinking constraint.
/// Returns (max_tokens, whether_fallback_is_needed)
/// 
/// IMPORTANT: Always returns at least SUMMARY_MIN_TOKENS to avoid API errors
/// when context is nearly full (90%+).
pub fn calculate_summary_max_tokens(
    config: &Config,
    provider_name: &str,
    model_limit: u32,
    current_usage: u32,
) -> (u32, bool) {
    let (provider_type, _) = parse_provider_ref(provider_name);
    
    // Get the configured max_tokens for this provider
    let configured_max_tokens = resolve_max_tokens(config, provider_name);
    
    // Calculate available tokens with buffer
    let buffer = (model_limit / 40).clamp(1000, 10000); // 2.5% buffer
    let available = model_limit
        .saturating_sub(current_usage)
        .saturating_sub(buffer);
    // Ensure we have at least a minimum floor for summary requests
    // This prevents max_tokens=0 errors when context is 90%+ full
    let available = available.max(SUMMARY_MIN_TOKENS);
    
    // Use the smaller of available tokens (with floor) or configured max_tokens,
    // but ensure we don't go below thinking budget floor for Anthropic
    let proposed_max_tokens = available.min(configured_max_tokens);
    let proposed_max_tokens = if provider_type == "anthropic" {
        if let Some(budget) = get_thinking_budget_tokens(config, provider_name) {
            proposed_max_tokens.max(budget + 1024)
        } else {
            proposed_max_tokens
        }
    } else {
        proposed_max_tokens
    };

    // Validate against thinking budget constraint
    preflight_validate_max_tokens(config, provider_name, proposed_max_tokens)
}

/// Get the provider-specific cap for summary max_tokens.
pub fn get_summary_max_tokens_cap(config: &Config, provider_name: &str) -> u32 {
    let (provider_type, _) = parse_provider_ref(provider_name);
    
    // For Anthropic with thinking enabled, we need max_tokens > thinking.budget_tokens
    // So we set a higher cap when thinking is configured
    match provider_type {
        "anthropic" => {
            match get_thinking_budget_tokens(config, provider_name) {
                Some(budget) => (budget + 2000).max(10_000),
                None => 10_000,
            }
        }
        "databricks" => 10_000,
        "embedded" => 3000,
        _ => 5000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider_ref_with_dot() {
        let (ptype, name) = parse_provider_ref("anthropic.default");
        assert_eq!(ptype, "anthropic");
        assert_eq!(name, "default");
    }

    #[test]
    fn test_parse_provider_ref_simple() {
        let (ptype, name) = parse_provider_ref("anthropic");
        assert_eq!(ptype, "anthropic");
        assert_eq!(name, "default");
    }

    #[test]
    fn test_parse_provider_ref_with_custom_name() {
        let (ptype, name) = parse_provider_ref("openai.gpt4");
        assert_eq!(ptype, "openai");
        assert_eq!(name, "gpt4");
    }
}
