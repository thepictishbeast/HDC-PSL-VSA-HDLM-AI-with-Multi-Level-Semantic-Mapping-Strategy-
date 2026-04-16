// ============================================================
// Property-Based Tests for HDC Core (BipolarVector Algebra)
//
// AVP-2 Tier 6: Meta-validation via proptest
// Tests mathematical invariants of the hyperdimensional algebra:
//   - Binding (XOR): commutative, associative, self-inverse
//   - Bundling (Sum+Clip): commutative, similarity-preserving
//   - Permutation (Cyclic Shift): invertible, weight-preserving
//   - Similarity: bounds, self-similarity, orthogonality
// ============================================================

use proptest::prelude::*;
use lfi_vsa_core::hdc::vector::{BipolarVector, HD_DIMENSIONS};

// ============================================================
// Strategy: generate random BipolarVectors from u64 seeds
// ============================================================

fn arb_vector() -> impl Strategy<Value = BipolarVector> {
    any::<u64>().prop_map(BipolarVector::from_seed)
}

fn arb_shift() -> impl Strategy<Value = usize> {
    0..HD_DIMENSIONS
}

// ============================================================
// Binding (XOR) Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // BUG ASSUMPTION: XOR is commutative — if this fails, the XOR impl
    // is operating on misaligned memory or the operands differ in length.
    #[test]
    fn prop_bind_commutative(seed_a in any::<u64>(), seed_b in any::<u64>()) {
        let a = BipolarVector::from_seed(seed_a);
        let b = BipolarVector::from_seed(seed_b);
        let ab = a.bind(&b).unwrap();
        let ba = b.bind(&a).unwrap();
        prop_assert_eq!(ab, ba, "bind must be commutative");
    }

    // BUG ASSUMPTION: XOR is associative — (A⊕B)⊕C == A⊕(B⊕C).
    // Failure here means bitvec XOR has order-dependent side effects.
    #[test]
    fn prop_bind_associative(sa in any::<u64>(), sb in any::<u64>(), sc in any::<u64>()) {
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let c = BipolarVector::from_seed(sc);
        let ab_c = a.bind(&b).unwrap().bind(&c).unwrap();
        let a_bc = a.bind(&b.bind(&c).unwrap()).unwrap();
        prop_assert_eq!(ab_c, a_bc, "bind must be associative");
    }

    // BUG ASSUMPTION: A⊕A == identity (all-zeros bitvec, bipolar all -1).
    // Self-binding must produce the identity element.
    #[test]
    fn prop_bind_self_inverse(seed in any::<u64>()) {
        let a = BipolarVector::from_seed(seed);
        let aa = a.bind(&a).unwrap();
        let zeros = BipolarVector::zeros();
        prop_assert_eq!(aa, zeros, "A bind A must equal identity (all-zeros)");
    }

    // BUG ASSUMPTION: A⊕identity == A. The identity element (all-zeros)
    // must leave any vector unchanged.
    #[test]
    fn prop_bind_identity(seed in any::<u64>()) {
        let a = BipolarVector::from_seed(seed);
        let id = BipolarVector::zeros();
        let result = a.bind(&id).unwrap();
        prop_assert_eq!(result, a, "A bind identity must equal A");
    }

    // BUG ASSUMPTION: Binding preserves dimensionality.
    #[test]
    fn prop_bind_preserves_dim(sa in any::<u64>(), sb in any::<u64>()) {
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let ab = a.bind(&b).unwrap();
        prop_assert_eq!(ab.dim(), HD_DIMENSIONS, "bind must preserve dimensionality");
    }

    // BUG ASSUMPTION: A⊕B⊕B == A. Binding with B twice recovers A.
    #[test]
    fn prop_bind_double_cancel(sa in any::<u64>(), sb in any::<u64>()) {
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let abb = a.bind(&b).unwrap().bind(&b).unwrap();
        prop_assert_eq!(abb, a, "A bind B bind B must recover A");
    }
}

