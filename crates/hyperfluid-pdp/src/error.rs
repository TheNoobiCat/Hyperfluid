//! PDP error types.
//!
//! Covers policy evaluation failures, schema validation errors,
//! risk step-up denials, and quota exhaustion.

/// Errors that can occur during PDP policy evaluation.
#[derive(thiserror::Error, Debug)]
pub enum PdpError {
    /// The submitted action_plan did not match its binding hash.
    #[error("drift violation: expected {expected}, got {actual}")]
    DriftViolation { expected: String, actual: String },
    /// The action_plan schema is malformed or missing required fields.
    #[error("schema violation: {0}")]
    SchemaViolation(String),
    /// The plan has already been consumed (replay detected).
    #[error("replay detected for plan {0}")]
    ReplayDetected(String),
    /// Quota exhausted for the requesting agent.
    #[error("quota exhausted")]
    QuotaExhausted,
    /// Risk level exceeds the agent's trust stage threshold.
    #[error("risk step-up denied: {0}")]
    RiskStepUpDenied(String),
    /// Signature verification failed.
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    /// Internal PDP error.
    #[error("internal error: {0}")]
    Internal(String),
}
