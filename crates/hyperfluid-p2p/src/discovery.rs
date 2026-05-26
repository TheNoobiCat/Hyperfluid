use crate::types::{ConnState, ConnectionState, DiscoveryConfig, Hash32};

/// Outcome of a direct connection probe.
pub enum ProbeOutcome {
    Success { endpoint: String, height: u64 },
    Timeout,
    Refused,
}

/// Transition the connection state machine based on a probe event.
///
/// Source: p2p-wire-spec.md Section 1.4 — Connection state machine.
///
/// Returns the new state the connection should enter.
pub fn transition_connection(
    current: &ConnectionState,
    event: ConnectionEvent,
    config: &DiscoveryConfig,
) -> ConnState {
    let cf = current.consecutive_failures;
    match (&current.state, event) {
        (ConnState::Unknown, ConnectionEvent::ProbeInitiated) => ConnState::DirectProbing,

        (ConnState::DirectProbing, ConnectionEvent::DirectConnectSuccess) => {
            ConnState::DirectActive
        }

        (ConnState::DirectProbing, ConnectionEvent::DirectConnectTimeout) => {
            if cf + 1 >= config.direct_retry_attempts {
                ConnState::RelayActive
            } else {
                ConnState::DirectProbing
            }
        }
        (ConnState::DirectProbing, ConnectionEvent::DirectConnectRefused) => {
            if cf + 1 >= config.direct_retry_attempts {
                ConnState::RelayActive
            } else {
                ConnState::DirectProbing
            }
        }

        (ConnState::DirectActive, ConnectionEvent::UpgradeProbeSucceeded) => ConnState::Upgrading,

        (ConnState::DirectActive, ConnectionEvent::ConnectionLost) => ConnState::Unknown,

        (ConnState::RelayActive, ConnectionEvent::UpgradeProbeSucceeded) => ConnState::Upgrading,

        (ConnState::RelayActive, ConnectionEvent::AllRelayPathsLost) => ConnState::Unknown,

        (ConnState::Upgrading, ConnectionEvent::MigrationComplete) => ConnState::DirectActive,

        (ConnState::Upgrading, ConnectionEvent::ConnectionLost) => ConnState::Unknown,

        // ── Explicit no-op arms ──────────────────────────────────────
        // Every known (state, event) combination that is not a valid
        // transition stays in the current state and logs a warning.
        // Unknown combinations are caught here rather than silently ignored.
        (ConnState::Unknown, ConnectionEvent::DirectConnectSuccess)
        | (ConnState::Unknown, ConnectionEvent::DirectConnectTimeout)
        | (ConnState::Unknown, ConnectionEvent::DirectConnectRefused)
        | (ConnState::Unknown, ConnectionEvent::UpgradeProbeSucceeded)
        | (ConnState::Unknown, ConnectionEvent::ConnectionLost)
        | (ConnState::Unknown, ConnectionEvent::MigrationComplete)
        | (ConnState::Unknown, ConnectionEvent::AllRelayPathsLost) => {
            eprintln!(
                "[p2p] Warning: invalid transition {:?} from Unknown — staying in Unknown",
                event
            );
            ConnState::Unknown
        }

        (ConnState::DirectProbing, ConnectionEvent::ProbeInitiated)
        | (ConnState::DirectProbing, ConnectionEvent::UpgradeProbeSucceeded)
        | (ConnState::DirectProbing, ConnectionEvent::ConnectionLost)
        | (ConnState::DirectProbing, ConnectionEvent::MigrationComplete)
        | (ConnState::DirectProbing, ConnectionEvent::AllRelayPathsLost) => {
            eprintln!(
                "[p2p] Warning: invalid transition {:?} from DirectProbing — staying in DirectProbing",
                event
            );
            ConnState::DirectProbing
        }

        (ConnState::DirectActive, ConnectionEvent::ProbeInitiated)
        | (ConnState::DirectActive, ConnectionEvent::DirectConnectSuccess)
        | (ConnState::DirectActive, ConnectionEvent::DirectConnectTimeout)
        | (ConnState::DirectActive, ConnectionEvent::DirectConnectRefused)
        | (ConnState::DirectActive, ConnectionEvent::MigrationComplete)
        | (ConnState::DirectActive, ConnectionEvent::AllRelayPathsLost) => {
            eprintln!(
                "[p2p] Warning: invalid transition {:?} from DirectActive — staying in DirectActive",
                event
            );
            ConnState::DirectActive
        }

        (ConnState::RelayActive, ConnectionEvent::ProbeInitiated)
        | (ConnState::RelayActive, ConnectionEvent::DirectConnectSuccess)
        | (ConnState::RelayActive, ConnectionEvent::DirectConnectTimeout)
        | (ConnState::RelayActive, ConnectionEvent::DirectConnectRefused)
        | (ConnState::RelayActive, ConnectionEvent::ConnectionLost)
        | (ConnState::RelayActive, ConnectionEvent::MigrationComplete) => {
            eprintln!(
                "[p2p] Warning: invalid transition {:?} from RelayActive — staying in RelayActive",
                event
            );
            ConnState::RelayActive
        }

        (ConnState::Upgrading, ConnectionEvent::ProbeInitiated)
        | (ConnState::Upgrading, ConnectionEvent::DirectConnectSuccess)
        | (ConnState::Upgrading, ConnectionEvent::DirectConnectTimeout)
        | (ConnState::Upgrading, ConnectionEvent::DirectConnectRefused)
        | (ConnState::Upgrading, ConnectionEvent::UpgradeProbeSucceeded)
        | (ConnState::Upgrading, ConnectionEvent::AllRelayPathsLost) => {
            eprintln!(
                "[p2p] Warning: invalid transition {:?} from Upgrading — staying in Upgrading",
                event
            );
            ConnState::Upgrading
        }
    }
}

