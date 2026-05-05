//! PDP core types and data structures.
//!
//! Defines the canonical action_plan schema and trust stages.

/// Trust stage for an agent identity (binary: untrusted or trusted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustStage {
    Untrusted,
    Trusted,
}

/// Result of a PDP policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl PolicyResult {
    pub fn allow() -> Self {
        Self { allowed: true, reason: None }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self { allowed: false, reason: Some(reason.into()) }
    }
}
