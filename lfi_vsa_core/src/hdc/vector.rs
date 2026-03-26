// ============================================================
// BipolarVector — 10,000-Dimensional Hyperdimensional Computing
// Section 1.I: HDC Core (Logic & Compute)
//
// Encoding: bit=0 -> bipolar -1,  bit=1 -> bipolar +1
// Algebra:  Binding=XOR, Bundling=Sum+Clip, Permutation=CyclicShift
// ============================================================

use bitvec::prelude::*;
use rand::RngCore;
use crate::hdc::error::HdcError;
use crate::debuglog;

/// Dimensionality of the hyperdimensional space.
pub const HD_DIMENSIONS: usize = 10_000;

/// A bipolar hypervector in {-1, +1}^10000 stored as a bitvec.
///
/// Bit mapping:  `0 -> -1`,  `1 -> +1`.
/// This allows Binding (bipolar multiplication) to map directly to XOR.
#[derive(Clone, Debug, PartialEq)]
pub struct BipolarVector {
    data: BitVec<u8, Lsb0>,
}

impl BipolarVector {
    // ====================================================
    // Constructors
    // ====================================================

    /// Initialize a random bipolar hypervector using system RNG.
    pub fn new_random() -> Result<Self, HdcError> {
        let byte_count = (HD_DIMENSIONS + 7) / 8;
        let mut bytes = vec![0u8; byte_count];
        rand::thread_rng().fill_bytes(&mut bytes);

        let mut data = BitVec::<u8, Lsb0>::from_vec(bytes);
        data.truncate(HD_DIMENSIONS);

        debuglog!("new_random: dim={}", data.len());

        if data.len() == HD_DIMENSIONS {
            Ok(Self { data })
        } else {
            debuglog!("new_random FAIL: dim={} != {}", data.len(), HD_DIMENSIONS);
            Err(HdcError::InitializationFailed {
                reason: format!("expected {} bits, got {}", HD_DIMENSIONS, data.len()),
            })
        }
    }

    /// Construct from an existing BitVec. Enforces exact dimensionality.
    pub fn from_bitvec(data: BitVec<u8, Lsb0>) -> Result<Self, HdcError> {
        debuglog!("from_bitvec: input dim={}", data.len());
        if data.len() == HD_DIMENSIONS {
            Ok(Self { data })
        } else {
            Err(HdcError::DimensionMismatch {
                expected: HD_DIMENSIONS,
                actual: data.len(),
            })
        }
    }

    /// All-zeros vector (all components = -1 in bipolar space).
    pub fn zeros() -> Self {
        let data = bitvec![u8, Lsb0; 0; HD_DIMENSIONS];
        debuglog!("zeros: dim={}", data.len());
        Self { data }
    }

    /// All-ones vector (all components = +1 in bipolar space).
    pub fn ones() -> Self {
        let data = bitvec![u8, Lsb0; 1; HD_DIMENSIONS];
        debuglog!("ones: dim={}", data.len());
        Self { data }
    }

    // ====================================================
    // Accessors
    // ====================================================

    /// Number of dimensions.
    pub fn dim(&self) -> usize {
        self.data.len()
    }

    /// Read-only access to the underlying bit-slice.
    pub fn bits(&self) -> &BitSlice<u8, Lsb0> {
        &self.data
    }

    /// Count of +1 components (bits set to 1).
    pub fn count_ones(&self) -> usize {
        self.data.count_ones()
    }

    /// Count of -1 components (bits set to 0).
    pub fn count_neg_ones(&self) -> usize {
        self.data.count_zeros()
    }

    // ====================================================
    // HDC Algebra — The Three Fundamental Operations
    // ====================================================

    /// **Binding (XOR)**
    ///
    /// Element-wise XOR implements bipolar binding.
    /// XOR identity element = all-zeros bitvec (A XOR 0 = A).
    /// Self-inverse: A XOR A = 0 (identity), so A XOR A XOR B = B.
    ///
    /// Properties: commutative, associative, self-inverse, dimension-preserving.
    /// Produces a vector quasi-orthogonal to both inputs.
    pub fn bind(&self, other: &BipolarVector) -> Result<BipolarVector, HdcError> {
        self.check_dim()?;
        other.check_dim()?;

        let result_bits = self.data.clone() ^ &other.data;
        debuglog!("bind: completed, result dim={}", result_bits.len());
        Ok(BipolarVector { data: result_bits })
    }

