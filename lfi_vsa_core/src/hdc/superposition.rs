// ============================================================
// VSA Superposition — Plausible Deniability Storage
// Section 1.I: "Bundle actual data vectors with synthetic noise."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
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
    /// The first signal is stored directly; subsequent signals are bundled.
    pub fn commit_real(&mut self, signal: &BipolarVector) -> Result<(), HdcError> {
        debuglog!("SuperpositionStorage: Committing REAL signal (count={})", self.signal_count);
        if self.signal_count == 0 {
            // First signal: store directly to avoid zeros-bias from tie-breaking
            self.memory = signal.clone();
        } else {
            self.memory = BipolarVector::bundle(&[&self.memory, signal])?;
        }
        self.signal_count += 1;
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

    #[test]
    fn test_empty_storage() {
        let storage = SuperpositionStorage::new();
        assert_eq!(storage.signal_count, 0);
    }

    #[test]
    fn test_commit_real_increments_count() -> Result<(), HdcError> {
        let mut storage = SuperpositionStorage::new();
        let sig = BipolarVector::new_random()?;
        storage.commit_real(&sig)?;
        assert_eq!(storage.signal_count, 1);
        storage.commit_real(&BipolarVector::new_random()?)?;
        assert_eq!(storage.signal_count, 2);
        Ok(())
    }

    #[test]
    fn test_chaff_injection_count() -> Result<(), HdcError> {
        let mut storage = SuperpositionStorage::new();
        let sig = BipolarVector::new_random()?;
        storage.commit_real(&sig)?;
        storage.inject_chaff(10)?;
        assert_eq!(storage.signal_count, 11); // 1 real + 10 chaff
        Ok(())
    }

    #[test]
    fn test_disk_persistence() -> Result<(), HdcError> {
        let mut storage = SuperpositionStorage::new();
        let sig = BipolarVector::new_random()?;
        storage.commit_real(&sig)?;

        let path = "/tmp/test_superposition_persist.json";
        storage.save_to_disk(path)?;

        let loaded = SuperpositionStorage::load_from_disk(path)?;
        assert_eq!(loaded.signal_count, 1);

        // Probing should give similar results.
        let original_sim = storage.probe(&sig)?;
        let loaded_sim = loaded.probe(&sig)?;
        assert!((original_sim - loaded_sim).abs() < 0.01,
            "Loaded storage should produce same probe results: {:.4} vs {:.4}", original_sim, loaded_sim);

        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[test]
    fn test_heavy_chaff_obscures_signal() -> Result<(), HdcError> {
        let mut storage = SuperpositionStorage::new();
        let sig = BipolarVector::new_random()?;
        storage.commit_real(&sig)?;

        // Heavy chaff should reduce signal detectability.
        storage.inject_chaff(50)?;
        let sim = storage.probe(&sig)?;
        debuglog!("test_heavy_chaff: similarity after 50 chaff = {:.4}", sim);
        // Signal should still be above noise floor but attenuated.
        // With 51 vectors bundled, expected similarity is ~1/sqrt(51) ≈ 0.14
        assert!(sim.abs() < 0.5, "Heavy chaff should attenuate signal: {:.4}", sim);
        Ok(())
    }

    #[test]
    fn test_multiple_real_signals_detectable() -> Result<(), HdcError> {
        let mut storage = SuperpositionStorage::new();
        let sig1 = BipolarVector::new_random()?;
        let sig2 = BipolarVector::new_random()?;
        storage.commit_real(&sig1)?;
        storage.commit_real(&sig2)?;

        // Both signals should be detectable.
        let sim1 = storage.probe(&sig1)?;
        let sim2 = storage.probe(&sig2)?;
        assert!(sim1 > 0.0, "First signal should be detectable: {:.4}", sim1);
        assert!(sim2 > 0.0, "Second signal should be detectable: {:.4}", sim2);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for SuperpositionStorage
    // ============================================================

    /// INVARIANT: signal_count grows by exactly 1 per commit_real call.
    #[test]
    fn invariant_signal_count_monotonic_on_commit() -> Result<(), HdcError> {
        let mut s = SuperpositionStorage::new();
        for i in 0..15 {
            let before = s.signal_count;
            let v = BipolarVector::new_random()?;
            s.commit_real(&v)?;
            assert_eq!(s.signal_count, before + 1,
                "count must grow by 1 per commit at iter {}", i);
        }
        Ok(())
    }

    /// INVARIANT: inject_chaff(n) grows signal_count by exactly n.
    #[test]
    fn invariant_chaff_grows_count_by_n() -> Result<(), HdcError> {
        let mut s = SuperpositionStorage::new();
        for n in [1, 5, 20] {
            let before = s.signal_count;
            s.inject_chaff(n)?;
            assert_eq!(s.signal_count, before + n,
                "inject_chaff({}) must grow count by {}", n, n);
        }
        Ok(())
    }

    /// INVARIANT: probing an empty storage doesn't panic and returns a finite f64.
    #[test]
    fn invariant_probe_empty_finite() -> Result<(), HdcError> {
        let s = SuperpositionStorage::new();
        let key = BipolarVector::new_random()?;
        let sim = s.probe(&key)?;
        assert!(sim.is_finite(), "empty probe must return finite, got {}", sim);
        Ok(())
    }

    /// INVARIANT: probe similarity is in [-1.0, 1.0] after stress.
    #[test]
    fn invariant_probe_similarity_in_cosine_range() -> Result<(), HdcError> {
        let mut s = SuperpositionStorage::new();
        for _ in 0..50 {
            s.commit_real(&BipolarVector::new_random()?)?;
        }
        for _ in 0..10 {
            let probe_key = BipolarVector::new_random()?;
            let sim = s.probe(&probe_key)?;
            assert!(sim >= -1.0 - 1e-6 && sim <= 1.0 + 1e-6,
                "cosine probe out of [-1, 1]: {}", sim);
        }
        Ok(())
    }

    /// INVARIANT: new() starts with signal_count == 0.
    #[test]
    fn invariant_new_empty_signal_count() {
        let s = SuperpositionStorage::new();
        assert_eq!(s.signal_count, 0);
    }

    /// INVARIANT: commit_real grows signal_count monotonically.
    #[test]
    fn invariant_commit_real_monotone() -> Result<(), HdcError> {
        let mut s = SuperpositionStorage::new();
        let mut prev = s.signal_count;
        for _ in 0..10 {
            s.commit_real(&BipolarVector::new_random()?)?;
            let cur = s.signal_count;
            assert!(cur > prev, "signal_count should grow: {} -> {}", prev, cur);
            prev = cur;
        }
        Ok(())
    }

    /// INVARIANT: save/load roundtrip preserves signal_count.
    #[test]
    fn invariant_save_load_roundtrip() -> Result<(), HdcError> {
        let mut s = SuperpositionStorage::new();
        for _ in 0..5 {
            s.commit_real(&BipolarVector::new_random()?)?;
        }
        let path = "/tmp/lfi_superposition_invariant_test.json";
        s.save_to_disk(path)?;
        let loaded = SuperpositionStorage::load_from_disk(path)?;
        assert_eq!(s.signal_count, loaded.signal_count);
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    /// INVARIANT: load_from_disk returns Err for non-existent path.
    #[test]
    fn invariant_load_missing_path_errors() {
        let result = SuperpositionStorage::load_from_disk("/tmp/lfi_nonexistent_xxx.json");
        assert!(result.is_err(), "load of non-existent file should error");
    }
}
