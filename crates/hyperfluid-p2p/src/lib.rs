//! C7 P2P Networking & Connection Manager
//!
//! Direct-first routing, relay fallback, hybrid discovery (DHT + gossip),
//! secure channels, NAT traversal, connection state machine, mempool.
//!
//! Source: docs/04-specifications/protocol/p2p-wire-spec.md

pub mod discovery;
pub mod mempool;
pub mod tcp;
pub mod transport;
pub mod types;

pub mod identity;
pub mod secure_channel;

pub use transport::{CachedPeer, PeerCache};

pub use secure_channel::ClatterHandshake;
pub use secure_channel::ClatterSecureChannel as SecureChannel;

pub use types::{
    BootstrapResponse, CapabilityFlags, ConnState, ConnectionState, DHTEntry, DiscoveryConfig,
    GossipBloomFilter, GossipMessage, Hash32, PeerId, PeerInfo, SecureChannelError, TrustPolicy,
};
