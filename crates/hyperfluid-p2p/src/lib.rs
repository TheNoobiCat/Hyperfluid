//! C7 P2P Networking & Connection Manager
//!
//! Direct-first routing, relay fallback, hybrid discovery (DHT + gossip),
//! secure channels, NAT traversal, connection state machine, mempool.
//!
//! Source: docs/04-specifications/protocol/p2p-wire-spec.md

pub mod discovery;
pub mod mempool;
pub mod transport;
pub mod types;

#[cfg(feature = "clatter-secure-channel")]
pub mod identity;

#[cfg(feature = "clatter-secure-channel")]
pub mod secure_channel;

pub use transport::{CachedPeer, PeerCache};

// Clatter (real cryptography) is the default.
// Mock (XOR cipher) is opt-in via `mock-secure-channel` feature flag.
#[cfg(feature = "clatter-secure-channel")]
pub use secure_channel::ClatterSecureChannel as SecureChannel;

#[cfg(not(feature = "clatter-secure-channel"))]
pub use transport::MockSecureChannel as SecureChannel;

#[cfg(feature = "clatter-secure-channel")]
pub use secure_channel::ClatterHandshake;

pub use types::{
    BootstrapResponse, CapabilityFlags, ConnState, ConnectionState, DHTEntry, DiscoveryConfig,
    GossipBloomFilter, GossipMessage, Hash32, PeerId, PeerInfo, TrustPolicy,
};
