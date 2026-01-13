//! G3 Ensembles - Multi-agent ensemble functionality
//!
//! This crate provides functionality for running multiple G3 agents in coordination,
//! enabling parallel development across different architectural modules.
//!
//! ✨💖 Extended for GB with persona-based segment assignment 💖✨

pub mod dialogue;
pub mod flock;
pub mod status;
mod tests;

/// Re-export main types for convenience
pub use dialogue::{
    DialogueMessage, DialogueMessageType, ReviewChain, get_dialogue_log_path,
    get_theatrical_dialogue, log_dialogue_message, read_dialogue_log,
};
pub use flock::{FlockConfig, FlockMode};
pub use status::{FlockStatus, SegmentPersonaAssignment, SegmentStatus};
