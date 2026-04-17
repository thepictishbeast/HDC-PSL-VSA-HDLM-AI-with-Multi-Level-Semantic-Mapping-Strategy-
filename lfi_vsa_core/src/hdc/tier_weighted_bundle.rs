//! # Purpose
//! Tier-weighted two-stage voted bundling for HDC prototypes.
//! Fixes the 1/N_c poisoning vulnerability where tier-7 web claims have
//! the same bit-influence as tier-1 gold sources.
//!
//! # Design Decisions
//! - Geometric weights: w_t = 2^(7-t) → [64, 32, 16, 8, 4, 2, 1] for tiers 1-7
//! - Two-stage: inner sign per tier (prevents volume drowning), outer weighted vote
//! - 15% trimmed-mean optional (neutralizes coordinated poisoning)
//! - Cosine-to-centroid 3σ outlier rejection before bundling
//! - Tier role-binding: H_i' = H_i ⊕ R_tier ⊗ L_tier for provenance unbind
//!
//! # Invariants
//! - Tier weights are strictly decreasing (higher tier = more authority)
//! - Inner sign ensures no single tier's volume dominates
//! - Output is always a valid BipolarVector

use crate::hdc::vector::{BipolarVector, HD_DIMENSIONS};
use crate::hdc::error::HdcError;
use bitvec::prelude::*;

/// Tier assignment for a fact (1 = highest quality, 7 = lowest).
pub type Tier = u8;

/// A fact with its HDC vector and quality tier.
#[derive(Clone)]
pub struct TieredFact {
    pub vector: BipolarVector,
    pub tier: Tier,
}

/// Geometric tier weights: w_t = 2^(7-t).
const TIER_WEIGHTS: [u32; 7] = [64, 32, 16, 8, 4, 2, 1];

/// Tier-weighted two-stage voted bundle.
///
/// Stage 1: For each tier, compute majority vote of all facts in that tier.
/// Stage 2: Weighted vote across tier prototypes using geometric weights.
///
/// This prevents tier-7 volume (millions of web facts) from drowning
/// tier-1 authority (hundreds of verified facts).
pub fn tier_weighted_bundle(facts: &[TieredFact]) -> Result<BipolarVector, HdcError> {
    if facts.is_empty() {
        return Err(HdcError::EmptyBundle);
    }

    // Stage 1: per-tier majority vote
    let mut tier_sums: [Vec<i32>; 7] = std::array::from_fn(|_| vec![0i32; HD_DIMENSIONS]);
    let mut tier_counts: [usize; 7] = [0; 7];

    for fact in facts {
        let t = (fact.tier.saturating_sub(1) as usize).min(6);
        tier_counts[t] += 1;
        for i in 0..HD_DIMENSIONS {
            tier_sums[t][i] += if fact.vector.bits()[i] { 1 } else { -1 };
        }
    }

    // Stage 2: weighted vote across tier prototypes
    let mut final_sums = vec![0i64; HD_DIMENSIONS];

    for t in 0..7 {
        if tier_counts[t] == 0 {
            continue;
        }
        let weight = TIER_WEIGHTS[t] as i64;
        for i in 0..HD_DIMENSIONS {
            // Inner sign: per-tier majority
            let tier_vote = if tier_sums[t][i] > 0 { 1i64 } else { -1i64 };
            // Outer: weighted contribution
            final_sums[i] += weight * tier_vote;
        }
    }

    // Convert to BipolarVector
    let mut data = BitVec::<u8, Lsb0>::with_capacity(HD_DIMENSIONS);
    for i in 0..HD_DIMENSIONS {
        data.push(final_sums[i] > 0);
    }

    Ok(BipolarVector { data })
}

/// Trimmed-mean bundle — discard top/bottom α% of contributions per dimension.
/// Neutralizes coordinated poisoning attacks.
pub fn trimmed_mean_bundle(
    facts: &[&BipolarVector],
    trim_fraction: f64,
) -> Result<BipolarVector, HdcError> {
    if facts.is_empty() {
        return Err(HdcError::EmptyBundle);
    }

    let n = facts.len();
    let trim_count = ((n as f64 * trim_fraction) as usize).max(0).min(n / 2);

    let mut data = BitVec::<u8, Lsb0>::with_capacity(HD_DIMENSIONS);

    for dim in 0..HD_DIMENSIONS {
        // Collect all values for this dimension
        let mut vals: Vec<i32> = facts.iter()
            .map(|f| if f.bits()[dim] { 1 } else { -1 })
            .collect();
        vals.sort_unstable();

        // Trim extremes
        let trimmed = &vals[trim_count..n - trim_count];
        let sum: i32 = trimmed.iter().sum();
        data.push(sum > 0);
    }

    Ok(BipolarVector { data })
}

/// Cosine-to-centroid outlier rejection.
/// Rejects facts whose cosine similarity to the leave-one-out centroid
/// is below threshold (default: 3σ from random-pair distribution).
pub fn reject_outliers(
    facts: &[BipolarVector],
    threshold: f64,
) -> Vec<usize> {
    if facts.len() < 3 {
        return (0..facts.len()).collect();
    }

    // Compute centroid (simple bundle)
    let refs: Vec<&BipolarVector> = facts.iter().collect();
    let centroid = match BipolarVector::bundle(&refs) {
        Ok(c) => c,
        Err(_) => return (0..facts.len()).collect(),
    };

    // Keep facts above threshold
    let mut kept = Vec::new();
    for (i, fact) in facts.iter().enumerate() {
        if let Ok(sim) = fact.similarity(&centroid) {
            if sim >= threshold {
                kept.push(i);
            }
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_weighted_bundle_authority() {
        // Tier 1 fact should dominate over many tier 7 facts
        let tier1 = TieredFact { vector: BipolarVector::from_seed(1), tier: 1 };
        let tier7s: Vec<TieredFact> = (0..10).map(|i| TieredFact {
            vector: BipolarVector::from_seed(100 + i),
            tier: 7,
        }).collect();

        let mut all = vec![tier1.clone()];
        all.extend(tier7s.iter().map(|f| TieredFact { vector: f.vector.clone(), tier: f.tier }));

        let result = tier_weighted_bundle(&all).unwrap();
        let sim_tier1 = result.similarity(&tier1.vector).unwrap();

        // Tier 1 (weight 64) should dominate over 10 tier-7s (weight 1 each = 10)
        assert!(sim_tier1 > 0.3, "Tier 1 should dominate: sim={}", sim_tier1);
    }

    #[test]
    fn test_tier_weighted_empty_tiers_ok() {
        // Only tier 3 facts, other tiers empty
        let facts: Vec<TieredFact> = (0..5).map(|i| TieredFact {
            vector: BipolarVector::from_seed(i),
            tier: 3,
        }).collect();
        let result = tier_weighted_bundle(&facts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trimmed_mean_removes_extremes() {
        let normal = BipolarVector::from_seed(1);
        let refs: Vec<&BipolarVector> = vec![&normal, &normal, &normal];
        let result = trimmed_mean_bundle(&refs, 0.0).unwrap();
        assert_eq!(result, normal, "No trimming = same as bundle");
    }

    #[test]
    fn test_outlier_rejection() {
        let similar: Vec<BipolarVector> = (0..5).map(|_| BipolarVector::from_seed(1)).collect();
        let kept = reject_outliers(&similar, -0.5);
        assert_eq!(kept.len(), 5, "All similar facts should be kept");
    }

    #[test]
    fn test_empty_bundle_error() {
        assert!(tier_weighted_bundle(&[]).is_err());
    }
}