/// Events that drive the connection state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionEvent {
    ProbeInitiated,
    DirectConnectSuccess,
    DirectConnectTimeout,
    DirectConnectRefused,
    UpgradeProbeSucceeded,
    ConnectionLost,
    MigrationComplete,
    AllRelayPathsLost,
}

/// Determine whether it is time to fire an upgrade probe from a relay path.
///
/// Returns true if the elapsed seconds since last probe exceeds
/// `upgrade_probe_secs` minus jitter. Jitter is ±`jitter_pct`%.
pub fn should_upgrade_probe(
    seconds_since_last_probe: u64,
    current_height: u64,
    last_probe_height: u64,
    config: &DiscoveryConfig,
) -> bool {
    let blocks_since = current_height.saturating_sub(last_probe_height);
    let base_interval = config.upgrade_probe_secs;
    let jitter_range = (base_interval * config.upgrade_probe_jitter_pct as u64) / 100;
    let min_interval = base_interval.saturating_sub(jitter_range);
    seconds_since_last_probe >= min_interval || blocks_since == 0
}

/// Check if a gossip message should be propagated:
/// TTL > 0 and fanout > 0.
pub fn should_propagate_gossip(ttl: u8, fanout: u8, config: &DiscoveryConfig) -> bool {
    ttl > 0 && fanout > 0 && ttl <= config.gossip_ttl && fanout <= config.gossip_fanout
}

/// Decrement TTL for gossip forwarding.
pub fn decrement_ttl(ttl: u8) -> Option<u8> {
    if ttl > 0 {
        Some(ttl - 1)
    } else {
        None
    }
}

/// Derive a peer's Kademlia DHT key from its identity pubkey hash.
pub fn dht_key(identity_pubkey: &[u8; 32]) -> Hash32 {
    crate::types::hash_bytes(identity_pubkey)
}

