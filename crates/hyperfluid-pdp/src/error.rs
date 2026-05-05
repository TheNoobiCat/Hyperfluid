//! PDP error types.
//!
//! Covers policy evaluation failures: schema validation, replay, quota, signature.

/// Errors that can occur during PDP policy evaluation.
#[derive(thiserror::Error, Debug)]
pub enum PdpError {
    /// The action_plan schema is malformed or missing required fields.
    #[error("schema violation: {0}")]
    SchemaViolation(String),
    /// The plan has already been consumed (replay detected).
    #[error("replay detected for plan {0}")]
    ReplayDetected(String),
    /// Quota exhausted for the requesting agent.
    #[error("quota exhausted")]
    QuotaExhausted,
    /// Signature verification failed.
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    /// Internal PDP error.
    #[error("internal error: {0}")]
    Internal(String),
}
