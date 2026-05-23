//! C11 Collaboration & Inbox Layer
//!
//! Canonical type definitions live in `hyperfluid-state`. State machine logic lives
//! in `hyperfluid-state::state_machine`. This crate re-exports types and provides
//! pure helper functions for collaboration-layer collaboration logic.

pub mod inbox;
pub mod replay;

pub use hyperfluid_state::{
    EscrowStatus, HeartbeatPayload, Task, TaskLease, TaskStatus, TopicRecord, TopicStatus,
    TrustStageEnum, TrustStageRecord,
};
pub use inbox::{InboxConfig, InboxDecision, InboxMessage, InboxRouter};
pub use replay::generate_nonce;
