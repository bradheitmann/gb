//! Message Serialization Integration Tests
//!
//! CHARACTERIZATION: These tests verify that Message and related types
//! serialize and deserialize correctly, ensuring data integrity across
//! the system boundary (JSON API communication).
//!
//! What these tests protect:
//! - Message round-trip serialization/deserialization
//! - MessageRole enum serialization
//! - Cache control serialization
//! - Tool definitions serialization
//!
//! What these tests intentionally do NOT assert:
//! - Provider-specific API formats (tested elsewhere)
//! - Internal field ordering in JSON
//! - Whitespace or formatting in serialized output

use g3_providers::{Message, MessageRole, CacheControl, Tool};
use serde_json::json;

// =============================================================================
// Helper functions for comparison (types don't implement PartialEq)
// =============================================================================

fn role_matches(role: &MessageRole, expected: &str) -> bool {
    let json = serde_json::to_string(role).unwrap();
    json.to_lowercase().contains(&expected.to_lowercase())
}

fn cache_control_is_ephemeral(cc: &Option<CacheControl>) -> bool {
    match cc {
        Some(c) => {
            let json = serde_json::to_string(c).unwrap();
            json.contains("ephemeral")
        }
        None => false,
    }
}

fn cache_control_is_five_minute(cc: &Option<CacheControl>) -> bool {
    match cc {
        Some(c) => {
            let json = serde_json::to_string(c).unwrap();
            // Check for "5m" TTL
            json.contains("5m")
        }
        None => false,
    }
}

fn cache_control_is_one_hour(cc: &Option<CacheControl>) -> bool {
    match cc {
        Some(c) => {
            let json = serde_json::to_string(c).unwrap();
            // Check for "1h" TTL
            json.contains("1h")
        }
        None => false,
    }
}

// =============================================================================
// Test: Message round-trip serialization
// =============================================================================

#[test]
fn test_message_roundtrip_user() {
    let original = Message::new(MessageRole::User, "Hello, world!".to_string());
    
    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    
    // Deserialize back
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert!(role_matches(&restored.role, "user"));
    assert_eq!(restored.content, "Hello, world!");
}

#[test]
fn test_message_roundtrip_assistant() {
    let original = Message::new(MessageRole::Assistant, "I can help with that.".to_string());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert!(role_matches(&restored.role, "assistant"));
    assert_eq!(restored.content, "I can help with that.");
}

#[test]
fn test_message_roundtrip_system() {
    let original = Message::new(MessageRole::System, "You are a helpful assistant.".to_string());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert!(role_matches(&restored.role, "system"));
    assert_eq!(restored.content, "You are a helpful assistant.");
}

// =============================================================================
// Test: Message with special characters
// =============================================================================

#[test]
fn test_message_with_unicode() {
    let content = "Hello 世界! 🌍 Привет мир!";
    let original = Message::new(MessageRole::User, content.to_string());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.content, content);
}

#[test]
fn test_message_with_newlines() {
    let content = "Line 1\nLine 2\nLine 3";
    let original = Message::new(MessageRole::User, content.to_string());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.content, content);
}

#[test]
fn test_message_with_quotes() {
    let content = r#"He said "Hello" and she replied 'Hi'"#;
    let original = Message::new(MessageRole::User, content.to_string());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.content, content);
}

#[test]
fn test_message_with_json_content() {
    // Message containing JSON as content (common for tool calls)
    let content = r#"{"tool": "shell", "args": {"command": "ls -la"}}"#;
    let original = Message::new(MessageRole::Assistant, content.to_string());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.content, content);
}

// =============================================================================
// Test: MessageRole serialization
// =============================================================================

#[test]
fn test_message_role_user_serialization() {
    let role = MessageRole::User;
    let json = serde_json::to_string(&role).expect("Failed to serialize");
    
    // Should serialize to lowercase "user"
    assert!(json.to_lowercase().contains("user"));
    
    let restored: MessageRole = serde_json::from_str(&json).expect("Failed to deserialize");
    assert!(role_matches(&restored, "user"));
}

#[test]
fn test_message_role_assistant_serialization() {
    let role = MessageRole::Assistant;
    let json = serde_json::to_string(&role).expect("Failed to serialize");
    
    let restored: MessageRole = serde_json::from_str(&json).expect("Failed to deserialize");
    assert!(role_matches(&restored, "assistant"));
}

