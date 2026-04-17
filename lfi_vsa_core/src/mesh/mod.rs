//! # Purpose
//! Supersociety mesh networking — libp2p-based peer discovery, knowledge
//! exchange, and distributed consensus for PlausiDen nodes.
//!
//! # Design Decisions
//! - libp2p with QUIC transport + Noise encryption + Yamux multiplexing
//! - GossipSub for knowledge broadcast (HDC vector summaries)
//! - Kademlia DHT for peer discovery beyond local network
//! - mDNS for local network auto-discovery
//! - PeerID derived from Ed25519 — compatible with node identity design
//! - CRDT-based consensus via hdc/crdt.rs PN-counters (not naive bundling)
//!
//! # Invariants
//! - All mesh traffic is encrypted (Noise protocol)
//! - Knowledge exchange uses staging table — never writes directly to live
//! - Trust propagation via EigenTrust over source reputation
//!
//! # Failure Modes
//! - Network partition: nodes diverge but converge on reconnection (CRDT)
//! - Malicious peer: bounded influence via trimmed-mean aggregation
//! - NAT traversal: fallback to relay if hole-punching fails

pub mod node;
