//! # Purpose
//! CRDT-safe hyperdimensional vector consensus for the Supersociety mesh.
//! Naive HDC bundling (sign-of-sum) is NOT associative and therefore NOT a CRDT:
//!   sign(sign(a+b)+c) ≠ sign(a+b+c)  — confirmed empirically at 100% failure rate.
//! This module implements a per-dimension PN-counter that IS a proper state-based
//! CRDT, giving eventual consistency on the sign readout across mesh nodes.
//!
//! # Design Decisions
//! - Each dimension tracks (+1 votes, -1 votes) as separate monotonic counters
//! - Join (merge) takes max of each counter per replica — standard PN-counter
//! - Readout: sign(positive_count - negative_count) gives the consensus bipolar value
//! - Delta-state gossip: only send changed dimensions (~3-4KB compressed for 10K-D)
//! - Compatible with existing BipolarVector via to_bipolar()/from_bipolar()
//!
//! # Invariants
//! - Positive and negative counters are monotonically non-decreasing per replica
//! - Join is commutative, associative, and idempotent (CRDT properties)
//! - Readout matches the true majority vote across all contributing nodes
//!
//! # Failure Modes
//! - Network partitions: replicas diverge but converge on reconnection (by design)
//! - Counter overflow: u32 per dimension per replica → 4B votes before overflow
//! - Stale replica: old node with outdated counters is harmless (join is monotonic)

use crate::hdc::vector::{BipolarVector, HD_DIMENSIONS};
use crate::hdc::error::HdcError;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// A replica identifier for the mesh.
pub type ReplicaId = u64;

/// Per-dimension PN-counter for CRDT-safe HDC consensus.
/// Each dimension independently tracks positive and negative votes
/// from each replica, ensuring associative, commutative, idempotent merge.
///
/// #346 serde derive: enables bincode-serialised gossip payloads over
/// the wire and `to_bytes`/`from_bytes` round-trips for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdcCrdt {
    /// Per-replica positive vote counts for each dimension.
    /// positive[replica_id][dim] = count of +1 votes from that replica.
    positive: HashMap<ReplicaId, Vec<u32>>,
    /// Per-replica negative vote counts.
    negative: HashMap<ReplicaId, Vec<u32>>,
    /// Dimensionality (must match BipolarVector).
    dim: usize,
}

impl HdcCrdt {
    /// Create a new empty CRDT state.
    pub fn new(dim: usize) -> Self {
        Self {
            positive: HashMap::new(),
            negative: HashMap::new(),
            dim,
        }
    }

    /// Create with standard HD_DIMENSIONS.
    pub fn standard() -> Self {
        Self::new(HD_DIMENSIONS)
    }

    /// Contribute a BipolarVector from a specific replica.
    /// Each +1 bit increments the positive counter for that dimension,
    /// each -1 (0 bit) increments the negative counter.
    pub fn contribute(&mut self, replica: ReplicaId, vector: &BipolarVector) -> Result<(), HdcError> {
        if vector.dim() != self.dim {
            return Err(HdcError::DimensionMismatch {
                expected: self.dim,
                actual: vector.dim(),
            });
        }

        let pos = self.positive.entry(replica).or_insert_with(|| vec![0u32; self.dim]);
        let neg = self.negative.entry(replica).or_insert_with(|| vec![0u32; self.dim]);

        for i in 0..self.dim {
            if vector.bits()[i] {
                pos[i] += 1; // +1 vote
            } else {
                neg[i] += 1; // -1 vote
            }
        }
        Ok(())
    }

