// ============================================================
// VSA Holographic Memory — High-Dimensional Associative Storage
// Section 1.III: "Retrieval is O(1) complexity... stored in
// high-dimensional superposition."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use std::collections::HashMap;

/// A Holographic Memory unit using VSA.
/// Maps keys (vectors) to values (vectors) in a single superimposed object.
pub struct HolographicMemory {
    /// The holographic sum of all Key XOR Value pairs.
    pub storage: BipolarVector,
    /// Metadata to track the number of bundled associations.
    pub capacity: usize,
    /// Fast-lookup clean-room for Sovereign verification (Optional).
    pub registry: HashMap<u64, String>,
}

impl HolographicMemory {
    pub fn new() -> Self {
        Self {
            storage: BipolarVector::zeros(),
            capacity: 0,
            registry: HashMap::new(),
        }
    }

    /// Store an association (Key, Value) into the holographic space.
    /// Memory = Bundle(Memory, Key XOR Value)
    pub fn associate(&mut self, key: &BipolarVector, value: &BipolarVector) -> Result<(), HdcError> {
        debuglog!("HolographicMemory: Associating new hypervector pair");
        
        // 1. Bind key and value (XOR)
        let bound = key.bind(value)?;
        
        // 2. Bundle into the holographic sum
        let mut bundle_targets = vec![&self.storage];
        if self.capacity > 0 {
            bundle_targets.push(&bound);
            self.storage = BipolarVector::bundle(&bundle_targets)?;
        } else {
            self.storage = bound;
        }
        
        self.capacity += 1;
        Ok(())
    }

    /// Retrieve a value from the holographic space using a key.
    /// Retrieval = Memory XOR Key -> Similarity check against possible values.
    pub fn probe(&self, key: &BipolarVector) -> Result<BipolarVector, HdcError> {
        debuglog!("HolographicMemory: Probing memory with key (O(1) complexity)");
        // XOR is self-inverse: (Key XOR Value) XOR Key = Value
        self.storage.bind(key)
    }

    /// Quantifies the "Logic Flux" or noise level in the holographic space.
    pub fn logic_flux(&self) -> Result<f64, HdcError> {
        debuglog!("HolographicMemory::logic_flux: capacity={}", self.capacity);
        let ones = self.storage.count_ones() as f64;
        let dim = crate::hdc::vector::HD_DIMENSIONS as f64;
        Ok((ones / dim - 0.5).abs())
    }

    /// Estimate the theoretical retrieval quality at current capacity.
    ///
    /// With N associations in D dimensions, expected retrieval similarity is:
    ///   sim ≈ 1/sqrt(N) for random vectors (theoretical bound)
    ///
    /// Returns (estimated_quality, theoretical_max_capacity).
    pub fn capacity_estimate(&self) -> (f64, usize) {
        let _dim = crate::hdc::vector::HD_DIMENSIONS as f64;
        let n = self.capacity.max(1) as f64;

        // Theoretical retrieval quality degrades as 1/sqrt(N).
        let quality = 1.0 / n.sqrt();

        // Max capacity before quality drops below 0.1.
        // 0.1 = 1/sqrt(N_max) → N_max = 100
        // In practice, HDC with 10k dims supports ~100-500 clean associations.
        let max_capacity = (1.0 / 0.1_f64).powi(2) as usize; // 100

        debuglog!("HolographicMemory::capacity_estimate: quality={:.4}, max={}", quality, max_capacity);
        (quality, max_capacity)
    }

    /// Check if the memory is approaching capacity (quality degrading).
    pub fn is_near_capacity(&self) -> bool {
        let (quality, _) = self.capacity_estimate();
        quality < 0.15 // Below 15% quality = near capacity
    }

    /// Number of stored associations.
    pub fn association_count(&self) -> usize {
        self.capacity
    }

    /// Clear the memory and reset to empty state.
    pub fn clear(&mut self) {
        debuglog!("HolographicMemory::clear: Resetting memory");
        self.storage = BipolarVector::zeros();
        self.capacity = 0;
        self.registry.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holographic_retrieval() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        let key = BipolarVector::new_random()?;
        let value = BipolarVector::new_random()?;
        mem.associate(&key, &value)?;
        let retrieved = mem.probe(&key)?;
        let sim = retrieved.similarity(&value)?;
        assert!(sim > 0.5, "Holographic retrieval should yield high similarity: {:.4}", sim);
        Ok(())
    }

    #[test]
    fn test_empty_memory() {
        let mem = HolographicMemory::new();
        assert_eq!(mem.association_count(), 0);
        assert!(!mem.is_near_capacity());
    }