/// Determine the number of retries remaining before relay fallback.
pub fn retries_remaining(failures: u32, config: &DiscoveryConfig) -> u32 {
    config.direct_retry_attempts.saturating_sub(failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConnectionState;

    fn test_conn(state: ConnState, failures: u32) -> ConnectionState {
        ConnectionState {
            peer_id: [0u8; 32],
            state,
            direct_endpoint: None,
            relay_path: None,
            last_probe_height: 0,
            consecutive_failures: failures,
        }
    }

    #[test]
    fn unknown_to_direct_probing_on_probe() {
        let conn = test_conn(ConnState::Unknown, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::ProbeInitiated, &config);
        assert_eq!(next, ConnState::DirectProbing);
    }

    #[test]
    fn direct_probing_to_active_on_success() {
        let conn = test_conn(ConnState::DirectProbing, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::DirectConnectSuccess, &config);
        assert_eq!(next, ConnState::DirectActive);
    }

    #[test]
    fn direct_probing_to_relay_after_max_failures() {
        let conn = test_conn(ConnState::DirectProbing, 2);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::DirectConnectTimeout, &config);
        assert_eq!(next, ConnState::RelayActive);
    }

    #[test]
    fn direct_probing_retries_before_max() {
        let conn = test_conn(ConnState::DirectProbing, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::DirectConnectTimeout, &config);
        assert_eq!(next, ConnState::DirectProbing);
    }

    #[test]
    fn direct_active_to_upgrading_on_upgrade_probe() {
        let conn = test_conn(ConnState::DirectActive, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::UpgradeProbeSucceeded, &config);
        assert_eq!(next, ConnState::Upgrading);
    }

    #[test]
    fn direct_active_to_unknown_on_loss() {
        let conn = test_conn(ConnState::DirectActive, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::ConnectionLost, &config);
        assert_eq!(next, ConnState::Unknown);
    }

    #[test]
    fn relay_active_to_upgrading_on_upgrade_probe() {
        let conn = test_conn(ConnState::RelayActive, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::UpgradeProbeSucceeded, &config);
        assert_eq!(next, ConnState::Upgrading);
    }

    #[test]
    fn relay_active_to_unknown_when_all_relays_lost() {
        let conn = test_conn(ConnState::RelayActive, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::AllRelayPathsLost, &config);
        assert_eq!(next, ConnState::Unknown);
    }

    #[test]
    fn upgrading_to_direct_active_on_migration() {
        let conn = test_conn(ConnState::Upgrading, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::MigrationComplete, &config);
        assert_eq!(next, ConnState::DirectActive);
    }

    #[test]
    fn no_transition_on_invalid_event_for_state() {
        let conn = test_conn(ConnState::DirectActive, 0);
        let config = DiscoveryConfig::default();
        let next = transition_connection(&conn, ConnectionEvent::ProbeInitiated, &config);
        assert_eq!(next, ConnState::DirectActive);
    }

    #[test]
    fn should_propagate_gossip_with_valid_ttl_and_fanout() {
        let config = DiscoveryConfig::default();
        assert!(should_propagate_gossip(10, 4, &config));
        assert!(!should_propagate_gossip(0, 4, &config));
        assert!(!should_propagate_gossip(10, 0, &config));
    }

    #[test]
    fn should_propagate_gossip_exceeds_max_ttl() {
        let config = DiscoveryConfig::default();
        assert!(!should_propagate_gossip(17, 4, &config));
    }

    #[test]
    fn should_propagate_gossip_exceeds_max_fanout() {
        let config = DiscoveryConfig::default();
        assert!(!should_propagate_gossip(10, 9, &config));
    }

    #[test]
    fn decrement_ttl_works() {
        assert_eq!(decrement_ttl(5), Some(4));
        assert_eq!(decrement_ttl(1), Some(0));
        assert_eq!(decrement_ttl(0), None);
    }

    #[test]
    fn retries_remaining_works() {
        let config = DiscoveryConfig { direct_retry_attempts: 5, ..Default::default() };
        assert_eq!(retries_remaining(0, &config), 5);
        assert_eq!(retries_remaining(3, &config), 2);
        assert_eq!(retries_remaining(6, &config), 0);
    }

    #[test]
    fn dht_key_is_deterministic() {
        let pk = [1u8; 32];
        let k1 = dht_key(&pk);
        let k2 = dht_key(&pk);
        assert_eq!(k1, k2);
    }
}