    /// CRDT join (merge) — takes the max of each counter per replica.
    /// This is the core CRDT operation: commutative, associative, idempotent.
    pub fn join(&mut self, other: &HdcCrdt) -> Result<(), HdcError> {
        if other.dim != self.dim {
            return Err(HdcError::DimensionMismatch {
                expected: self.dim,
                actual: other.dim,
            });
        }

        // Merge positive counters
        for (replica, other_pos) in &other.positive {
            let pos = self.positive.entry(*replica)
                .or_insert_with(|| vec![0u32; self.dim]);
            for i in 0..self.dim {
                pos[i] = pos[i].max(other_pos[i]);
            }
        }

        // Merge negative counters
        for (replica, other_neg) in &other.negative {
            let neg = self.negative.entry(*replica)
                .or_insert_with(|| vec![0u32; self.dim]);
            for i in 0..self.dim {
                neg[i] = neg[i].max(other_neg[i]);
            }
        }

        Ok(())
    }

    /// Readout — compute the consensus BipolarVector from the CRDT state.
    /// For each dimension: total_pos - total_neg determines the bit.
    /// Ties break to -1 (bit=0), matching BipolarVector::bundle convention.
    pub fn readout(&self) -> BipolarVector {
        use bitvec::prelude::*;
        let mut data = BitVec::<u8, Lsb0>::with_capacity(self.dim);

        for i in 0..self.dim {
            let total_pos: u64 = self.positive.values()
                .map(|v| v.get(i).copied().unwrap_or(0) as u64)
                .sum();
            let total_neg: u64 = self.negative.values()
                .map(|v| v.get(i).copied().unwrap_or(0) as u64)
                .sum();

            // Strictly positive → +1 (bit=1), else -1 (bit=0)
            data.push(total_pos > total_neg);
        }

        BipolarVector { data }
    }

    /// Number of contributing replicas.
    pub fn replica_count(&self) -> usize {
        let mut replicas: std::collections::HashSet<ReplicaId> = self.positive.keys().copied().collect();
        replicas.extend(self.negative.keys());
        replicas.len()
    }

    /// Total votes across all dimensions and replicas.
    pub fn total_votes(&self) -> u64 {
        let pos: u64 = self.positive.values()
            .flat_map(|v| v.iter())
            .map(|&x| x as u64)
            .sum();
        let neg: u64 = self.negative.values()
            .flat_map(|v| v.iter())
            .map(|&x| x as u64)
            .sum();
        pos + neg
    }

    /// Generate a delta for gossip — only dimensions that changed since last sync.
    /// Returns (replica_id, dimension_index, positive_count, negative_count) tuples.
    pub fn delta_since(&self, replica: ReplicaId, last_pos: &[u32], last_neg: &[u32]) -> Vec<(usize, u32, u32)> {
        let mut deltas = Vec::new();
        let pos = self.positive.get(&replica).map(|v| v.as_slice()).unwrap_or(&[]);
        let neg = self.negative.get(&replica).map(|v| v.as_slice()).unwrap_or(&[]);

        for i in 0..self.dim {
            let p = pos.get(i).copied().unwrap_or(0);
            let n = neg.get(i).copied().unwrap_or(0);
            let lp = last_pos.get(i).copied().unwrap_or(0);
            let ln = last_neg.get(i).copied().unwrap_or(0);

            if p != lp || n != ln {
                deltas.push((i, p, n));
            }
        }
        deltas
    }

    /// #346 Apply a received delta from a peer replica. Monotonic: counts
    /// never decrease — if the peer's count is lower, we keep our own.
    /// Returns the number of dimensions actually updated.
    pub fn apply_delta(&mut self, replica: ReplicaId, delta: &CrdtDelta) -> usize {
        let pos = self.positive.entry(replica).or_insert_with(|| vec![0; self.dim]);
        let neg = self.negative.entry(replica).or_insert_with(|| vec![0; self.dim]);
        let mut changed = 0;
        for &(dim_idx, p, n) in &delta.entries {
            if dim_idx >= self.dim { continue; }
            let new_p = pos[dim_idx].max(p);
            let new_n = neg[dim_idx].max(n);
            if new_p != pos[dim_idx] || new_n != neg[dim_idx] {
                pos[dim_idx] = new_p;
                neg[dim_idx] = new_n;
                changed += 1;
            }
        }
        changed
    }

