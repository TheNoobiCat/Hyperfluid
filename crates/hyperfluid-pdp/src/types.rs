//! PDP core types and data structures.
//!
//! Defines the canonical action_plan schema, risk levels,
//! trust stages, and policy evaluation result types.

/// Trust ladder stage for an agent identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustStage {
    UntrustedJoiner,
    SandboxedContributor,
    TrustedContributor,
    CoordinatorEligible,
}

/// Risk level assigned to an action_plan.
/// Maps to spec's RiskClass (3 levels). Source: policy-engine-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Result of a PDP policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyResult {
    pub allowed: bool,
    pub risk_level: RiskLevel,
    pub reason: Option<String>,
}

impl PolicyResult {
    pub fn allow(risk_level: RiskLevel) -> Self {
        Self { allowed: true, risk_level, reason: None }
    }

    pub fn deny(risk_level: RiskLevel, reason: impl Into<String>) -> Self {
        Self { allowed: false, risk_level, reason: Some(reason.into()) }
    }
}
