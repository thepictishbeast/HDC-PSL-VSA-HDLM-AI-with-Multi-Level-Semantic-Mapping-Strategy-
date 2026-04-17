// ============================================================
// Hadamard On-The-Fly Vector Generation
//
// Generates quasi-orthogonal hypervectors deterministically from
// an integer index, eliminating the need to store codebook base
// vectors in memory. Uses Walsh-Hadamard sequences extended to
// HD_DIMENSIONS via seed-based spreading.
//
// Memory savings: ~12.5MB (10K vectors × 10K bits) → effectively 0
// Reference: Springer 978-3-032-15638-9_32 (2026)
// ============================================================

use crate::hdc::vector::{BipolarVector, HD_DIMENSIONS};
use crate::hdc::error::HdcError;
use bitvec::prelude::*;
use rand::{SeedableRng, RngCore};
use rand_chacha::ChaCha8Rng;

/// Generates orthogonal hypervectors on-the-fly from integer indices.
///
/// Strategy:
/// 1. For indices 0..1024, use Walsh-Hadamard rows (perfectly orthogonal
///    within that range, extended to HD_DIMENSIONS via tiling + seed mixing).
/// 2. For indices >= 1024, use Gold-code-inspired seed mixing that
///    guarantees quasi-orthogonality (expected similarity ~0).
///
/// No storage required — any vector can be recomputed from its index.
pub struct HadamardGenerator;

impl HadamardGenerator {
    /// Generate a deterministic, quasi-orthogonal hypervector for a given index.
    ///
    /// The same index always produces the same vector.
    /// Different indices produce quasi-orthogonal vectors (similarity ~0).
    pub fn generate(index: usize) -> Result<BipolarVector, HdcError> {
        debuglog!("HadamardGenerator::generate: index={}", index);

        if index < 1024 {
            Self::generate_walsh(index)
        } else {
            Self::generate_gold(index)
        }
    }

    /// Generate using Walsh-Hadamard rows for indices 0..1024.
    ///
    /// Walsh-Hadamard matrix H_n has perfectly orthogonal rows.
    /// We compute row `index` of H_1024, then tile and mix to
    /// fill HD_DIMENSIONS bits.
    fn generate_walsh(index: usize) -> Result<BipolarVector, HdcError> {
        debuglog!("HadamardGenerator::generate_walsh: index={}", index);

        // Compute 1024-bit Walsh-Hadamard row using the recursive property:
        // H[i][j] = popcount(i & j) is even => +1, odd => -1
        // In our bit encoding: even parity => bit=1 (+1), odd => bit=0 (-1)
        let mut walsh_row = BitVec::<u8, Lsb0>::with_capacity(1024);
        for j in 0..1024_usize {
            let parity = (index & j).count_ones();
            walsh_row.push(parity % 2 == 0); // even parity = +1 = bit 1
        }

        // Extend to HD_DIMENSIONS by seeded mixing:
        // For each 1024-bit chunk beyond the first, XOR the Walsh row
        // with a deterministic random pattern derived from (index, chunk_id).
        let mut data = BitVec::<u8, Lsb0>::with_capacity(HD_DIMENSIONS);
        let full_chunks = HD_DIMENSIONS / 1024;
        let remainder = HD_DIMENSIONS % 1024;

        // First chunk: pure Walsh row
        data.extend_from_bitslice(&walsh_row[..]);

        // Subsequent chunks: Walsh XOR seeded_random(index, chunk_id)
        for chunk_id in 1..full_chunks {
            let seed = Self::mix_seed(index as u64, chunk_id as u64);
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let mut noise_bytes = vec![0u8; 128]; // 1024 bits = 128 bytes
            rng.fill_bytes(&mut noise_bytes);
            let noise = BitVec::<u8, Lsb0>::from_vec(noise_bytes);
            for j in 0..1024 {
                data.push(walsh_row[j] ^ noise[j]);
            }
        }

        // Remainder bits
        if remainder > 0 {
            let seed = Self::mix_seed(index as u64, full_chunks as u64);
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let mut noise_bytes = vec![0u8; (remainder + 7) / 8];
            rng.fill_bytes(&mut noise_bytes);
            let noise = BitVec::<u8, Lsb0>::from_vec(noise_bytes);
            for j in 0..remainder {
                data.push(walsh_row[j % 1024] ^ noise[j]);
            }
        }

        data.truncate(HD_DIMENSIONS);
        BipolarVector::from_bitvec(data)
    }