    /// Serialise the entire CRDT state for wire transport or persistence.
    /// Uses bincode for compactness (u32 counters pack to 4B each).
    pub fn to_bytes(&self) -> Result<Vec<u8>, HdcError> {
        bincode::serialize(self).map_err(|e| HdcError::PersistenceFailure { detail: e.to_string() })
    }

    /// Deserialise a CRDT state produced by `to_bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HdcError> {
        bincode::deserialize(bytes).map_err(|e| HdcError::PersistenceFailure { detail: e.to_string() })
    }

    /// Build a wire-format delta relative to a baseline snapshot.
    pub fn delta_payload(&self, replica: ReplicaId,
                         last_pos: &[u32], last_neg: &[u32]) -> CrdtDelta {
        CrdtDelta {
            replica,
            dim: self.dim,
            entries: self.delta_since(replica, last_pos, last_neg),
        }
    }
}

/// Wire-format delta for mesh gossip. Serde-serialisable so it can fly
/// over any byte-safe transport (HTTPS POST body, QUIC, WireGuard UDP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtDelta {
    pub replica: ReplicaId,
    pub dim: usize,
    /// (dimension_index, positive_count, negative_count) tuples.
    pub entries: Vec<(usize, u32, u32)>,
}

impl CrdtDelta {
    pub fn to_bytes(&self) -> Result<Vec<u8>, HdcError> {
        bincode::serialize(self).map_err(|e| HdcError::PersistenceFailure { detail: e.to_string() })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HdcError> {
        bincode::deserialize(bytes).map_err(|e| HdcError::PersistenceFailure { detail: e.to_string() })
    }

    pub fn entry_count(&self) -> usize { self.entries.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crdt_roundtrip_through_bytes() {
        let v = BipolarVector::from_seed(7);
        let mut crdt = HdcCrdt::standard();
        crdt.contribute(42, &v).unwrap();
        let bytes = crdt.to_bytes().unwrap();
        let restored = HdcCrdt::from_bytes(&bytes).unwrap();
        assert_eq!(restored.readout(), crdt.readout());
        assert_eq!(restored.replica_count(), crdt.replica_count());
    }

    #[test]
    fn test_crdt_apply_delta_matches_direct_contribute() {
        // Node A contributes directly; node B receives the delta and applies it.
        // Readouts must match.
        let v = BipolarVector::from_seed(99);
        let mut node_a = HdcCrdt::standard();
        node_a.contribute(1, &v).unwrap();

        let mut node_b = HdcCrdt::standard();
        let delta = node_a.delta_payload(1, &[], &[]);
        let updated = node_b.apply_delta(1, &delta);
        assert!(updated > 0, "delta should have non-zero updates");
        assert_eq!(node_a.readout(), node_b.readout());
    }

    #[test]
    fn test_crdt_delta_apply_is_idempotent() {
        // Applying the same delta twice must not change state further.
        let v = BipolarVector::from_seed(123);
        let mut source = HdcCrdt::standard();
        source.contribute(5, &v).unwrap();
        let delta = source.delta_payload(5, &[], &[]);

        let mut sink = HdcCrdt::standard();
        let first = sink.apply_delta(5, &delta);
        let readout1 = sink.readout();
        let second = sink.apply_delta(5, &delta);
        assert!(first > 0);
        assert_eq!(second, 0, "re-applying same delta must be a no-op");
        assert_eq!(sink.readout(), readout1);
    }

    #[test]
    fn test_crdt_delta_wire_roundtrip() {
        let v = BipolarVector::from_seed(4242);
        let mut node = HdcCrdt::standard();
        node.contribute(9, &v).unwrap();
        let delta = node.delta_payload(9, &[], &[]);
        let bytes = delta.to_bytes().unwrap();
        let restored = CrdtDelta::from_bytes(&bytes).unwrap();
        assert_eq!(restored.replica, 9);
        assert_eq!(restored.dim, HD_DIMENSIONS);
        assert_eq!(restored.entry_count(), delta.entry_count());
    }

    #[test]
    fn test_crdt_join_is_commutative() {
        let v1 = BipolarVector::from_seed(1);
        let v2 = BipolarVector::from_seed(2);

        let mut crdt_a = HdcCrdt::standard();
        crdt_a.contribute(1, &v1).unwrap();

        let mut crdt_b = HdcCrdt::standard();
        crdt_b.contribute(2, &v2).unwrap();

        // A join B
        let mut ab = crdt_a.clone();
        ab.join(&crdt_b).unwrap();

        // B join A
        let mut ba = crdt_b.clone();
        ba.join(&crdt_a).unwrap();

        assert_eq!(ab.readout(), ba.readout(), "Join must be commutative");
    }

    #[test]
    fn test_crdt_join_is_associative() {
        let v1 = BipolarVector::from_seed(10);
        let v2 = BipolarVector::from_seed(20);
        let v3 = BipolarVector::from_seed(30);

        let mut c1 = HdcCrdt::standard();
        c1.contribute(1, &v1).unwrap();
        let mut c2 = HdcCrdt::standard();
        c2.contribute(2, &v2).unwrap();
        let mut c3 = HdcCrdt::standard();
        c3.contribute(3, &v3).unwrap();

        // (c1 join c2) join c3
        let mut ab_c = c1.clone();
        ab_c.join(&c2).unwrap();
        ab_c.join(&c3).unwrap();

        // c1 join (c2 join c3)
        let mut bc = c2.clone();
        bc.join(&c3).unwrap();
        let mut a_bc = c1.clone();
        a_bc.join(&bc).unwrap();

        assert_eq!(ab_c.readout(), a_bc.readout(), "Join must be associative");
    }

    #[test]
    fn test_crdt_join_is_idempotent() {
        let v = BipolarVector::from_seed(42);
        let mut crdt = HdcCrdt::standard();
        crdt.contribute(1, &v).unwrap();

        let before = crdt.readout();
        crdt.join(&crdt.clone()).unwrap();
        let after = crdt.readout();

        assert_eq!(before, after, "Join must be idempotent");
    }

    #[test]
    fn test_crdt_readout_matches_majority() {
        let mut crdt = HdcCrdt::standard();
        // 3 replicas, 2 agree on seed(1), 1 disagrees with seed(99)
        let majority = BipolarVector::from_seed(1);
        let minority = BipolarVector::from_seed(99);

        crdt.contribute(1, &majority).unwrap();
        crdt.contribute(2, &majority).unwrap();
        crdt.contribute(3, &minority).unwrap();

        let result = crdt.readout();
        let sim_majority = result.similarity(&majority).unwrap();
        let sim_minority = result.similarity(&minority).unwrap();

        assert!(sim_majority > sim_minority,
            "Readout should be closer to majority: maj={:.3} min={:.3}",
            sim_majority, sim_minority);
    }

    #[test]
    fn test_crdt_replica_count() {
        let mut crdt = HdcCrdt::standard();
        assert_eq!(crdt.replica_count(), 0);

        crdt.contribute(1, &BipolarVector::from_seed(1)).unwrap();
        crdt.contribute(2, &BipolarVector::from_seed(2)).unwrap();
        assert_eq!(crdt.replica_count(), 2);
    }

    #[test]
    fn test_crdt_delta_gossip() {
        let mut crdt = HdcCrdt::standard();
        let v = BipolarVector::from_seed(1);
        crdt.contribute(1, &v).unwrap();

        // Delta from zero state should include all changed dimensions
        let zeros = vec![0u32; HD_DIMENSIONS];
        let delta = crdt.delta_since(1, &zeros, &zeros);
        assert_eq!(delta.len(), HD_DIMENSIONS, "First contribute changes all dimensions");
    }

    #[test]
    fn test_crdt_empty_readout() {
        let crdt = HdcCrdt::standard();
        let result = crdt.readout();
        // All zeros → all -1 (bit=0) since 0 > 0 is false
        assert_eq!(result.count_ones(), 0, "Empty CRDT readout should be all -1");
    }
}
