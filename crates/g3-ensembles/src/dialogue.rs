//! 🎭✨ GB Inter-Agent Dialogue System ✨🎭
//!
//! Theatrical multi-agent communication with persona markers.
//! Enables Regina, Gretchen, Monica, and the crew to chat like they're
//! in a reality TV show about code review.
//!
//! ## Format
//!
//! ```text
//! 💅 GRETCHEN:
//! I implemented the feature! It's giving EFFICIENCY.
//!
//! 👑 REGINA:
//! *reviews*
//! This is from Stack Overflow. Posted by user "penguin2019." 47 downvotes.
//!
//! 🔒 DARIA:
//! The code doesn't even compile. I checked. While this conversation was happening.
//! ```
//!
//! See `docs/EXCHANGES.md` for full examples.

use chrono::{DateTime, Utc};
use gb_personas::{AgentRole, Persona};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write as IoWrite;
use std::path::PathBuf;
use tracing::debug;

// ═══════════════════════════════════════════════════════════════════════════════
// 💬 Dialogue Message Structures
// ═══════════════════════════════════════════════════════════════════════════════

/// Type of dialogue message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DialogueMessageType {
    /// Initial analysis or observation
    Analysis,
    /// Review of another agent's work
    Review,
    /// Response to a review
    Response,
    /// Final summary or conclusion
    Summary,
    /// General comment or remark
    Comment,
}

/// A single message in the inter-agent dialogue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueMessage {
    /// When the message was sent
    pub timestamp: DateTime<Utc>,
    /// Which persona sent this message
    pub persona: Persona,
    /// What role the agent was playing
    pub role: AgentRole,
    /// Agent identifier (e.g., "monica-segment-1")
    pub agent_id: String,
    /// The actual message content
    pub content: String,
    /// Type of message
    pub message_type: DialogueMessageType,
}

impl DialogueMessage {
    /// Create a new dialogue message
    pub fn new(
        persona: Persona,
        role: AgentRole,
        agent_id: impl Into<String>,
        content: impl Into<String>,
        message_type: DialogueMessageType,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            persona,
            role,
            agent_id: agent_id.into(),
            content: content.into(),
            message_type,
        }
    }

    /// Format this message in the theatrical EXCHANGES.md style
    pub fn format_theatrical(&self) -> String {
        use gb_personas::get_persona_data;

        let data = get_persona_data(self.persona);
        let emoji = data
            .emoji_favorites
            .first()
            .copied()
            .unwrap_or("💬");
        let name = data.display_name;

        // Format with persona marker
        let marker = format!("{} {}:", emoji, name);
        let mut lines: Vec<String> = self.content.lines().map(|l| l.to_string()).collect();

        if lines.is_empty() {
            return format!("{}\n(no content)\n", marker);
        }

        // First line gets the marker
        let first_line = format!("{}\n{}\n", marker, lines.remove(0));

        // Remaining lines are indented
        let remaining = if !lines.is_empty() {
            lines.join("\n") + "\n"
        } else {
            String::new()
        };

        format!("{}{}\n", first_line, remaining)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 📝 Dialogue Logging
// ═══════════════════════════════════════════════════════════════════════════════

/// Get the dialogue log file path for a session
pub fn get_dialogue_log_path(session_id: &str) -> PathBuf {
    let g3_dir = g3_core::paths::get_g3_dir();
    g3_dir
        .join("dialogues")
        .join(session_id)
        .join("dialogue.log")
}

/// Ensure the dialogue directory exists
fn ensure_dialogue_dir(session_id: &str) -> std::io::Result<PathBuf> {
    let dialogue_dir = g3_core::paths::get_g3_dir()
        .join("dialogues")
        .join(session_id);
    create_dir_all(&dialogue_dir)?;
    Ok(dialogue_dir)
}

/// Append a dialogue message to the session log
pub fn log_dialogue_message(session_id: &str, message: &DialogueMessage) -> std::io::Result<()> {
    ensure_dialogue_dir(session_id)?;

    let log_file = get_dialogue_log_path(session_id);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    // Write in theatrical format
    let formatted = message.format_theatrical();
    file.write_all(formatted.as_bytes())?;

    // Also log as JSON for machine parsing
    let json_log = get_dialogue_log_path(session_id).with_extension("jsonl");
    let mut json_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&json_log)?;

    if let Ok(json) = serde_json::to_string(message) {
        writeln!(json_file, "{}", json)?;
    }

    debug!(
        "Logged dialogue message from {} to {}",
        message.agent_id, log_file.display()
    );

    Ok(())
}

