//! C7 P2P Networking & Connection Manager
//!
//! Direct-first routing, relay fallback, hybrid discovery (DHT + gossip),
//! secure channels, NAT traversal, connection state machine, mempool.
//!
//! Source: docs/04-specifications/protocol/p2p-wire-spec.md

pub mod discovery;
pub mod mempool;
pub mod types;

pub use types::{
    BootstrapResponse, CapabilityFlags, ConnectionState, DHTEntry, DiscoveryConfig,
    GossipBloomFilter, GossipMessage, Hash32, PeerInfo, TrustPolicy,
};