    /// **Bundling (Sum + Clip)**
    ///
    /// Majority-vote superposition. For each dimension, sum the bipolar
    /// values across all input vectors, then clip:
    ///   sum > 0  -> +1 (bit=1)
    ///   sum <= 0 -> -1 (bit=0)
    ///
    /// Ties (sum=0, even input count) default to -1 (bit=0).
    ///
    /// Properties: commutative, produces vector similar to all inputs.
    pub fn bundle(vectors: &[&BipolarVector]) -> Result<BipolarVector, HdcError> {
        if vectors.is_empty() {
            debuglog!("bundle: EmptyBundle");
            return Err(HdcError::EmptyBundle);
        }

        for (i, v) in vectors.iter().enumerate() {
            if v.data.len() != HD_DIMENSIONS {
                debuglog!("bundle: DimensionMismatch at index={}, dim={}", i, v.data.len());
                return Err(HdcError::DimensionMismatch {
                    expected: HD_DIMENSIONS,
                    actual: v.data.len(),
                });
            }
        }

        // Accumulate in i32 scratch space to handle arbitrary bundle sizes
        let mut sums = vec![0i32; HD_DIMENSIONS];
        for v in vectors {
            for (i, bit) in v.data.iter().enumerate() {
                sums[i] += if *bit { 1 } else { -1 };
            }
        }

        // Clip: strictly positive -> 1, zero or negative -> 0
        let mut result = BitVec::<u8, Lsb0>::with_capacity(HD_DIMENSIONS);
        for s in &sums {
            result.push(*s > 0);
        }

        debuglog!("bundle: merged {} vectors, result dim={}", vectors.len(), result.len());
        Ok(BipolarVector { data: result })
    }

    /// **Permutation (Cyclic Left Shift)**
    ///
    /// Rotates the vector left by `shift` positions.
    /// `new[i] = old[(i + shift) % DIM]`
    ///
    /// Properties: invertible (shift DIM-k undoes shift k),
    /// preserves Hamming weight, produces quasi-orthogonal vector
    /// for non-trivial shifts.
    pub fn permute(&self, shift: usize) -> Result<BipolarVector, HdcError> {
        self.check_dim()?;

        let effective = shift % HD_DIMENSIONS;
        if effective == 0 {
            debuglog!("permute: shift=0, returning clone");
            return Ok(self.clone());
        }

        let mut new_data = BitVec::<u8, Lsb0>::with_capacity(HD_DIMENSIONS);
        for i in 0..HD_DIMENSIONS {
            let src = (i + effective) % HD_DIMENSIONS;
            new_data.push(self.data[src]);
        }

        debuglog!("permute: shift={}, result dim={}", effective, new_data.len());
        Ok(BipolarVector { data: new_data })
    }

    // ====================================================
    // Similarity Metrics
    // ====================================================

    /// Cosine similarity for bipolar vectors.
    ///
    /// `cos(A, B) = (2 * agreements - DIM) / DIM`
    ///
    /// Returns a value in [-1.0, 1.0]:
    ///   +1.0 = identical
    ///    0.0 = orthogonal (expected for random pairs in high D)
    ///   -1.0 = anti-correlated (bitwise complement)
    pub fn similarity(&self, other: &BipolarVector) -> Result<f64, HdcError> {
        self.check_dim()?;
        other.check_dim()?;

        let xor_bits = self.data.clone() ^ &other.data;
        let disagreements = xor_bits.count_ones();
        let agreements = HD_DIMENSIONS - disagreements;

        let sim = (2.0 * agreements as f64 - HD_DIMENSIONS as f64) / HD_DIMENSIONS as f64;
        debuglog!("similarity: agree={}, disagree={}, cos={:.6}", agreements, disagreements, sim);
        Ok(sim)
    }

    /// Hamming distance: count of positions where bits differ.
    pub fn hamming_distance(&self, other: &BipolarVector) -> Result<usize, HdcError> {
        self.check_dim()?;
        other.check_dim()?;

        let xor_bits = self.data.clone() ^ &other.data;
        let dist = xor_bits.count_ones();
        debuglog!("hamming_distance: {}", dist);
        Ok(dist)
    }

    // ====================================================
    // Internal helpers
    // ====================================================

    fn check_dim(&self) -> Result<(), HdcError> {
        if self.data.len() != HD_DIMENSIONS {
            Err(HdcError::DimensionMismatch {
                expected: HD_DIMENSIONS,
                actual: self.data.len(),
            })
        } else {
            Ok(())
        }
    }
}

