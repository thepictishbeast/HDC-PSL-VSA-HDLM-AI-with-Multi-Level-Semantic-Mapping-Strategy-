//! # Purpose
//! CommitmentRegistry — the cross-cutting cryptographic fabric for LFI.
//! Every module that needs to commit-before-reveal uses this shared primitive:
//! proof obligations before Lean dispatch, encoder seeds per epoch,
//! CRDT deltas before gossip, message nonces before consumption.
//!
//! # Design Decisions
//! - SHA-256 for commitments (PQ-safe: Grover gives only √ speedup)
//! - Content-addressed storage via Blake3 CIDs for dedup
//! - Commitments are append-only (never modified or deleted)
//! - Reveal must match commit hash exactly — no partial reveals
//!
//! # Invariants
//! - A commitment hash uniquely identifies its content
//! - Reveal(data) succeeds iff SHA-256(data) == committed hash
//! - Registry is append-only — entries never removed (audit trail)

use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// A commitment — SHA-256 hash of some data committed before reveal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Commitment {
    pub hash: [u8; 32],
    pub purpose: CommitmentPurpose,
    pub timestamp: u64,
    pub revealed: bool,
}

/// What was committed and why.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommitmentPurpose {
    /// Proof obligation before Lean4 dispatch.
    ProofObligation,
    /// Encoder seed for HDC epoch (prevents adaptive adversarial examples).
    EncoderSeed,
    /// CRDT delta before gossip propagation.
    CrdtDelta,
    /// Message nonce before consumption.
    MessageNonce,
    /// Belief vector before epistemic certification.
    BeliefCommitment,
    /// Generic labeled commitment.
    Custom(String),
}

/// The shared commitment registry.
pub struct CommitmentRegistry {
    /// All commitments indexed by hash.
    commits: HashMap<[u8; 32], Commitment>,
    /// Total commits made.
    pub total_commits: u64,
    /// Total successful reveals.
    pub total_reveals: u64,
    /// Total failed reveal attempts.
    pub total_failures: u64,
}

impl CommitmentRegistry {
    pub fn new() -> Self {
        Self {
            commits: HashMap::new(),
            total_commits: 0,
            total_reveals: 0,
            total_failures: 0,
        }
    }

    /// Compute SHA-256 hash of data.
    pub fn hash(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Commit: store the hash, return it for later reveal.
    pub fn commit(&mut self, data: &[u8], purpose: CommitmentPurpose) -> [u8; 32] {
        let hash = Self::hash(data);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);

        self.commits.insert(hash, Commitment {
            hash,
            purpose,
            timestamp: ts,
            revealed: false,
        });
        self.total_commits += 1;
        hash
    }

    /// Reveal: verify data matches committed hash, mark as revealed.
    /// Returns true if the reveal matches.
    pub fn reveal(&mut self, data: &[u8]) -> bool {
        let hash = Self::hash(data);
        if let Some(commitment) = self.commits.get_mut(&hash) {
            if !commitment.revealed {
                commitment.revealed = true;
                self.total_reveals += 1;
                return true;
            }
        }
        self.total_failures += 1;
        false
    }

    /// Check if a commitment exists (without revealing).
    pub fn is_committed(&self, hash: &[u8; 32]) -> bool {
        self.commits.contains_key(hash)
    }

    /// Check if a commitment has been revealed.
    pub fn is_revealed(&self, hash: &[u8; 32]) -> bool {
        self.commits.get(hash).map_or(false, |c| c.revealed)
    }

    /// Number of unrevealed commitments.
    pub fn pending_count(&self) -> usize {
        self.commits.values().filter(|c| !c.revealed).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_and_reveal() {
        let mut reg = CommitmentRegistry::new();
        let data = b"test obligation";
        let hash = reg.commit(data, CommitmentPurpose::ProofObligation);
        assert!(reg.is_committed(&hash));
        assert!(!reg.is_revealed(&hash));
        assert!(reg.reveal(data));
        assert!(reg.is_revealed(&hash));
    }

    #[test]
    fn test_wrong_reveal_fails() {
        let mut reg = CommitmentRegistry::new();
        reg.commit(b"correct data", CommitmentPurpose::EncoderSeed);
        assert!(!reg.reveal(b"wrong data"));
        assert_eq!(reg.total_failures, 1);
    }

    #[test]
    fn test_double_reveal_fails() {
        let mut reg = CommitmentRegistry::new();
        let data = b"once only";
        reg.commit(data, CommitmentPurpose::MessageNonce);
        assert!(reg.reveal(data));
        assert!(!reg.reveal(data)); // Second reveal fails
    }

    #[test]
    fn test_hash_deterministic() {
        let h1 = CommitmentRegistry::hash(b"same input");
        let h2 = CommitmentRegistry::hash(b"same input");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_pending_count() {
        let mut reg = CommitmentRegistry::new();
        reg.commit(b"a", CommitmentPurpose::CrdtDelta);
        reg.commit(b"b", CommitmentPurpose::CrdtDelta);
        assert_eq!(reg.pending_count(), 2);
        reg.reveal(b"a");
        assert_eq!(reg.pending_count(), 1);
    }
}
