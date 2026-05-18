use hyperfluid_state::Hash32;

/// Generate a deterministic freshness nonce bound to a task and block height.
/// Used for artifact replay prevention (FR-0175).
pub fn generate_nonce(task_id: Hash32, current_height: u64) -> Hash32 {
    hyperfluid_state::sha3_256(&[task_id.as_slice(), &current_height.to_le_bytes()].concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonce_is_deterministic() {
        let nonce1 = generate_nonce([1u8; 32], 100);
        let nonce2 = generate_nonce([1u8; 32], 100);
        assert_eq!(nonce1, nonce2);
    }

    #[test]
    fn nonce_changes_with_height() {
        let nonce1 = generate_nonce([1u8; 32], 100);
        let nonce2 = generate_nonce([1u8; 32], 101);
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn nonce_changes_with_task() {
        let nonce1 = generate_nonce([1u8; 32], 100);
        let nonce2 = generate_nonce([2u8; 32], 100);
        assert_ne!(nonce1, nonce2);
    }
}