// ============================================================
// Permutation Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    // BUG ASSUMPTION: Permuting by 0 is identity.
    #[test]
    fn prop_permute_zero_is_identity(seed in any::<u64>()) {
        let a = BipolarVector::from_seed(seed);
        let p = a.permute(0).unwrap();
        prop_assert_eq!(p, a, "permute(0) must be identity");
    }

    // BUG ASSUMPTION: Permuting by DIM is identity (full cycle).
    #[test]
    fn prop_permute_full_cycle_is_identity(seed in any::<u64>()) {
        let a = BipolarVector::from_seed(seed);
        let p = a.permute(HD_DIMENSIONS).unwrap();
        prop_assert_eq!(p, a, "permute(DIM) must be identity");
    }

    // BUG ASSUMPTION: permute(k) composed with permute(DIM-k) is identity.
    // This proves invertibility.
    #[test]
    fn prop_permute_inverse(seed in any::<u64>(), shift in 1..HD_DIMENSIONS) {
        let a = BipolarVector::from_seed(seed);
        let forward = a.permute(shift).unwrap();
        let back = forward.permute(HD_DIMENSIONS - shift).unwrap();
        prop_assert_eq!(back, a, "permute(k) then permute(DIM-k) must recover original");
    }

    // BUG ASSUMPTION: Permutation preserves Hamming weight (count of +1 bits).
    // Cyclic shift moves bits around but never creates or destroys them.
    #[test]
    fn prop_permute_preserves_weight(seed in any::<u64>(), shift in arb_shift()) {
        let a = BipolarVector::from_seed(seed);
        let p = a.permute(shift).unwrap();
        prop_assert_eq!(
            a.count_ones(), p.count_ones(),
            "permute must preserve Hamming weight (ones count)"
        );
    }

    // BUG ASSUMPTION: Permutation preserves dimensionality.
    #[test]
    fn prop_permute_preserves_dim(seed in any::<u64>(), shift in arb_shift()) {
        let a = BipolarVector::from_seed(seed);
        let p = a.permute(shift).unwrap();
        prop_assert_eq!(p.dim(), HD_DIMENSIONS, "permute must preserve dimensionality");
    }

    // BUG ASSUMPTION: permute(a) composed with permute(b) == permute(a+b).
    // Composition of shifts must equal the combined shift.
    #[test]
    fn prop_permute_composition(
        seed in any::<u64>(),
        shift_a in 0..HD_DIMENSIONS,
        shift_b in 0..HD_DIMENSIONS
    ) {
        let v = BipolarVector::from_seed(seed);
        let step1 = v.permute(shift_a).unwrap().permute(shift_b).unwrap();
        let combined = v.permute((shift_a + shift_b) % HD_DIMENSIONS).unwrap();
        prop_assert_eq!(step1, combined, "permute(a) then permute(b) must equal permute(a+b mod DIM)");
    }
}