/// Read all dialogue messages for a session
pub fn read_dialogue_log(session_id: &str) -> std::io::Result<Vec<DialogueMessage>> {
    let json_log = get_dialogue_log_path(session_id).with_extension("jsonl");

    if !json_log.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&json_log)?;
    let messages: Vec<DialogueMessage> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    Ok(messages)
}

/// Get the theatrical dialogue log as a string
pub fn get_theatrical_dialogue(session_id: &str) -> std::io::Result<String> {
    let log_file = get_dialogue_log_path(session_id);

    if !log_file.exists() {
        return Ok(String::new());
    }

    std::fs::read_to_string(&log_file)
}

// ═══════════════════════════════════════════════════════════════════════════════
// 🔗 Review Chains
// ═══════════════════════════════════════════════════════════════════════════════

/// A review chain specifies the sequence of agents that review work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewChain {
    /// Name of the chain (e.g., "security-review", "full-review")
    pub name: String,
    /// Ordered list of (Persona, Role) pairs
    pub agents: Vec<(Persona, AgentRole)>,
}

impl ReviewChain {
    /// Create a standard security review chain: Daria (Security)
    pub fn security() -> Self {
        Self {
            name: "security".to_string(),
            agents: vec![(Persona::Daria, AgentRole::Security)],
        }
    }

    /// Create a full review chain: Monica → Rachel → Daria
    pub fn full() -> Self {
        Self {
            name: "full".to_string(),
            agents: vec![
                (Persona::Monica, AgentRole::Architect),
                (Persona::Rachel, AgentRole::Refactorer),
                (Persona::Daria, AgentRole::Security),
            ],
        }
    }

    /// Create a quick review chain: Rachel (Refactorer)
    pub fn quick() -> Self {
        Self {
            name: "quick".to_string(),
            agents: vec![(Persona::Rachel, AgentRole::Refactorer)],
        }
    }

    /// Create a debugging chain: Phoebe → Daria
    pub fn debugging() -> Self {
        Self {
            name: "debugging".to_string(),
            agents: vec![
                (Persona::Phoebe, AgentRole::Debugger),
                (Persona::Daria, AgentRole::Security),
            ],
        }
    }

    /// Parse a review chain from requirements text
    /// Looks for keywords like "security review", "full review", etc.
    pub fn from_requirements(requirements: &str) -> Option<Self> {
        let lower = requirements.to_lowercase();

        if lower.contains("full review") || lower.contains("comprehensive review") {
            Some(Self::full())
        } else if lower.contains("security review") || lower.contains("security audit") {
            Some(Self::security())
        } else if lower.contains("debug") || lower.contains("bug") {
            Some(Self::debugging())
        } else if lower.contains("quick review") || lower.contains("refactor") {
            Some(Self::quick())
        } else {
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 🧪 Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_message_creation() {
        let msg = DialogueMessage::new(
            Persona::Regina,
            AgentRole::Coach,
            "regina-test",
            "This code is NOT fetch.",
            DialogueMessageType::Review,
        );

        assert_eq!(msg.persona, Persona::Regina);
        assert_eq!(msg.role, AgentRole::Coach);
        assert_eq!(msg.agent_id, "regina-test");
        assert!(msg.content.contains("NOT fetch"));
    }

    #[test]
    fn test_theatrical_formatting() {
        let msg = DialogueMessage::new(
            Persona::Gretchen,
            AgentRole::Player,
            "gretchen-test",
            "I implemented the feature!\nIt's giving EFFICIENCY.",
            DialogueMessageType::Analysis,
        );

        let formatted = msg.format_theatrical();

        assert!(formatted.contains("💅 GRETCHEN:"));
        assert!(formatted.contains("I implemented the feature!"));
        assert!(formatted.contains("EFFICIENCY"));
    }

    #[test]
    fn test_review_chain_from_requirements() {
        let req1 = "Please do a full review of this module";
        let chain1 = ReviewChain::from_requirements(req1).unwrap();
        assert_eq!(chain1.name, "full");
        assert_eq!(chain1.agents.len(), 3);

        let req2 = "Run a security audit on the authentication code";
        let chain2 = ReviewChain::from_requirements(req2).unwrap();
        assert_eq!(chain2.name, "security");
        assert_eq!(chain2.agents[0].0, Persona::Daria);

        let req3 = "Just implement this feature";
        let chain3 = ReviewChain::from_requirements(req3);
        assert!(chain3.is_none());
    }

    #[test]
    fn test_dialogue_serialization() {
        let msg = DialogueMessage::new(
            Persona::Monica,
            AgentRole::Architect,
            "monica-test",
            "The folder structure is concerning.",
            DialogueMessageType::Analysis,
        );

        let json = serde_json::to_string(&msg).unwrap();
        let restored: DialogueMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.persona, Persona::Monica);
        assert_eq!(restored.agent_id, "monica-test");
    }
}
