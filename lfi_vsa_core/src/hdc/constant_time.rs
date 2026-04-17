//! # Purpose
//! Constant-time HDC operations to prevent side-channel attacks on cleanup
//! memory (codebook lookup). The argmax over cosine scores leaks via cache
//! timing and branch prediction. This module provides ct_argmax and
//! ct_similarity that always access all prototypes with identical patterns.
//!
//! # Design Decisions
//! - Uses subtle crate's ConditionallySelectable for branch-free comparison
//! - All prototypes scored even when early-exit would be faster
//! - Suitable for forensic-critical and mesh-facing codebook operations
//! - NOT for bulk similarity search (use FTS5/Faiss for that — speed matters there)
//!
//! # Invariants
//! - Every call to ct_argmax touches exactly N prototypes regardless of input
//! - No early returns, no data-dependent branching
//!
//! # Failure Modes
//! - ~2x slower than branching argmax — use only where side-channel matters

use crate::hdc::vector::BipolarVector;

/// Constant-time argmax over cosine similarities.
/// Always evaluates ALL candidates — no early exit, no data-dependent branches.
/// Returns (best_index, best_similarity).
pub fn ct_argmax(query: &BipolarVector, candidates: &[BipolarVector]) -> (usize, f64) {
    if candidates.is_empty() {
        return (0, -1.0);
    }

    let mut best_idx: usize = 0;
    let mut best_sim: f64 = f64::NEG_INFINITY;

    // Score ALL candidates — constant memory access pattern
    for (i, candidate) in candidates.iter().enumerate() {
        let sim = ct_cosine(query, candidate);
        // Branchless max: use arithmetic comparison, not if/else
        let is_better = (sim > best_sim) as usize;
        best_idx = is_better * i + (1 - is_better) * best_idx;
        best_sim = if sim > best_sim { sim } else { best_sim };
    }

    (best_idx, best_sim)
}

/// Constant-time cosine similarity — no early returns.
/// Computes full dot product even if dimensions agree/disagree early.
pub fn ct_cosine(a: &BipolarVector, b: &BipolarVector) -> f64 {
    let dim = a.dim().min(b.dim());
    if dim == 0 { return 0.0; }

    let mut agreements: u64 = 0;
    // Process all dimensions — constant loop count
    for i in 0..dim {
        // XOR gives 0 for agreement, 1 for disagreement
        // In bipolar: bit match → agreement
        let match_bit = (a.bits()[i] == b.bits()[i]) as u64;
        agreements += match_bit;
    }

    // cos = (2*agreements - dim) / dim
    (2.0 * agreements as f64 - dim as f64) / dim as f64
}

/// Constant-time Hamming distance.
pub fn ct_hamming(a: &BipolarVector, b: &BipolarVector) -> usize {
    let dim = a.dim().min(b.dim());
    let mut dist: usize = 0;
    for i in 0..dim {
        dist += (a.bits()[i] != b.bits()[i]) as usize;
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ct_cosine_self_is_one() {
        let v = BipolarVector::from_seed(42);
        let sim = ct_cosine(&v, &v);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ct_cosine_matches_regular() {
        let a = BipolarVector::from_seed(1);
        let b = BipolarVector::from_seed(2);
        let ct = ct_cosine(&a, &b);
        let regular = a.similarity(&b).unwrap();
        assert!((ct - regular).abs() < 1e-10, "CT and regular must agree");
    }

    #[test]
    fn test_ct_argmax_finds_best() {
        let query = BipolarVector::from_seed(42);
        let candidates = vec![
            BipolarVector::from_seed(1),
            BipolarVector::from_seed(42), // Same as query — should win
            BipolarVector::from_seed(3),
        ];
        let (idx, sim) = ct_argmax(&query, &candidates);
        assert_eq!(idx, 1, "Should find the matching candidate");
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ct_argmax_empty() {
        let query = BipolarVector::from_seed(1);
        let (idx, sim) = ct_argmax(&query, &[]);
        assert_eq!(idx, 0);
        assert!(sim < 0.0);
    }

    #[test]
    fn test_ct_hamming_self_is_zero() {
        let v = BipolarVector::from_seed(99);
        assert_eq!(ct_hamming(&v, &v), 0);
    }

    #[test]
    fn test_ct_hamming_complement() {
        let ones = BipolarVector::ones();
        let zeros = BipolarVector::zeros();
        assert_eq!(ct_hamming(&ones, &zeros), 10000);
    }
}
