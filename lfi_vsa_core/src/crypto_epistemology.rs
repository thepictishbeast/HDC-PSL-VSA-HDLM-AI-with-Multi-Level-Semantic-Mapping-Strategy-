// ============================================================
// Cryptographic Epistemology — Verifiable Belief Commitments
//
// PURPOSE: Allows the sovereign agent to commit to beliefs at time T
// and later prove what it believed without revealing belief contents
// until verification. This creates a tamper-evident epistemic log.
//
// SCHEME (Hash-based Commitment):
//   Commit phase:
//     1. Agent forms belief B (a VSA hypervector)
//     2. Agent generates random nonce R
//     3. Commitment C = SHA-256(B || R || timestamp)
//     4. Agent publishes C, keeps (B, R) private
//
//   Reveal phase:
//     1. Agent reveals (B, R, timestamp)
//     2. Verifier recomputes SHA-256(B || R || timestamp) and checks C == C'
//     3. If match: agent provably held belief B at the committed time
//
// WHY THIS MATTERS:
//   - Prevents post-hoc rationalization: the agent can't claim it
//     "always believed X" if its commitment says otherwise
//   - Enables trust auditing: third parties can verify epistemic honesty
//   - Creates an immutable belief timeline for debugging reasoning failures
//   - PSL integration: BeliefConsistencyAxiom can audit for contradictions
//
// VSA INTEGRATION:
//   Beliefs are HyperMemory vectors. The commitment hash binds the belief
//   to a specific point in time, creating a cryptographic "snapshot" of
//   the agent's epistemic state.
// ============================================================

use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use tracing::info;

/// A cryptographic commitment to a belief held at a specific time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefCommitment {
    /// SHA-256 hash of (belief_vector || nonce || timestamp)
    pub commitment_hash: [u8; 32],
    /// When the belief was committed (Unix epoch millis)
    pub timestamp: u64,
    /// Human-readable label for the belief (not part of the commitment)
    pub label: String,
    /// Whether this commitment has been revealed
    pub revealed: bool,
}

/// The private data needed to reveal a commitment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefWitness {
    /// The actual belief vector (serialized)
    pub belief_data: Vec<i8>,
    /// Random nonce used during commitment
    pub nonce: [u8; 32],
    /// Timestamp matching the commitment
    pub timestamp: u64,
}

/// Result of verifying a revealed belief against its commitment.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub commitment_hash: [u8; 32],
    pub recomputed_hash: [u8; 32],
    pub detail: String,
}

/// The Epistemic Ledger — maintains the agent's commitment history.
pub struct EpistemicLedger {
    /// Published commitments (public)
    pub commitments: Vec<BeliefCommitment>,
    /// Private witnesses (kept secret until reveal)
    witnesses: Vec<BeliefWitness>,
    /// Belief labels index for O(1) lookup by name
    label_index: std::collections::HashMap<String, usize>,
}

impl EpistemicLedger {
    pub fn new() -> Self {
        debuglog!("EpistemicLedger::new: Initializing sovereign epistemic ledger");
        Self {
            commitments: Vec::new(),
            witnesses: Vec::new(),
            label_index: std::collections::HashMap::new(),
        }
    }

    /// COMMIT: Record a belief commitment without revealing the belief.
    ///
    /// Returns the commitment index for later reference.
    pub fn commit_belief(&mut self, belief: &HyperMemory, label: &str) -> usize {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.commit_belief_at(belief, label, timestamp)
    }

    /// COMMIT with explicit timestamp (for testing and deterministic replay).
    pub fn commit_belief_at(&mut self, belief: &HyperMemory, label: &str, timestamp: u64) -> usize {
        // Generate cryptographic nonce
        let mut nonce = [0u8; 32];
        for byte in &mut nonce {
            *byte = rand::random::<u8>();
        }

        // Compute commitment hash: SHA-256(belief || nonce || timestamp)
        let commitment_hash = Self::compute_hash(&belief.vector.to_vec(), &nonce, timestamp);

        let idx = self.commitments.len();
        debuglog!("EpistemicLedger::commit: idx={}, label='{}', hash={}", idx, label, hex_str(&commitment_hash));

        self.commitments.push(BeliefCommitment {
            commitment_hash,
            timestamp,
            label: label.to_string(),
            revealed: false,
        });

        self.witnesses.push(BeliefWitness {
            belief_data: belief.vector.to_vec(),
            nonce,
            timestamp,
        });

        self.label_index.insert(label.to_string(), idx);

        info!("// AUDIT: Belief committed — label='{}', hash={}", label, hex_str(&commitment_hash));
        idx
    }