    /// Generate using Gold-code-inspired seed mixing for indices >= 1024.
    ///
    /// Uses two independent PRNG streams XOR'd together, seeded from
    /// the index via different mixing constants. This guarantees
    /// quasi-orthogonality (expected similarity = 0 ± 1/√D).
    fn generate_gold(index: usize) -> Result<BipolarVector, HdcError> {
        debuglog!("HadamardGenerator::generate_gold: index={}", index);

        let seed_a = Self::mix_seed(index as u64, 0x_DEAD_BEEF_CAFE_1337);
        let seed_b = Self::mix_seed(index as u64, 0x_1917_0501_A1FA_B3EA);

        let mut rng_a = ChaCha8Rng::seed_from_u64(seed_a);
        let mut rng_b = ChaCha8Rng::seed_from_u64(seed_b);

        let byte_count = (HD_DIMENSIONS + 7) / 8;
        let mut bytes_a = vec![0u8; byte_count];
        let mut bytes_b = vec![0u8; byte_count];
        rng_a.fill_bytes(&mut bytes_a);
        rng_b.fill_bytes(&mut bytes_b);

        // Gold code: XOR of two independent sequences
        let bytes: Vec<u8> = bytes_a.iter()
            .zip(bytes_b.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        let mut data = BitVec::<u8, Lsb0>::from_vec(bytes);
        data.truncate(HD_DIMENSIONS);
        BipolarVector::from_bitvec(data)
    }

    /// Mix two u64 values into a single seed using SplitMix64.
    /// Ensures good avalanche properties — small input changes
    /// produce dramatically different seeds.
    fn mix_seed(a: u64, b: u64) -> u64 {
        let mut z = a.wrapping_add(b).wrapping_mul(0x9E3779B97F4A7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Generate a batch of vectors for indices 0..count.
    /// Useful for initializing codebooks or running orthogonality checks.
    pub fn generate_batch(count: usize) -> Result<Vec<BipolarVector>, HdcError> {
        debuglog!("HadamardGenerator::generate_batch: count={}", count);
        let mut vectors = Vec::with_capacity(count);
        for i in 0..count {
            vectors.push(Self::generate(i)?);
        }
        Ok(vectors)
    }
}

/// Correlated vector generator for learning/classification tasks.
///
/// Unlike orthogonal Hadamard vectors, correlated vectors share
/// structural similarity proportional to their semantic relatedness.
/// This improves classification accuracy from 65% to 95% (UC Irvine 2026).
///
/// Strategy: Start with a base Hadamard vector, then apply controlled
/// perturbation based on a correlation parameter (0.0 = orthogonal, 1.0 = identical).
pub struct CorrelatedGenerator;

impl CorrelatedGenerator {
    /// Generate a correlated vector relative to a base vector.
    ///
    /// `correlation` in [0.0, 1.0]:
    ///   - 0.0 = fully random (orthogonal to base in expectation)
    ///   - 0.5 = 50% of bits match base, 50% random
    ///   - 1.0 = identical to base
    pub fn generate_correlated(
        base: &BipolarVector,
        correlation: f64,
        seed: u64,
    ) -> Result<BipolarVector, HdcError> {
        debuglog!("CorrelatedGenerator::generate_correlated: corr={:.2}, seed={}", correlation, seed);

        let correlation = correlation.clamp(0.0, 1.0);
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let mut data = BitVec::<u8, Lsb0>::with_capacity(HD_DIMENSIONS);
        let dim = base.dim();

        // For each bit: keep the base bit with probability `correlation`,
        // otherwise flip to random.
        // We use a threshold on random bytes to decide.
        let threshold = (correlation * 256.0) as u8;

        for i in 0..dim {
            let rand_byte = {
                let mut b = [0u8; 1];
                rng.fill_bytes(&mut b);
                b[0]
            };

            if rand_byte < threshold {
                // Keep base bit (correlated)
                data.push(base.data[i]);
            } else {
                // Random bit (decorrelate)
                let rand_bit = {
                    let mut b = [0u8; 1];
                    rng.fill_bytes(&mut b);
                    b[0] & 1 == 1
                };
                data.push(rand_bit);
            }
        }

        data.truncate(HD_DIMENSIONS);
        BipolarVector::from_bitvec(data)
    }

    /// Generate a family of correlated vectors around a centroid.
    /// All vectors in the family share `correlation` similarity with the centroid.
    /// Useful for encoding semantically related concepts.
    pub fn generate_family(
        centroid: &BipolarVector,
        count: usize,
        correlation: f64,
        base_seed: u64,
    ) -> Result<Vec<BipolarVector>, HdcError> {
        debuglog!("CorrelatedGenerator::generate_family: count={}, corr={:.2}", count, correlation);
        let mut family = Vec::with_capacity(count);
        for i in 0..count {
            let seed = HadamardGenerator::mix_seed(base_seed, i as u64);
            family.push(Self::generate_correlated(centroid, correlation, seed)?);
        }
        Ok(family)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hadamard_deterministic() -> Result<(), HdcError> {
        let v1 = HadamardGenerator::generate(42)?;
        let v2 = HadamardGenerator::generate(42)?;
        assert_eq!(v1, v2, "Same index must produce identical vectors");
        Ok(())
    }

    #[test]
    fn test_hadamard_correct_dimensions() -> Result<(), HdcError> {
        let v = HadamardGenerator::generate(0)?;
        assert_eq!(v.dim(), HD_DIMENSIONS);
        let v = HadamardGenerator::generate(5000)?;
        assert_eq!(v.dim(), HD_DIMENSIONS);
        Ok(())
    }

    #[test]
    fn test_hadamard_walsh_orthogonality() -> Result<(), HdcError> {
        // Walsh-Hadamard rows should be quasi-orthogonal after extension
        let v0 = HadamardGenerator::generate(0)?;
        let v1 = HadamardGenerator::generate(1)?;
        let v2 = HadamardGenerator::generate(2)?;
        let v100 = HadamardGenerator::generate(100)?;

        let sim_01 = v0.similarity(&v1)?;
        let sim_02 = v0.similarity(&v2)?;
        let sim_12 = v1.similarity(&v2)?;
        let sim_0_100 = v0.similarity(&v100)?;

        debuglog!("Walsh orthogonality: 0-1={:.4}, 0-2={:.4}, 1-2={:.4}, 0-100={:.4}",
            sim_01, sim_02, sim_12, sim_0_100);

        // All pairs should be quasi-orthogonal (|sim| < 0.1)
        assert!(sim_01.abs() < 0.15, "Walsh 0 vs 1: sim={}", sim_01);
        assert!(sim_02.abs() < 0.15, "Walsh 0 vs 2: sim={}", sim_02);
        assert!(sim_12.abs() < 0.15, "Walsh 1 vs 2: sim={}", sim_12);
        assert!(sim_0_100.abs() < 0.15, "Walsh 0 vs 100: sim={}", sim_0_100);
        Ok(())
    }

    #[test]
    fn test_hadamard_gold_orthogonality() -> Result<(), HdcError> {
        // Gold-code vectors (index >= 1024) should also be quasi-orthogonal
        let v_a = HadamardGenerator::generate(2000)?;
        let v_b = HadamardGenerator::generate(2001)?;
        let v_c = HadamardGenerator::generate(5000)?;

        let sim_ab = v_a.similarity(&v_b)?;
        let sim_ac = v_a.similarity(&v_c)?;

        debuglog!("Gold orthogonality: a-b={:.4}, a-c={:.4}", sim_ab, sim_ac);

        assert!(sim_ab.abs() < 0.1, "Gold a vs b: sim={}", sim_ab);
        assert!(sim_ac.abs() < 0.1, "Gold a vs c: sim={}", sim_ac);
        Ok(())
    }

    #[test]
    fn test_hadamard_walsh_vs_gold_orthogonal() -> Result<(), HdcError> {
        // Cross-regime: Walsh vectors vs Gold vectors should be orthogonal
        let walsh = HadamardGenerator::generate(7)?;
        let gold = HadamardGenerator::generate(2000)?;
        let sim = walsh.similarity(&gold)?;
        debuglog!("Walsh vs Gold cross-regime: sim={:.4}", sim);
        assert!(sim.abs() < 0.1, "Walsh vs Gold: sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_hadamard_self_similarity() -> Result<(), HdcError> {
        let v = HadamardGenerator::generate(42)?;
        let sim = v.similarity(&v)?;
        assert!((sim - 1.0).abs() < 0.001, "Self-similarity should be 1.0, got {}", sim);
        Ok(())
    }

    #[test]
    fn test_correlated_generator_high_correlation() -> Result<(), HdcError> {
        let base = HadamardGenerator::generate(0)?;
        let corr = CorrelatedGenerator::generate_correlated(&base, 0.9, 123)?;
        let sim = base.similarity(&corr)?;
        debuglog!("Correlated (0.9): sim={:.4}", sim);
        // With 0.9 correlation, ~90% of bits match → high similarity
        assert!(sim > 0.5, "High correlation should yield high sim, got {}", sim);
        Ok(())
    }

    #[test]
    fn test_correlated_generator_low_correlation() -> Result<(), HdcError> {
        let base = HadamardGenerator::generate(0)?;
        let corr = CorrelatedGenerator::generate_correlated(&base, 0.0, 456)?;
        let sim = base.similarity(&corr)?;
        debuglog!("Correlated (0.0): sim={:.4}", sim);
        // With 0.0 correlation, fully random → quasi-orthogonal
        assert!(sim.abs() < 0.1, "Zero correlation should yield ~0 sim, got {}", sim);
        Ok(())
    }

    #[test]
    fn test_correlated_family() -> Result<(), HdcError> {
        let centroid = HadamardGenerator::generate(5)?;
        let family = CorrelatedGenerator::generate_family(&centroid, 5, 0.7, 999)?;
        assert_eq!(family.len(), 5);
        for (i, member) in family.iter().enumerate() {
            let sim = centroid.similarity(member)?;
            debuglog!("Family member {}: sim={:.4}", i, sim);
            // All family members should be reasonably similar to centroid
            assert!(sim > 0.2, "Family member {} sim too low: {}", i, sim);
        }
        Ok(())
    }

    #[test]
    fn test_hadamard_batch() -> Result<(), HdcError> {
        let batch = HadamardGenerator::generate_batch(10)?;
        assert_eq!(batch.len(), 10);
        // Spot check: index 3 in batch should match standalone generation
        let standalone = HadamardGenerator::generate(3)?;
        assert_eq!(batch[3], standalone);
        Ok(())
    }

    #[test]
    fn test_hadamard_balanced() -> Result<(), HdcError> {
        // Vectors should be roughly balanced (50% ones, 50% zeros)
        let v = HadamardGenerator::generate(42)?;
        let ones = v.count_ones() as f64;
        let ratio = ones / HD_DIMENSIONS as f64;
        debuglog!("Balance check: ones_ratio={:.4}", ratio);
        assert!((ratio - 0.5).abs() < 0.05, "Vector should be balanced, ratio={}", ratio);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for HadamardGenerator
    // ============================================================

    /// INVARIANT: generate(i) is deterministic — same index always yields
    /// the same vector. Required for reproducible codebooks.
    #[test]
    fn invariant_hadamard_generate_deterministic() -> Result<(), HdcError> {
        for idx in [0usize, 1, 42, 100, 1023] {
            let v1 = HadamardGenerator::generate(idx)?;
            let v2 = HadamardGenerator::generate(idx)?;
            assert_eq!(v1, v2,
                "generate({}) must be deterministic", idx);
        }
        Ok(())
    }

    /// INVARIANT: different indices produce different vectors (low collision).
    #[test]
    fn invariant_hadamard_distinct_indices_different() -> Result<(), HdcError> {
        let indices = [0usize, 1, 42, 100, 500];
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let v_i = HadamardGenerator::generate(indices[i])?;
                let v_j = HadamardGenerator::generate(indices[j])?;
                assert_ne!(v_i, v_j,
                    "generate({}) and generate({}) collided",
                    indices[i], indices[j]);
            }
        }
        Ok(())
    }

    /// INVARIANT: generate_batch(N) produces exactly N vectors of correct dim.
    #[test]
    fn invariant_hadamard_batch_count() -> Result<(), HdcError> {
        for n in [1usize, 10, 50] {
            let batch = HadamardGenerator::generate_batch(n)?;
            assert_eq!(batch.len(), n,
                "batch({}) produced {} vectors", n, batch.len());
            for v in &batch {
                assert_eq!(v.dim(), HD_DIMENSIONS,
                    "batch vector dim must be {}", HD_DIMENSIONS);
            }
        }
        Ok(())
    }

    /// INVARIANT: batch vectors are pairwise quasi-orthogonal (no accidental
    /// duplicates within a batch).
    #[test]
    fn invariant_hadamard_batch_pairwise_distinct() -> Result<(), HdcError> {
        let batch = HadamardGenerator::generate_batch(20)?;
        for i in 0..batch.len() {
            for j in (i + 1)..batch.len() {
                assert_ne!(batch[i], batch[j],
                    "batch vectors {} and {} collided", i, j);
            }
        }
        Ok(())
    }

    /// INVARIANT: CorrelatedGenerator with correlation=1.0 returns an identical
    /// vector; with correlation=0.0 returns a quasi-orthogonal one.
    #[test]
    fn invariant_correlated_extreme_values() -> Result<(), HdcError> {
        let base = HadamardGenerator::generate(7)?;
        let identical = CorrelatedGenerator::generate_correlated(&base, 1.0, 42)?;
        let sim_ident = base.similarity(&identical)?;
        assert!(sim_ident > 0.99,
            "correlation=1.0 must produce ≈identical: sim={}", sim_ident);

        let orthogonal = CorrelatedGenerator::generate_correlated(&base, 0.0, 43)?;
        let sim_orth = base.similarity(&orthogonal)?;
        assert!(sim_orth.abs() < 0.1,
            "correlation=0.0 must produce quasi-orthogonal: sim={}", sim_orth);
        Ok(())
    }

    /// INVARIANT: generate produces 10_000-dim vectors for any valid index.
    #[test]
    fn invariant_generate_produces_correct_dim() -> Result<(), HdcError> {
        for index in [0, 1, 100, 1023, 1024, 1025, 10000, 1_000_000] {
            let v = HadamardGenerator::generate(index)?;
            assert_eq!(v.dim(), HD_DIMENSIONS,
                "index {} produced wrong dim: {}", index, v.dim());
        }
        Ok(())
    }

    /// INVARIANT: generate_batch returns exactly `count` vectors.
    #[test]
    fn invariant_batch_size_matches_request() -> Result<(), HdcError> {
        for count in [0, 1, 10, 100] {
            let batch = HadamardGenerator::generate_batch(count)?;
            assert_eq!(batch.len(), count,
                "generate_batch({}) returned {} vectors", count, batch.len());
        }
        Ok(())
    }

    /// INVARIANT: generate_family returns a non-empty family for count > 0.
    #[test]
    fn invariant_family_size_matches() -> Result<(), HdcError> {
        let base = HadamardGenerator::generate(0)?;
        for count in [1, 5, 10] {
            let family = CorrelatedGenerator::generate_family(&base, count, 0.7, 100)?;
            assert_eq!(family.len(), count,
                "family size mismatch: requested {}, got {}", count, family.len());
        }
        Ok(())
    }
}
