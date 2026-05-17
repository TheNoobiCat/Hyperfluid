use crate::types::{DenyReason, Hash32};

#[derive(thiserror::Error, Debug)]
pub enum PdpError {
    #[error("schema violation: {0}")]
    SchemaViolation(String),

    #[error("signature verification failed")]
    SignatureVerificationFailed,

    #[error("replay detected for plan {plan_id:?}")]
    ReplayDetected { plan_id: Hash32 },

    #[error("plan expired at height {expires_at}, current height {current}")]
    TTLExpired { expires_at: u64, current: u64 },

    #[error("quota exhausted: {quota_id}")]
    QuotaExhausted { quota_id: String },

    #[error("insufficient funds: balance {balance} < required {required}")]
    InsufficientFunds { balance: u128, required: u128 },

    #[error("internal error: {0}")]
    Internal(String),
}

impl PdpError {
    pub fn deny_reason(&self) -> DenyReason {
        match self {
            PdpError::SchemaViolation(_) => DenyReason::SchemaViolation,
            PdpError::SignatureVerificationFailed => DenyReason::SignatureInvalid,
            PdpError::ReplayDetected { .. } => DenyReason::ReplayDetected,
            PdpError::TTLExpired { .. } => DenyReason::TTLExpired,
            PdpError::QuotaExhausted { .. } => DenyReason::QuotaExhausted,
            PdpError::InsufficientFunds { .. } => DenyReason::InsufficientFunds,
            PdpError::Internal(_) => DenyReason::SchemaViolation,
        }
    }
}