#[test]
fn test_message_role_system_serialization() {
    let role = MessageRole::System;
    let json = serde_json::to_string(&role).expect("Failed to serialize");
    
    let restored: MessageRole = serde_json::from_str(&json).expect("Failed to deserialize");
    assert!(role_matches(&restored, "system"));
}

// =============================================================================
// Test: Message with cache control
// =============================================================================

#[test]
fn test_message_with_ephemeral_cache() {
    let mut msg = Message::new(MessageRole::User, "Cached content".to_string());
    msg.cache_control = Some(CacheControl::ephemeral());
    
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert!(cache_control_is_ephemeral(&restored.cache_control));
}

#[test]
fn test_message_with_five_minute_cache() {
    let mut msg = Message::new(MessageRole::System, "System prompt".to_string());
    msg.cache_control = Some(CacheControl::five_minute());
    
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert!(cache_control_is_five_minute(&restored.cache_control));
}

#[test]
fn test_message_with_one_hour_cache() {
    let mut msg = Message::new(MessageRole::System, "Long-lived content".to_string());
    msg.cache_control = Some(CacheControl::one_hour());
    
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert!(cache_control_is_one_hour(&restored.cache_control));
}

#[test]
fn test_message_without_cache_control() {
    let msg = Message::new(MessageRole::User, "No cache".to_string());
    
    let json = serde_json::to_string(&msg).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert!(restored.cache_control.is_none());
}

// =============================================================================
// Test: Tool definition serialization
// =============================================================================

#[test]
fn test_tool_roundtrip() {
    let tool = Tool {
        name: "shell".to_string(),
        description: "Execute shell commands".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                }
            },
            "required": ["command"]
        }),
    };
    
    let json = serde_json::to_string(&tool).expect("Failed to serialize");
    let restored: Tool = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.name, "shell");
    assert_eq!(restored.description, "Execute shell commands");
    assert!(restored.input_schema.get("properties").is_some());
}

#[test]
fn test_tool_with_complex_schema() {
    let tool = Tool {
        name: "code_search".to_string(),
        description: "Search code using tree-sitter".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "searches": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "query": { "type": "string" },
                            "language": { "type": "string" }
                        },
                        "required": ["name", "query", "language"]
                    }
                }
            },
            "required": ["searches"]
        }),
    };
    
    let json = serde_json::to_string(&tool).expect("Failed to serialize");
    let restored: Tool = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.name, "code_search");
    
    // Verify nested schema structure is preserved
    let searches = restored.input_schema
        .get("properties")
        .and_then(|p| p.get("searches"))
        .and_then(|s| s.get("items"));
    assert!(searches.is_some());
}

// =============================================================================
// Test: Empty and edge cases
// =============================================================================

#[test]
fn test_message_with_empty_content() {
    let original = Message::new(MessageRole::User, "".to_string());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.content, "");
}

#[test]
fn test_message_with_very_long_content() {
    // Simulate a large file content or long conversation
    let content = "x".repeat(100_000);
    let original = Message::new(MessageRole::Assistant, content.clone());
    
    let json = serde_json::to_string(&original).expect("Failed to serialize");
    let restored: Message = serde_json::from_str(&json).expect("Failed to deserialize");
    
    assert_eq!(restored.content.len(), 100_000);
}

// =============================================================================
// Test: Deserialization from external JSON format
// =============================================================================

#[test]
fn test_deserialize_from_api_format() {
    // Simulate JSON that might come from an API response
    let json = r#"{
        "role": "assistant",
        "content": "Here is my response."
    }"#;
    
    let msg: Message = serde_json::from_str(json).expect("Failed to deserialize");
    
    assert!(role_matches(&msg.role, "assistant"));
    assert_eq!(msg.content, "Here is my response.");
}

#[test]
fn test_deserialize_user_message_from_api() {
    let json = r#"{"role": "user", "content": "What is 2+2?"}"#;
    
    let msg: Message = serde_json::from_str(json).expect("Failed to deserialize");
    
    assert!(role_matches(&msg.role, "user"));
    assert_eq!(msg.content, "What is 2+2?");
}
