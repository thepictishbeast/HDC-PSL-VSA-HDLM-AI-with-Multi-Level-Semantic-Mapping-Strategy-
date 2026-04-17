// ============================================================
// LFI Persistent Knowledge Store — Cross-Session Memory
//
// Saves and loads learned concepts to/from JSON on disk so that
// knowledge survives across restarts. The AI remembers what it
// learned in previous sessions.
//
// Storage format: JSON file at a configurable path.
// Default: ~/.lfi/knowledge.json
//
// Includes metadata: last_saved timestamp, total concepts,
// session count, learning history.
// ============================================================

use crate::hdc::error::HdcError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A serializable learned concept (no BipolarVector — reconstructed on load).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredConcept {
    /// Name of the concept.
    pub name: String,
    /// How well the AI understands this (0.0 = barely, 1.0 = mastery).
    pub mastery: f64,
    /// How many times this concept has been encountered.
    pub encounter_count: usize,
    /// Trust score of the source (1.0 = Sovereign, 0.0 = Untrusted).
    pub trust_score: f64,
    /// Related concept names.
    pub related_concepts: Vec<String>,
    /// Human-readable definition if one was taught.
    pub definition: Option<String>,
    /// When this concept was first learned.
    pub first_learned: String,
    /// When this concept was last reinforced.
    pub last_reinforced: String,
}

/// A serializable conversation fact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFact {
    /// Key (e.g., "sovereign_name", "gravity").
    pub key: String,
    /// Value (e.g., "Paul", "force that attracts objects").
    pub value: String,
    /// Global session ID where this was first learned.
    pub session_id: String,
}

/// The full persistent knowledge store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeStore {
    /// Version of the store format.
    pub version: u32,
    /// When the store was last saved.
    pub last_saved: String,
    /// Current session ID.
    pub current_session_id: String,
    /// How many sessions have used this store.
    pub session_count: u64,
    /// All learned concepts.
    pub concepts: Vec<StoredConcept>,
    /// Persistent conversation facts (survive across sessions).
    pub facts: Vec<StoredFact>,
    /// Topics the AI has searched for (to avoid re-searching).
    pub searched_topics: Vec<String>,
    /// Background learning log entries.
    pub learning_log: Vec<String>,
}