// ================================================================
// Exhaustive Unit Tests — Mathematical Proofs Under Edge Cases
// Section 4 Alpha Mandate: "mathematically prove the code operates
// under edge cases before integrating it into the core."
//
// All tests use -> Result<(), HdcError> with ? operator.
// Zero uses of .unwrap(), .expect(), or panic!() per Section 5.
// ================================================================
#[cfg(test)]
mod tests {
    use super::*;

    // --------------------------------------------------------
    // I. Initialization Tests
    // --------------------------------------------------------

    #[test]
    fn test_init_random_exact_dimension() -> Result<(), HdcError> {
        let v = BipolarVector::new_random()?;
        assert_eq!(v.dim(), HD_DIMENSIONS);
        Ok(())
    }

    #[test]
    fn test_init_random_nontrivial_distribution() -> Result<(), HdcError> {
        // 10k random bits: P(all same) = 2^{-9999} ~ 0.
        // Expect ~50/50. Threshold 3000 is ~40 sigma; safe.
        let v = BipolarVector::new_random()?;
        assert!(v.count_ones() > 3000, "ones={}", v.count_ones());
        assert!(v.count_neg_ones() > 3000, "neg_ones={}", v.count_neg_ones());
        Ok(())
    }

    #[test]
    fn test_init_random_uniqueness() -> Result<(), HdcError> {
        // Collision prob = 2^{-10000} ~ 0.
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert_ne!(a, b);
        Ok(())
    }

    #[test]
    fn test_init_zeros() {
        let v = BipolarVector::zeros();
        assert_eq!(v.dim(), HD_DIMENSIONS);
        assert_eq!(v.count_ones(), 0);
        assert_eq!(v.count_neg_ones(), HD_DIMENSIONS);
    }

    #[test]
    fn test_init_ones() {
        let v = BipolarVector::ones();
        assert_eq!(v.dim(), HD_DIMENSIONS);
        assert_eq!(v.count_ones(), HD_DIMENSIONS);
        assert_eq!(v.count_neg_ones(), 0);
    }

    #[test]
    fn test_from_bitvec_valid() {
        let data = bitvec![u8, Lsb0; 1; HD_DIMENSIONS];
        assert!(BipolarVector::from_bitvec(data).is_ok());
    }

    #[test]
    fn test_from_bitvec_wrong_dimension() {
        let data = bitvec![u8, Lsb0; 0; 999];
        assert_eq!(
            BipolarVector::from_bitvec(data),
            Err(HdcError::DimensionMismatch { expected: HD_DIMENSIONS, actual: 999 })
        );
    }

    #[test]
    fn test_from_bitvec_empty() {
        let data = BitVec::<u8, Lsb0>::new();
        assert_eq!(
            BipolarVector::from_bitvec(data),
            Err(HdcError::DimensionMismatch { expected: HD_DIMENSIONS, actual: 0 })
        );
    }

    // --------------------------------------------------------
    // II. Binding (XOR) — Algebraic Proofs
    // --------------------------------------------------------