// ============================================================
// Similarity Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // BUG ASSUMPTION: Self-similarity is exactly 1.0.
    #[test]
    fn prop_self_similarity_is_one(seed in any::<u64>()) {
        let a = BipolarVector::from_seed(seed);
        let sim = a.similarity(&a).unwrap();
        prop_assert!((sim - 1.0).abs() < 1e-12, "self-similarity must be 1.0, got {}", sim);
    }

    // BUG ASSUMPTION: Similarity is bounded [-1, 1].
    #[test]
    fn prop_similarity_bounded(sa in any::<u64>(), sb in any::<u64>()) {
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let sim = a.similarity(&b).unwrap();
        prop_assert!(sim >= -1.0 && sim <= 1.0,
            "similarity must be in [-1, 1], got {}", sim);
    }

    // BUG ASSUMPTION: Similarity is symmetric.
    #[test]
    fn prop_similarity_symmetric(sa in any::<u64>(), sb in any::<u64>()) {
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let ab = a.similarity(&b).unwrap();
        let ba = b.similarity(&a).unwrap();
        prop_assert!((ab - ba).abs() < 1e-12,
            "similarity must be symmetric: sim(a,b)={} != sim(b,a)={}", ab, ba);
    }

    // BUG ASSUMPTION: Complement has similarity -1.0.
    // If A is all-1s and B is all-0s (bipolar complement), cosine = -1.
    #[test]
    fn prop_complement_similarity_is_neg_one(_dummy in 0u8..1u8) {
        let ones = BipolarVector::ones();
        let zeros = BipolarVector::zeros();
        let sim = ones.similarity(&zeros).unwrap();
        prop_assert!((sim - (-1.0)).abs() < 1e-12,
            "ones vs zeros (complement) must have similarity -1.0, got {}", sim);
    }

    // BUG ASSUMPTION: Random vectors in 10000-D are quasi-orthogonal.
    // For truly random vectors, |cos| should be < 0.05 with very high probability.
    #[test]
    fn prop_random_quasi_orthogonal(sa in any::<u64>(), sb in any::<u64>()) {
        prop_assume!(sa != sb);
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let sim = a.similarity(&b).unwrap();
        // In 10000-D, random cosine similarity has std dev ~ 1/sqrt(10000) = 0.01
        // 5 sigma = 0.05, so |sim| < 0.1 is extremely conservative.
        prop_assert!(sim.abs() < 0.1,
            "random 10000-D vectors should be quasi-orthogonal, got similarity {}", sim);
    }
}

// ============================================================
// Bundling Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    // BUG ASSUMPTION: Bundle of a single vector is that vector.
    #[test]
    fn prop_bundle_single_is_identity(seed in any::<u64>()) {
        let a = BipolarVector::from_seed(seed);
        let bundled = BipolarVector::bundle(&[&a]).unwrap();
        prop_assert_eq!(bundled, a, "bundle of one vector must return that vector");
    }

    // BUG ASSUMPTION: Bundle preserves dimensionality.
    #[test]
    fn prop_bundle_preserves_dim(sa in any::<u64>(), sb in any::<u64>(), sc in any::<u64>()) {
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let c = BipolarVector::from_seed(sc);
        let bundled = BipolarVector::bundle(&[&a, &b, &c]).unwrap();
        prop_assert_eq!(bundled.dim(), HD_DIMENSIONS, "bundle must preserve dimensionality");
    }

    // BUG ASSUMPTION: Bundle is commutative (order doesn't matter for 3 vectors).
    // Majority vote is order-independent.
    #[test]
    fn prop_bundle_commutative(sa in any::<u64>(), sb in any::<u64>(), sc in any::<u64>()) {
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let c = BipolarVector::from_seed(sc);
        let abc = BipolarVector::bundle(&[&a, &b, &c]).unwrap();
        let bca = BipolarVector::bundle(&[&b, &c, &a]).unwrap();
        let cab = BipolarVector::bundle(&[&c, &a, &b]).unwrap();
        prop_assert_eq!(abc.clone(), bca, "bundle must be commutative (abc vs bca)");
        prop_assert_eq!(abc, cab, "bundle must be commutative (abc vs cab)");
    }

    // BUG ASSUMPTION: Bundle result is similar to all input vectors.
    // The superposition should have positive cosine with each constituent.
    #[test]
    fn prop_bundle_similar_to_inputs(sa in any::<u64>(), sb in any::<u64>(), sc in any::<u64>()) {
        prop_assume!(sa != sb && sb != sc && sa != sc);
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let c = BipolarVector::from_seed(sc);
        let bundled = BipolarVector::bundle(&[&a, &b, &c]).unwrap();
        let sim_a = bundled.similarity(&a).unwrap();
        let sim_b = bundled.similarity(&b).unwrap();
        let sim_c = bundled.similarity(&c).unwrap();
        // Each input should contribute positively to the bundle.
        // For 3 random vectors, each should have ~0.33 similarity with the bundle.
        prop_assert!(sim_a > 0.0, "bundle must be positively similar to input A, got {}", sim_a);
        prop_assert!(sim_b > 0.0, "bundle must be positively similar to input B, got {}", sim_b);
        prop_assert!(sim_c > 0.0, "bundle must be positively similar to input C, got {}", sim_c);
    }

    // BUG ASSUMPTION: Dominant vector in bundle is most similar to result.
    // Bundling [A, A, A, B] should be more similar to A than to B.
    #[test]
    fn prop_bundle_dominant_vector(sa in any::<u64>(), sb in any::<u64>()) {
        prop_assume!(sa != sb);
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let bundled = BipolarVector::bundle(&[&a, &a, &a, &b]).unwrap();
        let sim_a = bundled.similarity(&a).unwrap();
        let sim_b = bundled.similarity(&b).unwrap();
        prop_assert!(sim_a > sim_b,
            "dominant vector A (3 copies) must be more similar than B (1 copy): sim_a={}, sim_b={}",
            sim_a, sim_b);
    }
}