impl KnowledgeStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        debuglog!("KnowledgeStore::new: creating empty persistent store");
        let session_id = format!("SESSION_{}", chrono::Utc::now().timestamp());
        Self {
            version: 1,
            last_saved: Self::now_timestamp(),
            current_session_id: session_id,
            session_count: 0,
            concepts: Vec::new(),
            facts: Vec::new(),
            searched_topics: Vec::new(),
            learning_log: Vec::new(),
        }
    }

    /// Get the default storage path.
    pub fn default_path() -> PathBuf {
        debuglog!("KnowledgeStore::default_path: resolving storage location");
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let dir = PathBuf::from(&home).join(".lfi");
        dir.join("knowledge.json")
    }

    /// Load from disk. Returns a new empty store if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, HdcError> {
        debuglog!("KnowledgeStore::load: from {:?}", path);

        if !path.exists() {
            debuglog!("KnowledgeStore::load: file not found, returning empty store");
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Failed to read knowledge store: {}", e),
        })?;

        let mut store: Self = serde_json::from_str(&content).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Failed to parse knowledge store: {}", e),
        })?;

        store.session_count += 1;
        store.current_session_id = format!("SESSION_{}", chrono::Utc::now().timestamp());
        debuglog!(
            "KnowledgeStore::load: loaded {} concepts, {} facts, session #{}",
            store.concepts.len(), store.facts.len(), store.session_count
        );

        Ok(store)
    }

    /// Save to disk.
    pub fn save(&mut self, path: &Path) -> Result<(), HdcError> {
        debuglog!("KnowledgeStore::save: to {:?}", path);

        self.last_saved = Self::now_timestamp();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| HdcError::InitializationFailed {
                reason: format!("Failed to create knowledge directory: {}", e),
            })?;
        }

        let json = serde_json::to_string_pretty(&self).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Failed to serialize knowledge store: {}", e),
        })?;

        std::fs::write(path, json).map_err(|e| HdcError::InitializationFailed {
            reason: format!("Failed to write knowledge store: {}", e),
        })?;

        debuglog!(
            "KnowledgeStore::save: saved {} concepts, {} facts",
            self.concepts.len(), self.facts.len()
        );

        Ok(())
    }

    /// Add or update a concept.
    pub fn upsert_concept(&mut self, concept: StoredConcept) {
        debuglog!("KnowledgeStore::upsert_concept: '{}'", concept.name);

        for existing in &mut self.concepts {
            if existing.name == concept.name {
                existing.mastery = concept.mastery;
                existing.encounter_count = concept.encounter_count;
                existing.trust_score = concept.trust_score;
                existing.related_concepts = concept.related_concepts;
                existing.definition = concept.definition;
                existing.last_reinforced = Self::now_timestamp();
                return;
            }
        }
        self.concepts.push(concept);
    }

    /// Add or update a fact.
    pub fn upsert_fact(&mut self, key: &str, value: &str) {
        debuglog!("KnowledgeStore::upsert_fact: '{}' = '{}'", key, value);
        for existing in &mut self.facts {
            if existing.key == key {
                existing.value = value.to_string();
                return;
            }
        }
        self.facts.push(StoredFact {
            key: key.to_string(),
            value: value.to_string(),
            session_id: self.current_session_id.clone(),
        });
    }

    /// Get a fact by key.
    pub fn get_fact(&self, key: &str) -> Option<&str> {
        self.facts.iter()
            .find(|f| f.key == key)
            .map(|f| f.value.as_str())
    }

    /// Check if a topic has already been searched.
    pub fn has_searched(&self, topic: &str) -> bool {
        self.searched_topics.iter().any(|t| t == topic)
    }

    /// Mark a topic as searched.
    pub fn mark_searched(&mut self, topic: &str) {
        if !self.has_searched(topic) {
            self.searched_topics.push(topic.to_string());
            // Keep a reasonable limit
            if self.searched_topics.len() > 1000 {
                self.searched_topics.remove(0);
            }
        }
    }

    /// Add a learning log entry.
    pub fn log_learning(&mut self, entry: &str) {
        let stamped = format!("[{}] {}", Self::now_timestamp(), entry);
        self.learning_log.push(stamped);
        // Keep last 500 entries
        if self.learning_log.len() > 500 {
            self.learning_log.remove(0);
        }
    }

    /// Get current timestamp as ISO string.
    fn now_timestamp() -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_store_new() {
        let store = KnowledgeStore::new();
        assert_eq!(store.version, 1);
        assert_eq!(store.session_count, 0);
        assert!(store.concepts.is_empty());
        assert!(store.facts.is_empty());
    }

    #[test]
    fn test_knowledge_store_upsert_concept() {
        let mut store = KnowledgeStore::new();
        store.upsert_concept(StoredConcept {
            name: "rust".to_string(),
            mastery: 0.9,
            encounter_count: 5,
            trust_score: 1.0,
            related_concepts: vec!["programming".to_string()],
            definition: Some("A systems programming language".to_string()),
            first_learned: "2026-03-28".to_string(),
            last_reinforced: "2026-03-28".to_string(),
        });
        assert_eq!(store.concepts.len(), 1);

        // Update existing
        store.upsert_concept(StoredConcept {
            name: "rust".to_string(),
            mastery: 0.95,
            encounter_count: 10,
            trust_score: 1.0,
            related_concepts: vec!["programming".to_string(), "safety".to_string()],
            definition: Some("A systems programming language focused on safety".to_string()),
            first_learned: "2026-03-28".to_string(),
            last_reinforced: "2026-03-28".to_string(),
        });
        assert_eq!(store.concepts.len(), 1);
        assert_eq!(store.concepts[0].mastery, 0.95);
    }

    #[test]
    fn test_knowledge_store_facts() {
        let mut store = KnowledgeStore::new();
        store.upsert_fact("name", "Paul");
        assert_eq!(store.get_fact("name"), Some("Paul"));
        assert_eq!(store.get_fact("unknown"), None);

        // Update
        store.upsert_fact("name", "William");
        assert_eq!(store.get_fact("name"), Some("William"));
        assert_eq!(store.facts.len(), 1);
    }

    #[test]
    fn test_knowledge_store_save_load() -> Result<(), HdcError> {
        let dir = std::env::temp_dir().join("lfi_test_knowledge");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_knowledge.json");

        let mut store = KnowledgeStore::new();
        store.facts.push(StoredFact {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
            session_id: "test_session".to_string(),
        });
        store.upsert_concept(StoredConcept {
            name: "test_concept".to_string(),
            mastery: 0.5,
            encounter_count: 1,
            trust_score: 1.0,
            related_concepts: Vec::new(),
            definition: Some("A test".to_string()),
            first_learned: "2026-03-28".to_string(),
            last_reinforced: "2026-03-28".to_string(),
        });
        store.save(&path)?;

        let loaded = KnowledgeStore::load(&path)?;
        assert_eq!(loaded.get_fact("test_key"), Some("test_value"));
        assert_eq!(loaded.concepts.len(), 1);
        assert_eq!(loaded.session_count, 1); // Incremented on load

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);

        Ok(())
    }

    #[test]
    fn test_searched_topics() {
        let mut store = KnowledgeStore::new();
        assert!(!store.has_searched("rust"));
        store.mark_searched("rust");
        assert!(store.has_searched("rust"));
    }

    #[test]
    fn test_log_learning() {
        let mut store = KnowledgeStore::new();
        store.log_learning("Learned about HDC vectors");
        store.log_learning("Learned about PSL axioms");
        assert_eq!(store.learning_log.len(), 2);
        assert!(store.learning_log[0].contains("HDC"));
    }

    #[test]
    fn test_fact_update() {
        let mut store = KnowledgeStore::new();
        store.upsert_fact("name", "Alice");
        assert_eq!(store.get_fact("name"), Some("Alice"));
        store.upsert_fact("name", "Bob"); // Update
        assert_eq!(store.get_fact("name"), Some("Bob"));
        assert_eq!(store.facts.len(), 1, "Should update, not duplicate");
    }

    #[test]
    fn test_default_path() {
        let path = KnowledgeStore::default_path();
        assert!(path.to_str().unwrap().contains("lfi"));
    }

    #[test]
    fn test_store_serialization() {
        let store = KnowledgeStore::new();
        let json = serde_json::to_string(&store).expect("serialize");
        let recovered: KnowledgeStore = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(recovered.version, store.version);
    }

    // ============================================================
    // Stress / invariant tests for KnowledgeStore
    // ============================================================

    /// INVARIANT: get_fact returns None for keys never inserted, Some for
    /// inserted keys.
    #[test]
    fn invariant_get_fact_matches_upsert() {
        let mut store = KnowledgeStore::new();
        assert!(store.get_fact("nonexistent").is_none());
        store.upsert_fact("k1", "v1");
        assert_eq!(store.get_fact("k1"), Some("v1"));
        assert!(store.get_fact("k2").is_none());
    }

    /// INVARIANT: upsert_fact overwrites when the key already exists —
    /// no silent duplicates, no rejection.
    #[test]
    fn invariant_upsert_fact_overwrites() {
        let mut store = KnowledgeStore::new();
        store.upsert_fact("color", "red");
        assert_eq!(store.get_fact("color"), Some("red"));
        store.upsert_fact("color", "blue");
        assert_eq!(store.get_fact("color"), Some("blue"));
        store.upsert_fact("color", "");
        assert_eq!(store.get_fact("color"), Some(""));
    }

    /// INVARIANT: mark_searched followed by has_searched returns true for
    /// the same topic, false for unmarked topics.
    #[test]
    fn invariant_mark_searched_consistent_with_has_searched() {
        let mut store = KnowledgeStore::new();
        assert!(!store.has_searched("topic_a"));
        store.mark_searched("topic_a");
        assert!(store.has_searched("topic_a"));
        assert!(!store.has_searched("topic_b"),
            "marking 'topic_a' must not flag 'topic_b'");
        // Idempotent: marking twice doesn't change observable state.
        store.mark_searched("topic_a");
        assert!(store.has_searched("topic_a"));
    }

    /// INVARIANT: KnowledgeStore round-trips through JSON serialization
    /// preserving facts, searched topics, and learning log.
    #[test]
    fn invariant_serialize_roundtrip_preserves_state() {
        let mut store = KnowledgeStore::new();
        store.upsert_fact("k1", "v1");
        store.upsert_fact("k2", "v2");
        store.mark_searched("topic_x");
        store.log_learning("learned X");
        let json = serde_json::to_string(&store).expect("serialize");
        let recovered: KnowledgeStore = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(recovered.get_fact("k1"), Some("v1"));
        assert_eq!(recovered.get_fact("k2"), Some("v2"));
        assert!(recovered.has_searched("topic_x"));
    }

    /// INVARIANT: loading a corrupt JSON file returns Err — caller can
    /// distinguish corruption from an absent (first-run) file. Missing
    /// files are intentionally treated as empty-store (first-run UX).
    #[test]
    fn invariant_load_corrupt_file_is_err() {
        let path = std::env::temp_dir().join("lfi_persistence_corrupt.json");
        std::fs::write(&path, b"not valid json {{{}}}}").expect("write");
        let result = KnowledgeStore::load(&path);
        assert!(result.is_err(),
            "loading corrupt JSON must return Err");
        let _ = std::fs::remove_file(&path);
    }

    /// INVARIANT: loading a missing path returns Ok(empty) — first-run UX
    /// is to treat absent state as fresh start, not as a fatal error.
    #[test]
    fn invariant_load_missing_path_returns_empty_ok() {
        let bad_path = PathBuf::from("/nonexistent/lfi_test_path_xyz_unique.json");
        let result = KnowledgeStore::load(&bad_path);
        assert!(result.is_ok(), "missing file must return Ok(empty), not Err");
        let store = result.unwrap();
        assert!(store.get_fact("anything").is_none());
    }

    /// INVARIANT: save() then load() returns a store equal to the original
    /// for arbitrary fact/topic content (round-trip via disk).
    #[test]
    fn invariant_save_load_disk_roundtrip() -> Result<(), HdcError> {
        let mut store = KnowledgeStore::new();
        for i in 0..20 {
            store.upsert_fact(&format!("k_{}", i), &format!("v_{}", i));
        }
        for i in 0..10 {
            store.mark_searched(&format!("topic_{}", i));
        }
        let path = std::env::temp_dir().join("lfi_persistence_invariant_test.json");
        store.save(&path)?;
        let loaded = KnowledgeStore::load(&path)?;
        for i in 0..20 {
            assert_eq!(loaded.get_fact(&format!("k_{}", i)),
                Some(format!("v_{}", i).as_str()),
                "fact k_{} not preserved through disk roundtrip", i);
        }
        for i in 0..10 {
            assert!(loaded.has_searched(&format!("topic_{}", i)),
                "topic_{} not preserved through disk roundtrip", i);
        }
        let _ = std::fs::remove_file(&path);
        Ok(())
    }

    /// INVARIANT: get_fact returns None for missing key.
    #[test]
    fn invariant_get_fact_missing_none() {
        let store = KnowledgeStore::new();
        assert!(store.get_fact("nonexistent_key_xxx").is_none());
    }

    /// INVARIANT: default_path is absolute and stable.
    #[test]
    fn invariant_default_path_absolute_and_stable() {
        let a = KnowledgeStore::default_path();
        let b = KnowledgeStore::default_path();
        assert_eq!(a, b, "default_path should be deterministic");
        assert!(a.is_absolute(), "default_path should be absolute: {:?}", a);
    }

    /// INVARIANT: has_searched returns false for unseen topics.
    #[test]
    fn invariant_has_searched_unseen_false() {
        let store = KnowledgeStore::new();
        assert!(!store.has_searched("UnseenTopic12345"));
    }

    /// INVARIANT: mark_searched is idempotent.
    #[test]
    fn invariant_mark_searched_idempotent() {
        let mut store = KnowledgeStore::new();
        store.mark_searched("topic");
        store.mark_searched("topic");
        store.mark_searched("topic");
        assert!(store.has_searched("topic"));
    }
}
