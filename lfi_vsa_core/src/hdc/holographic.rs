// ============================================================
// VSA Holographic Memory — High-Dimensional Associative Storage
// Section 1.III: "Retrieval is O(1) complexity... stored in
// high-dimensional superposition."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use crate::debuglog;
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
        
        // Retrieve
        let retrieved = mem.probe(&key)?;
        let sim = retrieved.similarity(&value)?;
        
        debuglog!("test_holographic_retrieval: Similarity = {:.4}", sim);
        assert!(sim > 0.5, "Holographic retrieval should yield high similarity");
        Ok(())
    }
}