    /// REVEAL: Disclose a previously committed belief for verification.
    ///
    /// Returns cloned copies of the commitment and witness data.
    /// The commitment is marked as revealed in the ledger.
    pub fn reveal(&mut self, idx: usize) -> Option<(BeliefCommitment, BeliefWitness)> {
        if idx >= self.commitments.len() {
            debuglog!("EpistemicLedger::reveal: idx={} out of range", idx);
            return None;
        }
        self.commitments[idx].revealed = true;
        debuglog!("EpistemicLedger::reveal: idx={}, label='{}'", idx, self.commitments[idx].label);
        Some((self.commitments[idx].clone(), self.witnesses[idx].clone()))
    }

    /// REVEAL by label name.
    pub fn reveal_by_label(&mut self, label: &str) -> Option<(BeliefCommitment, BeliefWitness)> {
        let idx = *self.label_index.get(label)?;
        self.reveal(idx)
    }

    /// VERIFY: Check that a revealed belief matches its commitment.
    ///
    /// This is the critical verification step — it recomputes the hash
    /// from the witness data and checks against the published commitment.
    /// Static method — can be called by any third-party verifier.
    pub fn verify(commitment: &BeliefCommitment, witness: &BeliefWitness) -> VerificationResult {
        debuglog!("EpistemicLedger::verify: Recomputing commitment hash...");

        let recomputed = Self::compute_hash(&witness.belief_data, &witness.nonce, witness.timestamp);
        let valid = recomputed == commitment.commitment_hash;

        debuglog!("EpistemicLedger::verify: original={}, recomputed={}, valid={}",
            hex_str(&commitment.commitment_hash), hex_str(&recomputed), valid);

        VerificationResult {
            valid,
            commitment_hash: commitment.commitment_hash,
            recomputed_hash: recomputed,
            detail: if valid {
                format!("Belief '{}' verified — agent held this belief at t={}", commitment.label, commitment.timestamp)
            } else {
                format!("VERIFICATION FAILED — commitment tampered or witness corrupted for '{}'", commitment.label)
            },
        }
    }

    /// Retrieve the belief vector from a witness (for analysis after reveal).
    pub fn reconstruct_belief(witness: &BeliefWitness) -> HyperMemory {
        let dim = witness.belief_data.len();
        debuglog!("EpistemicLedger::reconstruct_belief: dim={}", dim);
        let vector = ndarray::Array1::from_vec(witness.belief_data.clone());
        HyperMemory { vector, dimensions: dim }
    }

    /// Check if two committed beliefs (after reveal) are consistent.
    /// Returns the similarity between the two belief vectors.
    pub fn check_consistency(witness_a: &BeliefWitness, witness_b: &BeliefWitness) -> f64 {
        let belief_a = Self::reconstruct_belief(witness_a);
        let belief_b = Self::reconstruct_belief(witness_b);
        let sim = belief_a.similarity(&belief_b);
        debuglog!("EpistemicLedger::consistency: similarity={:.4}", sim);
        sim
    }

    /// Number of commitments in the ledger.
    pub fn commitment_count(&self) -> usize {
        self.commitments.len()
    }

    /// Number of commitments that have been revealed.
    pub fn revealed_count(&self) -> usize {
        self.commitments.iter().filter(|c| c.revealed).count()
    }

