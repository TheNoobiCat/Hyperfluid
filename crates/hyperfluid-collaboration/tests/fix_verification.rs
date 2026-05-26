//! Integration tests for production-readiness fixes.
//!
//! - F-30: route_message() signature delegation (debug_assert at entry)
//! - F-65: message_id content-addressed integrity check
//!
//! Test names follow `fix_F{N}_{short_description}` convention per task requirements.

#![allow(non_snake_case)]

use hyperfluid_collaboration::inbox::{
    compute_message_id, InboxConfig, InboxDecision, InboxMessage, InboxRouter,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a valid inbox message with proper content-addressed message_id and
/// a non-empty dummy signature.
fn make_valid_msg(sender: u8, recipient: u8, body: &[u8], nonce: u64) -> InboxMessage {
    let sender_id = [sender; 32];
    let recipient_id = [recipient; 32];
    let topic_id = [0u8; 32];
    let body_vec = body.to_vec();
    let msg_id = compute_message_id(&sender_id, &recipient_id, &topic_id, &body_vec, nonce, 1000);
    InboxMessage {
        message_id: msg_id,
        sender_id,
        recipient_id,
        topic_id,
        body_bytes: body_vec,
        nonce,
        expires_at_height: 1000,
        signature: vec![1u8; 64],
    }
}

// ---------------------------------------------------------------------------
// F-30: route_message() delegates signature verification to caller
// ---------------------------------------------------------------------------

#[test]
fn fix_F30_nonempty_signature_delivers() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let msg = make_valid_msg(1, 2, b"hello", 1);
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::Delivered);
}

#[test]
fn fix_F30_empty_signature_rejected() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let mut msg = make_valid_msg(1, 2, b"hello", 1);
    msg.signature = vec![];
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::InvalidContentAddressing);
}

// ---------------------------------------------------------------------------
// F-65: message_id content-addressed integrity verification
// ---------------------------------------------------------------------------

#[test]
fn fix_F65_valid_message_id_delivers() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let msg = make_valid_msg(1, 2, b"hello", 1);
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::Delivered);
}

#[test]
fn fix_F65_forged_message_id_rejected() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let mut msg = make_valid_msg(1, 2, b"hello", 1);
    // Tamper with message_id without recomputing
    msg.message_id = [0xFFu8; 32];
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::InvalidContentAddressing);
}

#[test]
fn fix_F65_wrong_sender_id_detected() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let mut msg = make_valid_msg(1, 2, b"hello", 1);
    // Change sender_id without updating message_id
    msg.sender_id = [3u8; 32];
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::InvalidContentAddressing);
}

#[test]
fn fix_F65_wrong_body_detected() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let mut msg = make_valid_msg(1, 2, b"hello", 1);
    // Change body without updating message_id
    msg.body_bytes = b"tampered".to_vec();
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::InvalidContentAddressing);
}

#[test]
fn fix_F65_wrong_nonce_detected() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let mut msg = make_valid_msg(1, 2, b"hello", 1);
    // Change nonce without updating message_id
    msg.nonce = 999;
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::InvalidContentAddressing);
}

#[test]
fn fix_F65_wrong_expiry_detected() {
    let mut router = InboxRouter::new(InboxConfig::default(), 600);
    router.advance_height(50);
    let mut msg = make_valid_msg(1, 2, b"hello", 1);
    // Change expires_at_height without updating message_id
    msg.expires_at_height = 9999;
    let result = router.route_message(msg);
    assert_eq!(result, InboxDecision::InvalidContentAddressing);
}
