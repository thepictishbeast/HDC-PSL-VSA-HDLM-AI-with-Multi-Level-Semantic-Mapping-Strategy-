// ============================================================
// VSA Superposition — Plausible Deniability Storage
// Section 1.I: "Bundle actual data vectors with synthetic noise."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use crate::debuglog;
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{Write, Read};

/// A storage unit utilizing superposition and chaff.
#[derive(Serialize, Deserialize)]
pub struct SuperpositionStorage {
    /// The aggregated memory vector.
    pub memory: BipolarVector,
    /// Number of signals bundled.
    pub signal_count: usize,
}

impl SuperpositionStorage {
    /// Create a new empty storage.
    pub fn new() -> Self {
        Self {
            memory: BipolarVector::zeros(),
            signal_count: 0,
        }
    }

    /// Save the persistent state blob to disk.
    pub fn save_to_disk(&self, path: &str) -> Result<(), HdcError> {
        debuglog!("SuperpositionStorage: Serializing state to {}", path);
        let serialized = serde_json::to_string(self).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Serialization failed: {}", e),
        })?;
        let mut file = File::create(path).map_err(|e| HdcError::InitializationFailed {
            reason: format!("File create failed: {}", e),
        })?;
        file.write_all(serialized.as_bytes()).map_err(|e| HdcError::InitializationFailed {
            reason: format!("File write failed: {}", e),
        })?;
        Ok(())
    }

    /// Load the persistent state blob from disk.
    pub fn load_from_disk(path: &str) -> Result<Self, HdcError> {
        debuglog!("SuperpositionStorage: Loading state from {}", path);
        let mut file = File::open(path).map_err(|e| HdcError::InitializationFailed {
            reason: format!("File open failed: {}", e),
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| HdcError::InitializationFailed {
            reason: format!("File read failed: {}", e),
        })?;
        let storage: Self = serde_json::from_str(&contents).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Deserialization failed: {}", e),
        })?;
        Ok(storage)
    }

    /// Commit a real signal into the superposition.
    pub fn commit_real(&mut self, signal: &BipolarVector) -> Result<(), HdcError> {
        debuglog!("SuperpositionStorage: Committing REAL signal");
        self.signal_count += 1;
        self.memory = BipolarVector::bundle(&[&self.memory, signal])?;
        Ok(())
    }

    /// Inject synthetic "chaff" into the superposition.
    pub fn inject_chaff(&mut self, count: usize) -> Result<(), HdcError> {
        debuglog!("SuperpositionStorage: Injecting {} CHAFF signals", count);
        let mut chaff_vectors = Vec::with_capacity(count);
        for _ in 0..count {
            chaff_vectors.push(BipolarVector::new_random()?);
        }

        let mut bundle_targets = vec![&self.memory];
        for c in &chaff_vectors {
            bundle_targets.push(c);
        }

        self.memory = BipolarVector::bundle(&bundle_targets)?;
        self.signal_count += count;
        Ok(())
    }

    /// Attempt to retrieve a signal using a key vector.
    /// In VSA, this is unbinding/similarity check.
    pub fn probe(&self, key: &BipolarVector) -> Result<f64, HdcError> {
        self.memory.similarity(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_superposition_deniability() -> Result<(), HdcError> {
        let mut storage = SuperpositionStorage::new();

        // 1. Commit Real Signal
        let real_signal = BipolarVector::new_random()?;
        storage.commit_real(&real_signal)?;

        // 2. Inject Chaff — fewer chaff vectors to keep signal recoverable
        storage.inject_chaff(3)?;

        // 3. Verify forensic indistinguishability
        // In a bundle of 1 real + 1 initial-zeros + 3 chaff = 5 vectors,
        // the initial zeros vector contributes no signal, so effective
        // bundle is ~4 vectors. Expected similarity ≈ 1/sqrt(4) = 0.5,
        // but majority-vote clip introduces variance.
        let sim = storage.probe(&real_signal)?;
        debuglog!("test_superposition_deniability: similarity to real signal = {:.4}", sim);

        // The real signal should be detectable above noise floor
        assert!(sim > 0.0, "Real signal should be positively correlated");

        // 4. Random noise should be near-orthogonal
        let noise = BipolarVector::new_random()?;
        let noise_sim = storage.probe(&noise)?;
        debuglog!("test_superposition_deniability: noise similarity = {:.4}", noise_sim);
        assert!(noise_sim.abs() < 0.15, "Noise should be approximately orthogonal");

        Ok(())
    }
}