    #[test]
    fn test_bind_dimension_preserved() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert_eq!(a.bind(&b)?.dim(), HD_DIMENSIONS);
        Ok(())
    }

    #[test]
    fn test_bind_commutativity() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert_eq!(a.bind(&b)?, b.bind(&a)?);
        Ok(())
    }

    #[test]
    fn test_bind_associativity() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let c = BipolarVector::new_random()?;
        assert_eq!(a.bind(&b)?.bind(&c)?, a.bind(&b.bind(&c)?)?);
        Ok(())
    }

    #[test]
    fn test_bind_self_produces_identity() -> Result<(), HdcError> {
        // A XOR A = all-zeros (the XOR identity element).
        let a = BipolarVector::new_random()?;
        assert_eq!(a.bind(&a)?, BipolarVector::zeros());
        Ok(())
    }

    #[test]
    fn test_bind_self_inverse() -> Result<(), HdcError> {
        // A XOR A XOR B = B (self-inverse property).
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let aa = a.bind(&a)?;
        assert_eq!(aa.bind(&b)?, b);
        Ok(())
    }

    #[test]
    fn test_bind_with_identity() -> Result<(), HdcError> {
        // zeros XOR A = A (identity element property).
        let a = BipolarVector::new_random()?;
        let id = BipolarVector::zeros();
        assert_eq!(id.bind(&a)?, a);
        Ok(())
    }

    #[test]
    fn test_bind_quasi_orthogonal_to_inputs() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let c = a.bind(&b)?;
        assert!(c.similarity(&a)?.abs() < 0.1, "bound vs A");
        assert!(c.similarity(&b)?.abs() < 0.1, "bound vs B");
        Ok(())
    }

    #[test]
    fn test_bind_recovery() -> Result<(), HdcError> {
        // Given C = A XOR B, recover B = A XOR C.
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let c = a.bind(&b)?;
        assert_eq!(a.bind(&c)?, b);
        Ok(())
    }

    // --------------------------------------------------------
    // III. Bundling (Sum+Clip) — Majority Vote Proofs
    // --------------------------------------------------------

    #[test]
    fn test_bundle_single_is_identity() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        assert_eq!(BipolarVector::bundle(&[&a])?, a);
        Ok(())
    }

    #[test]
    fn test_bundle_dimension_preserved() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert_eq!(BipolarVector::bundle(&[&a, &b])?.dim(), HD_DIMENSIONS);
        Ok(())
    }

    #[test]
    fn test_bundle_commutativity() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert_eq!(
            BipolarVector::bundle(&[&a, &b])?,
            BipolarVector::bundle(&[&b, &a])?
        );
        Ok(())
    }

    #[test]
    fn test_bundle_identical_returns_original() -> Result<(), HdcError> {
        // N copies: unanimous vote every dimension -> returns original.
        let a = BipolarVector::new_random()?;
        assert_eq!(BipolarVector::bundle(&[&a, &a, &a, &a, &a])?, a);
        Ok(())
    }

    #[test]
    fn test_bundle_similarity_to_all_inputs() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let c = BipolarVector::new_random()?;
        let bundled = BipolarVector::bundle(&[&a, &b, &c])?;
        assert!(bundled.similarity(&a)? > 0.15, "bundle vs A");
        assert!(bundled.similarity(&b)? > 0.15, "bundle vs B");
        assert!(bundled.similarity(&c)? > 0.15, "bundle vs C");
        Ok(())
    }

    #[test]
    fn test_bundle_majority_vote_dominance() -> Result<(), HdcError> {
        // 3xA vs 2xB: A should dominate.
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        let bundled = BipolarVector::bundle(&[&a, &a, &a, &b, &b])?;
        let sim_a = bundled.similarity(&a)?;
        let sim_b = bundled.similarity(&b)?;
        assert!(sim_a > sim_b, "A(x3) must dominate B(x2): a={:.4} b={:.4}", sim_a, sim_b);
        assert!(sim_a > 0.5, "Majority similarity must be strong: {:.4}", sim_a);
        Ok(())
    }

    #[test]
    fn test_bundle_tie_breaking() -> Result<(), HdcError> {
        // Even count, opposite vectors: sum=0 everywhere -> clips to 0 (bit=0).
        let a = BipolarVector::ones();
        let b = BipolarVector::zeros();
        let bundled = BipolarVector::bundle(&[&a, &b])?;
        assert_eq!(bundled, BipolarVector::zeros(), "Ties must break to -1 (bit=0)");
        Ok(())
    }

    #[test]
    fn test_bundle_empty_returns_error() {
        let empty: Vec<&BipolarVector> = vec![];
        assert_eq!(BipolarVector::bundle(&empty), Err(HdcError::EmptyBundle));
    }

    // --------------------------------------------------------
    // IV. Permutation (Cyclic Shift) — Group Proofs
    // --------------------------------------------------------

    #[test]
    fn test_permute_dimension_preserved() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        assert_eq!(a.permute(42)?.dim(), HD_DIMENSIONS);
        Ok(())
    }

    #[test]
    fn test_permute_zero_is_identity() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        assert_eq!(a.permute(0)?, a);
        Ok(())
    }

    #[test]
    fn test_permute_full_rotation_is_identity() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        assert_eq!(a.permute(HD_DIMENSIONS)?, a);
        Ok(())
    }

    #[test]
    fn test_permute_double_rotation_is_identity() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        assert_eq!(a.permute(2 * HD_DIMENSIONS)?, a);
        Ok(())
    }

    #[test]
    fn test_permute_invertible() -> Result<(), HdcError> {
        // shift(k) then shift(DIM - k) recovers original.
        let a = BipolarVector::new_random()?;
        let k = 1337;
        assert_eq!(a.permute(k)?.permute(HD_DIMENSIONS - k)?, a);
        Ok(())
    }

    #[test]
    fn test_permute_composition() -> Result<(), HdcError> {
        // shift(a) . shift(b) == shift(a + b).
        let v = BipolarVector::new_random()?;
        assert_eq!(v.permute(100)?.permute(200)?, v.permute(300)?);
        Ok(())
    }

    #[test]
    fn test_permute_quasi_orthogonal() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let sim = a.permute(1)?.similarity(&a)?;
        assert!(sim.abs() < 0.1, "shift(1) sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_permute_preserves_hamming_weight() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let orig = a.count_ones();
        assert_eq!(a.permute(500)?.count_ones(), orig);
        Ok(())
    }

    #[test]
    fn test_permute_nontrivial_changes_vector() -> Result<(), HdcError> {
        // P(random vec is all-same-bit) = 2^{-9999} ~ 0.
        let a = BipolarVector::new_random()?;
        assert_ne!(a.permute(1)?, a);
        Ok(())
    }

    // --------------------------------------------------------
    // V. Similarity — Metric Proofs
    // --------------------------------------------------------

    #[test]
    fn test_similarity_self_is_one() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let sim = a.similarity(&a)?;
        assert!((sim - 1.0).abs() < f64::EPSILON, "self-sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_similarity_complement_is_neg_one() -> Result<(), HdcError> {
        let sim = BipolarVector::ones().similarity(&BipolarVector::zeros())?;
        assert!((sim - (-1.0)).abs() < f64::EPSILON, "complement-sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_similarity_symmetry() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert!((a.similarity(&b)? - b.similarity(&a)?).abs() < f64::EPSILON);
        Ok(())
    }

    #[test]
    fn test_similarity_random_near_zero() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert!(a.similarity(&b)?.abs() < 0.1);
        Ok(())
    }

    #[test]
    fn test_hamming_self_is_zero() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        assert_eq!(a.hamming_distance(&a)?, 0);
        Ok(())
    }

    #[test]
    fn test_hamming_complements_is_dim() -> Result<(), HdcError> {
        assert_eq!(
            BipolarVector::ones().hamming_distance(&BipolarVector::zeros())?,
            HD_DIMENSIONS
        );
        Ok(())
    }

    #[test]
    fn test_hamming_symmetry() -> Result<(), HdcError> {
        let a = BipolarVector::new_random()?;
        let b = BipolarVector::new_random()?;
        assert_eq!(a.hamming_distance(&b)?, b.hamming_distance(&a)?);
        Ok(())
    }

    // --------------------------------------------------------
    // VI. Cross-Operation — HDC Associative Memory Proofs
    // --------------------------------------------------------

    #[test]
    fn test_bind_bundle_kv_recovery() -> Result<(), HdcError> {
        // Classic HDC: C = bundle(K1 XOR V1, K2 XOR V2).
        // Query K1 XOR C should be more similar to V1 than V2.
        let k1 = BipolarVector::new_random()?;
        let v1 = BipolarVector::new_random()?;
        let k2 = BipolarVector::new_random()?;
        let v2 = BipolarVector::new_random()?;

        let pair1 = k1.bind(&v1)?;
        let pair2 = k2.bind(&v2)?;
        let memory = BipolarVector::bundle(&[&pair1, &pair2])?;

        let query = k1.bind(&memory)?;
        let sim_v1 = query.similarity(&v1)?;
        let sim_v2 = query.similarity(&v2)?;

        assert!(
            sim_v1 > sim_v2,
            "K1 query must recover V1 ({:.4}) over V2 ({:.4})", sim_v1, sim_v2
        );
        assert!(sim_v1 > 0.3, "Recovery signal must be strong: {:.4}", sim_v1);
        Ok(())
    }

    #[test]
    fn test_permute_sequence_encoding() -> Result<(), HdcError> {
        // Position-encoded variants must be mutually ~orthogonal.
        let base = BipolarVector::new_random()?;
        let p0 = base.permute(0)?;
        let p1 = base.permute(1)?;
        let p2 = base.permute(2)?;

        assert!(p0.similarity(&p1)?.abs() < 0.1, "pos0 vs pos1");
        assert!(p0.similarity(&p2)?.abs() < 0.1, "pos0 vs pos2");
        assert!(p1.similarity(&p2)?.abs() < 0.1, "pos1 vs pos2");
        Ok(())
    }

    #[test]
    fn test_bind_permute_combined_encoding() -> Result<(), HdcError> {
        // permute(role, pos) XOR filler at different positions -> orthogonal.
        let role = BipolarVector::new_random()?;
        let filler = BipolarVector::new_random()?;

        let enc0 = role.permute(0)?.bind(&filler)?;
        let enc1 = role.permute(1)?.bind(&filler)?;

        assert!(enc0.similarity(&enc1)?.abs() < 0.1, "pos0 vs pos1 encoding");
        Ok(())
    }
}