// ============================================================
// Cross-Operation Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    // BUG ASSUMPTION: Binding produces a quasi-orthogonal result.
    // XOR of two random vectors should be quasi-orthogonal to both.
    #[test]
    fn prop_bind_produces_orthogonal(sa in any::<u64>(), sb in any::<u64>()) {
        prop_assume!(sa != sb);
        let a = BipolarVector::from_seed(sa);
        let b = BipolarVector::from_seed(sb);
        let ab = a.bind(&b).unwrap();
        let sim_a = ab.similarity(&a).unwrap();
        let sim_b = ab.similarity(&b).unwrap();
        // Bound vector should be quasi-orthogonal to both inputs.
        prop_assert!(sim_a.abs() < 0.1,
            "A bind B should be quasi-orthogonal to A, got similarity {}", sim_a);
        prop_assert!(sim_b.abs() < 0.1,
            "A bind B should be quasi-orthogonal to B, got similarity {}", sim_b);
    }

    // BUG ASSUMPTION: Permutation produces quasi-orthogonal for non-trivial shifts.
    // Even a shift of 1 should destroy correlation in high-D.
    #[test]
    fn prop_permute_non_trivial_orthogonal(seed in any::<u64>(), shift in 1..HD_DIMENSIONS-1) {
        let a = BipolarVector::from_seed(seed);
        let p = a.permute(shift).unwrap();
        let sim = a.similarity(&p).unwrap();
        // For large D and non-trivial shift, similarity should be near 0.
        // Allow a generous tolerance since small shifts might correlate slightly.
        prop_assert!(sim.abs() < 0.15,
            "permute({}) should produce quasi-orthogonal vector, got similarity {}", shift, sim);
    }

    // BUG ASSUMPTION: from_seed is deterministic.
    // Same seed must always produce the same vector.
    #[test]
    fn prop_from_seed_deterministic(seed in any::<u64>()) {
        let a = BipolarVector::from_seed(seed);
        let b = BipolarVector::from_seed(seed);
        prop_assert_eq!(a, b, "from_seed must be deterministic");
    }

    // BUG ASSUMPTION: Random vectors are balanced (Hamming weight ≈ DIM/2).
    // For 10000-D, expect ~5000 ones with std dev ~50.
    #[test]
    fn prop_random_balanced(seed in any::<u64>()) {
        let v = BipolarVector::from_seed(seed);
        let ones = v.count_ones();
        let expected = HD_DIMENSIONS / 2;
        // 10 sigma: 10 * sqrt(10000/4) = 10 * 50 = 500
        let tolerance = 500;
        prop_assert!(
            (ones as i64 - expected as i64).unsigned_abs() < tolerance as u64,
            "random vector ones count {} is too far from expected {} (tolerance {})",
            ones, expected, tolerance
        );
    }
}
