//! # Purpose
//! PlausiDen mesh node — manages libp2p networking, peer discovery,
//! and knowledge exchange protocol.

use std::collections::HashSet;

/// Configuration for a mesh node.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Listen addresses (e.g., "/ip4/0.0.0.0/tcp/4001")
    pub listen_addrs: Vec<String>,
    /// Bootstrap peers to connect to on startup.
    pub bootstrap_peers: Vec<String>,
    /// GossipSub topic for knowledge exchange.
    pub knowledge_topic: String,
    /// Enable mDNS for local discovery.
    pub mdns_enabled: bool,
    /// Maximum peers to maintain.
    pub max_peers: usize,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            listen_addrs: vec!["/ip4/0.0.0.0/tcp/4001".into()],
            bootstrap_peers: Vec::new(),
            knowledge_topic: "plausiden/knowledge/v1".into(),
            mdns_enabled: true,
            max_peers: 50,
        }
    }
}

/// A mesh peer with trust score.
#[derive(Debug, Clone)]
pub struct MeshPeer {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub trust_score: f64,
    pub facts_exchanged: u64,
    pub last_seen: u64,
}

/// Messages exchanged over the mesh.
#[derive(Debug, Clone)]
pub enum MeshMessage {
    /// Offer knowledge summary (HDC vector digest).
    OfferKnowledge {
        domain: String,
        fact_count: u64,
        summary_hash: String,
    },
    /// Request specific facts by domain.
    RequestFacts {
        domain: String,
        max_count: u64,
    },
    /// Deliver facts (goes to staging, never live).
    DeliverFacts {
        domain: String,
        facts: Vec<(String, String, f64)>, // key, value, confidence
    },
    /// Heartbeat with node capabilities.
    Heartbeat {
        facts_total: u64,
        domains: Vec<String>,
        uptime_secs: u64,
    },
    /// Trust update from EigenTrust computation.
    TrustUpdate {
        peer_id: String,
        new_score: f64,
    },
}

/// The mesh node state (non-networking, for testing without libp2p runtime).
pub struct MeshNode {
    pub config: MeshConfig,
    pub peers: Vec<MeshPeer>,
    pub known_peers: HashSet<String>,
    pub messages_sent: u64,
    pub messages_received: u64,
}

impl MeshNode {
    pub fn new(config: MeshConfig) -> Self {
        Self {
            config,
            peers: Vec::new(),
            known_peers: HashSet::new(),
            messages_sent: 0,
            messages_received: 0,
        }
    }

    /// Add a discovered peer.
    pub fn add_peer(&mut self, peer: MeshPeer) {
        if self.known_peers.insert(peer.peer_id.clone()) {
            self.peers.push(peer);
        }
    }

    /// Get peers sorted by trust score (highest first).
    pub fn trusted_peers(&self) -> Vec<&MeshPeer> {
        let mut sorted: Vec<&MeshPeer> = self.peers.iter().collect();
        sorted.sort_by(|a, b| b.trust_score.partial_cmp(&a.trust_score).unwrap_or(std::cmp::Ordering::Equal));
        sorted
    }

    /// Process an incoming mesh message.
    pub fn handle_message(&mut self, from: &str, msg: MeshMessage) -> Option<MeshMessage> {
        self.messages_received += 1;
        match msg {
            MeshMessage::Heartbeat { facts_total, domains, .. } => {
                // Update peer info
                if let Some(peer) = self.peers.iter_mut().find(|p| p.peer_id == from) {
                    peer.last_seen = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs()).unwrap_or(0);
                }
                None // No response needed
            }
            MeshMessage::RequestFacts { domain, max_count } => {
                // Would query brain.db and return facts — stub for now
                Some(MeshMessage::DeliverFacts {
                    domain,
                    facts: Vec::new(), // Populated by caller with DB query
                })
            }
            MeshMessage::OfferKnowledge { domain, fact_count, .. } => {
                // Decide if we want these facts based on our gaps
                if fact_count > 0 {
                    Some(MeshMessage::RequestFacts { domain, max_count: fact_count.min(1000) })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_peer() {
        let mut node = MeshNode::new(MeshConfig::default());
        node.add_peer(MeshPeer {
            peer_id: "peer1".into(),
            addresses: vec!["/ip4/192.168.1.1/tcp/4001".into()],
            trust_score: 0.8,
            facts_exchanged: 0,
            last_seen: 0,
        });
        assert_eq!(node.peer_count(), 1);
        // Duplicate ignored
        node.add_peer(MeshPeer {
            peer_id: "peer1".into(),
            addresses: vec![],
            trust_score: 0.5,
            facts_exchanged: 0,
            last_seen: 0,
        });
        assert_eq!(node.peer_count(), 1);
    }

    #[test]
    fn test_trusted_peers_sorted() {
        let mut node = MeshNode::new(MeshConfig::default());
        node.add_peer(MeshPeer { peer_id: "low".into(), addresses: vec![], trust_score: 0.2, facts_exchanged: 0, last_seen: 0 });
        node.add_peer(MeshPeer { peer_id: "high".into(), addresses: vec![], trust_score: 0.9, facts_exchanged: 0, last_seen: 0 });
        node.add_peer(MeshPeer { peer_id: "mid".into(), addresses: vec![], trust_score: 0.5, facts_exchanged: 0, last_seen: 0 });
        let trusted = node.trusted_peers();
        assert_eq!(trusted[0].peer_id, "high");
        assert_eq!(trusted[2].peer_id, "low");
    }

    #[test]
    fn test_handle_heartbeat() {
        let mut node = MeshNode::new(MeshConfig::default());
        node.add_peer(MeshPeer { peer_id: "p1".into(), addresses: vec![], trust_score: 0.5, facts_exchanged: 0, last_seen: 0 });
        let resp = node.handle_message("p1", MeshMessage::Heartbeat {
            facts_total: 1000, domains: vec!["security".into()], uptime_secs: 3600,
        });
        assert!(resp.is_none());
        assert_eq!(node.messages_received, 1);
    }

    #[test]
    fn test_handle_offer_requests_facts() {
        let mut node = MeshNode::new(MeshConfig::default());
        let resp = node.handle_message("p1", MeshMessage::OfferKnowledge {
            domain: "security".into(), fact_count: 500, summary_hash: "abc".into(),
        });
        assert!(matches!(resp, Some(MeshMessage::RequestFacts { .. })));
    }

    #[test]
    fn test_default_config() {
        let cfg = MeshConfig::default();
        assert!(cfg.mdns_enabled);
        assert_eq!(cfg.max_peers, 50);
        assert!(cfg.knowledge_topic.contains("plausiden"));
    }
}