    #[test]
    fn test_multiple_associations() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        for _ in 0..5 {
            let k = BipolarVector::new_random()?;
            let v = BipolarVector::new_random()?;
            mem.associate(&k, &v)?;
        }
        assert_eq!(mem.association_count(), 5);
        Ok(())
    }

    #[test]
    fn test_logic_flux_starts_low() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        let k = BipolarVector::new_random()?;
        let v = BipolarVector::new_random()?;
        mem.associate(&k, &v)?;
        let flux = mem.logic_flux()?;
        assert!(flux < 0.2, "Single association should have low flux: {:.4}", flux);
        Ok(())
    }

    #[test]
    fn test_capacity_estimate_decreases_with_load() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        let k = BipolarVector::new_random()?;
        let v = BipolarVector::new_random()?;
        mem.associate(&k, &v)?;
        let (q1, _) = mem.capacity_estimate();

        // Add more associations.
        for _ in 0..20 {
            let k2 = BipolarVector::new_random()?;
            let v2 = BipolarVector::new_random()?;
            mem.associate(&k2, &v2)?;
        }
        let (q2, _) = mem.capacity_estimate();
        assert!(q2 < q1, "Quality should decrease with more associations: {:.4} vs {:.4}", q2, q1);
        Ok(())
    }

    #[test]
    fn test_clear_resets_state() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        for _ in 0..10 {
            let k = BipolarVector::new_random()?;
            let v = BipolarVector::new_random()?;
            mem.associate(&k, &v)?;
        }
        assert_eq!(mem.association_count(), 10);
        mem.clear();
        assert_eq!(mem.association_count(), 0);
        Ok(())
    }

    #[test]
    fn test_near_capacity_detection() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        // Fill with many associations to approach capacity.
        for _ in 0..200 {
            let k = BipolarVector::new_random()?;
            let v = BipolarVector::new_random()?;
            mem.associate(&k, &v)?;
        }
        assert!(mem.is_near_capacity(), "200 associations should approach capacity");
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for HolographicMemory
    // ============================================================

    /// INVARIANT: association_count grows by exactly 1 per associate() call.
    #[test]
    fn invariant_association_count_monotonic() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        for i in 0..20 {
            let k = BipolarVector::new_random()?;
            let v = BipolarVector::new_random()?;
            let before = mem.association_count();
            mem.associate(&k, &v)?;
            assert_eq!(mem.association_count(), before + 1,
                "count must grow by 1 at iter {}", i);
        }
        Ok(())
    }

    /// INVARIANT: clear() resets association_count to zero.
    #[test]
    fn invariant_clear_zeros_count() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        for _ in 0..10 {
            let k = BipolarVector::new_random()?;
            let v = BipolarVector::new_random()?;
            mem.associate(&k, &v)?;
        }
        assert_eq!(mem.association_count(), 10);
        mem.clear();
        assert_eq!(mem.association_count(), 0,
            "clear must reset count to 0");
        Ok(())
    }

    /// INVARIANT: capacity_estimate ratio is in [0.0, 1.0+epsilon].
    #[test]
    fn invariant_capacity_estimate_ratio_in_unit_interval() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        for i in 0..150 {
            if i > 0 {
                let k = BipolarVector::new_random()?;
                let v = BipolarVector::new_random()?;
                mem.associate(&k, &v)?;
            }
            let (ratio, _capacity) = mem.capacity_estimate();
            assert!(ratio >= 0.0 && ratio <= 1.0 + 1e-6,
                "capacity ratio out of range at i={}: {}", i, ratio);
        }
        Ok(())
    }

    /// INVARIANT: logic_flux stays finite (no NaN/Inf) after stress.
    #[test]
    fn invariant_logic_flux_finite() -> Result<(), HdcError> {
        let mut mem = HolographicMemory::new();
        for _ in 0..100 {
            let k = BipolarVector::new_random()?;
            let v = BipolarVector::new_random()?;
            mem.associate(&k, &v)?;
        }
        let f = mem.logic_flux()?;
        assert!(f.is_finite(),
            "logic_flux must stay finite, got {}", f);
        Ok(())
    }

    /// INVARIANT: probing an empty memory does not panic.
    #[test]
    fn invariant_probe_empty_memory_safe() -> Result<(), HdcError> {
        let mem = HolographicMemory::new();
        let key = BipolarVector::new_random()?;
        let _ = mem.probe(&key); // Just must not panic.
        Ok(())
    }
}
