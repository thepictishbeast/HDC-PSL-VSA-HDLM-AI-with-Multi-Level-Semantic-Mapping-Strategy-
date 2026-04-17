// ============================================================
// Weight Manager — Persistent Intelligence Backup & Restore
//
// LFI's "intelligence" = VSA vectors, not neural network weights.
// This module saves and restores the complete learned state:
//   - Knowledge store (concepts, facts, mastery levels)
//   - PSL feedback rejections (avoidance patterns)
//   - Planner solution patterns
//   - Analogy library
// ============================================================

use crate::hdc::error::HdcError;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};

/// A complete intelligence checkpoint.
#[derive(Serialize, Deserialize)]
pub struct IntelligenceCheckpoint {
    pub version: u32,
    pub timestamp: String,
    pub description: String,
    pub episodes_completed: u64,
    pub concepts_count: usize,
    pub rejection_patterns: usize,
    pub solution_patterns: usize,
    pub knowledge_store_json: String,
    pub integrity_hash: String,
}

impl IntelligenceCheckpoint {
    pub fn capture(
        knowledge_json: &str, episodes: u64, concepts: usize,
        rejections: usize, solutions: usize, description: &str,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let integrity_hash = Self::compute_hash(knowledge_json, &timestamp);
        Self {
            version: 1, timestamp, description: description.into(),
            episodes_completed: episodes, concepts_count: concepts,
            rejection_patterns: rejections, solution_patterns: solutions,
            knowledge_store_json: knowledge_json.into(), integrity_hash,
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), HdcError> {
        debuglog!("IntelligenceCheckpoint::save: {}", path.display());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| HdcError::PersistenceFailure {
                detail: format!("mkdir failed: {}", e),
            })?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| HdcError::PersistenceFailure {
            detail: format!("serialize failed: {}", e),
        })?;
        std::fs::write(path, json).map_err(|e| HdcError::PersistenceFailure {
            detail: format!("write failed: {}", e),
        })
    }

    pub fn load(path: &Path) -> Result<Self, HdcError> {
        debuglog!("IntelligenceCheckpoint::load: {}", path.display());
        let json = std::fs::read_to_string(path).map_err(|e| HdcError::PersistenceFailure {
            detail: format!("read failed: {}", e),
        })?;
        let cp: Self = serde_json::from_str(&json).map_err(|e| HdcError::PersistenceFailure {
            detail: format!("deserialize failed: {}", e),
        })?;
        let expected = Self::compute_hash(&cp.knowledge_store_json, &cp.timestamp);
        if expected != cp.integrity_hash {
            return Err(HdcError::PersistenceFailure {
                detail: "Integrity check FAILED — data corrupted".into(),
            });
        }
        Ok(cp)
    }

    pub fn default_dir() -> PathBuf { PathBuf::from("/root/.lfi/checkpoints") }

    pub fn generate_filename() -> String {
        format!("checkpoint_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
    }

    fn compute_hash(data: &str, timestamp: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        data.hash(&mut h);
        timestamp.hash(&mut h);
        format!("{:016x}", h.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_and_save_load() -> Result<(), HdcError> {
        let cp = IntelligenceCheckpoint::capture(
            r#"{"concepts":["rust"]}"#, 100, 50, 10, 6, "Test",
        );
        let path = PathBuf::from("/tmp/lfi_test_cp.json");
        cp.save(&path)?;
        let loaded = IntelligenceCheckpoint::load(&path)?;
        assert_eq!(loaded.episodes_completed, 100);
        assert_eq!(loaded.concepts_count, 50);
        let _ = std::fs::remove_file(&path);
        Ok(())
    }

    #[test]
    fn test_integrity_catches_corruption() -> Result<(), HdcError> {
        let cp = IntelligenceCheckpoint::capture(r#"{"clean":true}"#, 10, 5, 1, 1, "Integrity");
        let path = PathBuf::from("/tmp/lfi_test_corrupt.json");
        cp.save(&path)?;
        let mut json = std::fs::read_to_string(&path).unwrap();
        json = json.replace("clean", "CORRUPT");
        std::fs::write(&path, json).unwrap();
        assert!(IntelligenceCheckpoint::load(&path).is_err());
        let _ = std::fs::remove_file(&path);
        Ok(())
    }

    #[test]
    fn test_generate_filename() {
        let name = IntelligenceCheckpoint::generate_filename();
        assert!(name.starts_with("checkpoint_") && name.ends_with(".json"));
    }

    // ============================================================
    // Stress / invariant tests for IntelligenceCheckpoint
    // ============================================================

    /// INVARIANT: capture produces a checkpoint where version >= 1 and
    /// integrity hash is non-empty.
    #[test]
    fn invariant_capture_has_version_and_hash() {
        let cp = IntelligenceCheckpoint::capture(
            r#"{"k":"v"}"#, 0, 0, 0, 0, "",
        );
        assert!(cp.version >= 1, "version must be >= 1");
        assert!(!cp.integrity_hash.is_empty(), "integrity hash must be non-empty");
        assert!(!cp.timestamp.is_empty(), "timestamp must be non-empty");
    }

    /// INVARIANT: save then load yields an equal checkpoint on every field
    /// that influences the hash.
    #[test]
    fn invariant_save_load_roundtrip() -> Result<(), HdcError> {
        let cp = IntelligenceCheckpoint::capture(
            r#"{"concepts":["x","y","z"]}"#, 42, 3, 1, 0, "round-trip",
        );
        let path = PathBuf::from("/tmp/lfi_invariant_roundtrip.json");
        cp.save(&path)?;
        let loaded = IntelligenceCheckpoint::load(&path)?;
        assert_eq!(loaded.episodes_completed, cp.episodes_completed);
        assert_eq!(loaded.concepts_count, cp.concepts_count);
        assert_eq!(loaded.knowledge_store_json, cp.knowledge_store_json);
        assert_eq!(loaded.integrity_hash, cp.integrity_hash);
        assert_eq!(loaded.description, cp.description);
        let _ = std::fs::remove_file(&path);
        Ok(())
    }

    /// INVARIANT: load fails on non-existent or garbage files.
    #[test]
    fn invariant_load_rejects_missing_and_garbage() {
        let missing = PathBuf::from("/tmp/lfi_missing_checkpoint_xxxxxx.json");
        assert!(IntelligenceCheckpoint::load(&missing).is_err(),
            "load of missing file should fail");

        let garbage_path = PathBuf::from("/tmp/lfi_invariant_garbage.json");
        std::fs::write(&garbage_path, b"NOT VALID JSON AT ALL").unwrap();
        assert!(IntelligenceCheckpoint::load(&garbage_path).is_err(),
            "load of non-JSON should fail");
        let _ = std::fs::remove_file(&garbage_path);
    }

    /// INVARIANT: default_dir is absolute and stable across calls.
    #[test]
    fn invariant_default_dir_stable() {
        let a = IntelligenceCheckpoint::default_dir();
        let b = IntelligenceCheckpoint::default_dir();
        assert_eq!(a, b, "default_dir should be deterministic");
        assert!(a.is_absolute(), "default_dir should be absolute: {:?}", a);
    }

    /// INVARIANT: generate_filename is unique across back-to-back calls
    /// when separated by at least one second (due to timestamp format).
    /// We test format instead.
    #[test]
    fn invariant_generate_filename_format() {
        for _ in 0..5 {
            let name = IntelligenceCheckpoint::generate_filename();
            assert!(name.starts_with("checkpoint_"));
            assert!(name.ends_with(".json"));
            let stem_len = name.len() - "checkpoint_".len() - ".json".len();
            assert!(stem_len > 0, "stem should have content");
        }
    }
}
