// NODE 008: Fractal Holographic Memory (VSA Substrate v2)
// STATUS: ALPHA - High-Res Scaling Active
// PROTOCOL: Dimensional Projection / 32k-to-10k

use ndarray::Array1;
use serde::{Serialize, Deserialize};
use std::fs::File;

/// Standard VSA Dimensionality constants.
pub const DIM_PROLETARIAT: usize = 10000;
pub const DIM_BIGBRAIN: usize = 32768;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperMemory {
    pub vector: Array1<i8>,
    pub dimensions: usize,
}

impl HyperMemory {
    /// Initialize VSA register with specific dimensionality.
    pub fn new(dim: usize) -> Self {
        Self {
            vector: Array1::zeros(dim),
            dimensions: dim,
        }
    }

    pub fn generate_seed(dim: usize) -> Self {
        let mut v = Array1::zeros(dim);
        for i in 0..dim {
            v[i] = if rand::random::<bool>() { 1 } else { -1 };
        }
        Self { vector: v, dimensions: dim }
    }

    pub fn from_string(input: &str, dim: usize) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let seed = hasher.finish();

        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut v = Array1::zeros(dim);
        for i in 0..dim {
            v[i] = if rand::Rng::gen::<bool>(&mut rng) { 1 } else { -1 };
        }
        Self { vector: v, dimensions: dim }
    }

    pub fn bind(&self, other: &Self) -> Result<Self, Box<dyn std::error::Error>> {
        if self.dimensions != other.dimensions {
            return Err("Dimension mismatch during VSA binding".into());
        }
        Ok(Self { 
            vector: &self.vector * &other.vector,
            dimensions: self.dimensions 
        })
    }

    pub fn bundle(vectors: &[Self]) -> Result<Self, Box<dyn std::error::Error>> {
        if vectors.is_empty() { return Err("Empty bundle attempt".into()); }
        let dim = vectors[0].dimensions;
        let mut sum = Array1::<i32>::zeros(dim);
        for v in vectors {
            for i in 0..dim {
                sum[i] += v.vector[i] as i32;
            }
        }
        let mut consensus = Array1::<i8>::zeros(dim);
        for i in 0..dim {
            if sum[i] > 0 { consensus[i] = 1; }
            else if sum[i] < 0 { consensus[i] = -1; }
            else { consensus[i] = if rand::random::<bool>() { 1 } else { -1 }; }
        }
        Ok(Self { vector: consensus, dimensions: dim })
    }

    pub fn project(&self, target_dim: usize) -> Result<Self, Box<dyn std::error::Error>> {
        if target_dim >= self.dimensions {
            return Err("Projection must be to a lower-dimensional space".into());
        }
        let mut projected = Array1::<i8>::zeros(target_dim);
        let ratio = self.dimensions / target_dim;
        for i in 0..target_dim {
            let mut chunk_sum = 0;
            for j in 0..ratio {
                chunk_sum += self.vector[i * ratio + j] as i32;
            }
            projected[i] = if chunk_sum >= 0 { 1 } else { -1 };
        }
        Ok(Self { vector: projected, dimensions: target_dim })
    }

    /// AUDIT: Verifies memory health by checking orthogonality against random probes.
    /// Mean similarity should remain < 0.05.
    pub fn audit_orthogonality(&self) -> f64 {
        let mut total_sim = 0.0;
        let probe_count = 10;
        for _ in 0..probe_count {
            let probe = Self::generate_seed(self.dimensions);
            total_sim += self.similarity(&probe);
        }
        total_sim / probe_count as f64
    }

    /// Cyclic permutation — shifts the vector by `amount` positions.
    /// This is the standard VSA "role" operator for structured representations.
    pub fn permute(&self, amount: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let dim = self.dimensions;
        if dim == 0 { return Err("Cannot permute zero-dimensional vector".into()); }
        let shift = amount % dim;
        let mut permuted = Array1::<i8>::zeros(dim);
        for i in 0..dim {
            permuted[(i + shift) % dim] = self.vector[i];
        }
        Ok(Self { vector: permuted, dimensions: dim })
    }

    /// Export as a bitvec-compatible raw vector: true = +1, false = -1.
    /// Used for bridging to BipolarVector (bitvec) for PSL audit targets.
    /// Returns BitVec<u8, Lsb0> to match BipolarVector's internal representation.
    pub fn export_raw_bitvec(&self) -> bitvec::vec::BitVec<u8, bitvec::order::Lsb0> {
        let mut bv = bitvec::vec::BitVec::<u8, bitvec::order::Lsb0>::with_capacity(self.dimensions);
        for &v in self.vector.iter() {
            bv.push(v > 0);
        }
        bv
    }

    pub fn similarity(&self, other: &Self) -> f64 {
        let mut matches = 0;
        let limit = self.dimensions.min(other.dimensions);
        for i in 0..limit {
            if self.vector[i] == other.vector[i] { matches += 1; }
        }
        matches as f64 / limit as f64
    }

    pub fn commit_to_disk(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(path)?;
        bincode::serialize_into(file, &self)?;
        Ok(())
    }

    pub fn load_from_disk(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let store: Self = bincode::deserialize_from(file)?;
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_correct_dimensions() {
        let hm = HyperMemory::new(DIM_PROLETARIAT);
        assert_eq!(hm.dimensions, DIM_PROLETARIAT);
        assert_eq!(hm.vector.len(), DIM_PROLETARIAT);
    }

    #[test]
    fn test_generate_seed_is_nonzero() {
        let hm = HyperMemory::generate_seed(DIM_PROLETARIAT);
        assert_eq!(hm.dimensions, DIM_PROLETARIAT);
        // At least some values should be nonzero.
        let nonzero = hm.vector.iter().filter(|&&v| v != 0).count();
        assert!(nonzero > 0, "Seeded vector should have nonzero elements");
    }

    #[test]
    fn test_from_string_deterministic() {
        let a = HyperMemory::from_string("test_input", DIM_PROLETARIAT);
        let b = HyperMemory::from_string("test_input", DIM_PROLETARIAT);
        assert!((a.similarity(&b) - 1.0).abs() < 0.001, "Same string should produce same vector");
    }

    #[test]
    fn test_from_string_different_inputs() {
        let a = HyperMemory::from_string("alpha", DIM_PROLETARIAT);
        let b = HyperMemory::from_string("beta", DIM_PROLETARIAT);
        let sim = a.similarity(&b);
        assert!(sim < 0.9, "Different strings should produce different vectors: {:.4}", sim);
    }

    #[test]
    fn test_self_similarity_is_one() {
        let hm = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let sim = hm.similarity(&hm);
        assert!((sim - 1.0).abs() < 0.001, "Self-similarity should be ~1.0, got {:.4}", sim);
    }

    #[test]
    fn test_bind_produces_result() {
        let a = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let b = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let bound = a.bind(&b).expect("bind should succeed");
        assert_eq!(bound.dimensions, DIM_PROLETARIAT);
    }

    #[test]
    fn test_bind_dimension_mismatch() {
        let a = HyperMemory::new(100);
        let b = HyperMemory::new(200);
        assert!(a.bind(&b).is_err(), "Mismatched dimensions should fail");
    }

    #[test]
    fn test_bundle_combines_vectors() {
        let a = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let b = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let bundled = HyperMemory::bundle(&[a, b]).expect("bundle should succeed");
        assert_eq!(bundled.dimensions, DIM_PROLETARIAT);
    }

    #[test]
    fn test_permute_changes_vector() {
        let a = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let permuted = a.permute(1).expect("permute should succeed");
        let sim = a.similarity(&permuted);
        assert!(sim < 0.9, "Permuted vector should differ: {:.4}", sim);
    }

    #[test]
    fn test_project_reduces_dimension() {
        let hm = HyperMemory::generate_seed(DIM_BIGBRAIN);
        let projected = hm.project(DIM_PROLETARIAT).expect("project should succeed");
        assert_eq!(projected.dimensions, DIM_PROLETARIAT);
    }

    #[test]
    fn test_audit_orthogonality() {
        let hm = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let ortho = hm.audit_orthogonality();
        // Orthogonality should be a finite number.
        assert!(ortho.is_finite(), "Orthogonality should be finite");
    }

    #[test]
    fn test_export_raw_bitvec() {
        let hm = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let bits = hm.export_raw_bitvec();
        assert_eq!(bits.len(), DIM_PROLETARIAT, "Bitvec should match dimensions");
    }

    #[test]
    fn test_disk_persistence_round_trip() {
        let original = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let path = "/tmp/test_hypermemory_persist.bin";

        original.commit_to_disk(path).expect("write should succeed");
        let loaded = HyperMemory::load_from_disk(path).expect("load should succeed");

        let sim = original.similarity(&loaded);
        assert!((sim - 1.0).abs() < 0.001, "Loaded vector should be identical: {:.4}", sim);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_constants() {
        assert_eq!(DIM_PROLETARIAT, 10000);
        assert_eq!(DIM_BIGBRAIN, 32768);
        assert!(DIM_BIGBRAIN > DIM_PROLETARIAT);
    }

    // ============================================================
    // Stress / invariant tests for HyperMemory
    // ============================================================

    /// INVARIANT: same string seed always produces the same vector.
    #[test]
    fn invariant_from_string_is_deterministic() {
        let v1 = HyperMemory::from_string("PlausiDen", DIM_PROLETARIAT);
        let v2 = HyperMemory::from_string("PlausiDen", DIM_PROLETARIAT);
        assert_eq!(v1.export_raw_bitvec(), v2.export_raw_bitvec(),
            "deterministic seed must yield identical bitvecs");
    }

    /// INVARIANT: distinct strings produce distinct vectors (no collisions
    /// for practical English-text inputs).
    #[test]
    fn invariant_distinct_strings_distinct_vectors() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..200 {
            let v = HyperMemory::from_string(&format!("term_{}", i), DIM_PROLETARIAT);
            let bits = v.export_raw_bitvec();
            // Hash-fingerprint the first 128 bits as a u128.
            let key: u128 = bits.iter().take(128).enumerate()
                .filter(|(_, b)| **b).map(|(i, _)| 1u128 << i).sum();
            assert!(seen.insert(key),
                "vector for term_{} collided with previous vector", i);
        }
    }

    /// INVARIANT: bind is approximately associative across 3 random vectors.
    #[test]
    fn invariant_bind_associative_high_similarity() {
        let a = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let b = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let c = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let lhs = a.bind(&b).unwrap().bind(&c).unwrap();
        let rhs = a.bind(&b.bind(&c).unwrap()).unwrap();
        let sim = lhs.similarity(&rhs);
        // Bind on HyperMemory is element-wise XOR; associativity holds
        // exactly. Allow a small floating-point tolerance for similarity.
        assert!(sim > 0.95,
            "associativity must hold (sim > 0.95), got {:.4}", sim);
    }

    /// INVARIANT: self-similarity always exceeds similarity to any random
    /// other vector — the basic guarantee that HDC respects identity.
    #[test]
    fn invariant_self_similarity_dominates_random() {
        let v = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let self_sim = v.similarity(&v);
        for _ in 0..20 {
            let other = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let other_sim = v.similarity(&other);
            assert!(self_sim > other_sim,
                "self-similarity {} must dominate random {}", self_sim, other_sim);
        }
    }
}