    // Internal: compute the commitment hash
    fn compute_hash(belief_data: &[i8], nonce: &[u8; 32], timestamp: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Feed belief vector bytes
        for &v in belief_data {
            hasher.update(&[v as u8]);
        }

        // Feed nonce
        hasher.update(nonce);

        // Feed timestamp
        hasher.update(&timestamp.to_le_bytes());

        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Helper: format a hash as a short hex string for logging.
fn hex_str(bytes: &[u8; 32]) -> String {
    format!("{:02x}{:02x}..{:02x}{:02x}", bytes[0], bytes[1], bytes[30], bytes[31])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_and_verify() {
        let mut ledger = EpistemicLedger::new();
        let belief = HyperMemory::generate_seed(DIM_PROLETARIAT);

        let idx = ledger.commit_belief(&belief, "test_belief");
        assert_eq!(ledger.commitment_count(), 1);
        assert_eq!(ledger.revealed_count(), 0);

        let (commitment, witness) = ledger.reveal(idx).expect("reveal should succeed");
        assert!(commitment.revealed);
        assert_eq!(ledger.revealed_count(), 1);

        let result = EpistemicLedger::verify(&commitment, &witness);
        assert!(result.valid, "Commitment should verify: {}", result.detail);
    }

    #[test]
    fn test_tampered_witness_fails_verification() {
        let mut ledger = EpistemicLedger::new();
        let belief = HyperMemory::generate_seed(DIM_PROLETARIAT);

        let idx = ledger.commit_belief(&belief, "honest_belief");
        let (commitment, witness) = ledger.reveal(idx).expect("reveal should succeed");

        // Tamper: modify the witness nonce
        let mut tampered_witness = witness.clone();
        tampered_witness.nonce[0] ^= 0xFF;

        let result = EpistemicLedger::verify(&commitment, &tampered_witness);
        assert!(!result.valid, "Tampered witness should fail verification");
    }

    #[test]
    fn test_different_beliefs_different_commitments() {
        let mut ledger = EpistemicLedger::new();
        let belief_a = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let belief_b = HyperMemory::generate_seed(DIM_PROLETARIAT);

        let idx_a = ledger.commit_belief(&belief_a, "belief_a");
        let idx_b = ledger.commit_belief(&belief_b, "belief_b");

        assert_ne!(
            ledger.commitments[idx_a].commitment_hash,
            ledger.commitments[idx_b].commitment_hash,
            "Different beliefs should produce different commitments"
        );
    }

    #[test]
    fn test_reveal_by_label() {
        let mut ledger = EpistemicLedger::new();
        let belief = HyperMemory::generate_seed(DIM_PROLETARIAT);

        let _ = ledger.commit_belief(&belief, "labeled_belief");
        let result = ledger.reveal_by_label("labeled_belief");
        assert!(result.is_some(), "Should find commitment by label");

        let missing = ledger.reveal_by_label("nonexistent");
        assert!(missing.is_none(), "Missing label should return None");
    }

    #[test]
    fn test_belief_reconstruction() {
        let mut ledger = EpistemicLedger::new();
        let original = HyperMemory::generate_seed(DIM_PROLETARIAT);

        let idx = ledger.commit_belief(&original, "reconstruct_test");
        let (_, witness) = ledger.reveal(idx).expect("reveal should succeed");

        let reconstructed = EpistemicLedger::reconstruct_belief(&witness);
        let similarity = original.similarity(&reconstructed);
        assert!((similarity - 1.0).abs() < 0.001,
            "Reconstructed belief should be identical to original (sim={:.4})", similarity);
    }

    #[test]
    fn test_consistency_check() {
        let mut ledger = EpistemicLedger::new();

        // Two identical beliefs should have high consistency
        let base = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let similar = base.clone();

        let idx_a = ledger.commit_belief(&base, "base");
        let idx_b = ledger.commit_belief(&similar, "similar");

        let (_, witness_a) = ledger.reveal(idx_a).unwrap();
        let (_, witness_b) = ledger.reveal(idx_b).unwrap();

        let consistency = EpistemicLedger::check_consistency(&witness_a, &witness_b);
        assert!((consistency - 1.0).abs() < 0.001,
            "Identical beliefs should have consistency=1.0 (got {:.4})", consistency);
    }

    #[test]
    fn test_deterministic_timestamp_commit() {
        let mut ledger = EpistemicLedger::new();
        let belief = HyperMemory::from_string("deterministic", DIM_PROLETARIAT);

        let idx = ledger.commit_belief_at(&belief, "fixed_time", 1234567890);
        assert_eq!(ledger.commitments[idx].timestamp, 1234567890);

        let (commitment, witness) = ledger.reveal(idx).unwrap();
        let result = EpistemicLedger::verify(&commitment, &witness);
        assert!(result.valid, "Fixed-timestamp commitment should verify");
    }
}
