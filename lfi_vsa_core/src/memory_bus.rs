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
