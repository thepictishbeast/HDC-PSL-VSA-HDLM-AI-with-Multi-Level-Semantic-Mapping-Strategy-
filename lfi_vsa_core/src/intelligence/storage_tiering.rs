//! # Purpose
//! Hot/warm/cold storage tiering for 56M+ facts.
//! Prevents unbounded growth by promoting frequently-accessed facts
//! and demoting rarely-used ones.
//!
//! # Tiers
//! - Hot: vector-indexed, kept in FTS5, fast retrieval (<10ms)
//! - Warm: in SQLite, FTS5 indexed, normal retrieval
//! - Cold: compressed archive, slow retrieval, re-derivable from theories

/// Storage tier for a fact.
#[derive(Debug, Clone, PartialEq)]
pub enum StorageTier {
    Hot,   // Frequently accessed, priority in RAG
    Warm,  // Normal, in SQLite + FTS5
    Cold,  // Rarely accessed, candidate for archival
}

/// Promote/demote facts based on access patterns.
pub struct TieringEngine {
    /// Access count threshold for hot promotion.
    pub hot_threshold: u32,
    /// Days without access before cold demotion.
    pub cold_after_days: u32,
}

impl Default for TieringEngine {
    fn default() -> Self {
        Self {
            hot_threshold: 5,
            cold_after_days: 90,
        }
    }
}

impl TieringEngine {
    /// Classify a fact's tier based on access patterns.
    pub fn classify(&self, access_count: u32, days_since_access: u32) -> StorageTier {
        if access_count >= self.hot_threshold {
            StorageTier::Hot
        } else if days_since_access > self.cold_after_days {
            StorageTier::Cold
        } else {
            StorageTier::Warm
        }
    }

    /// Record an access to a fact (for the API layer to call).
    pub fn record_access(key: &str, db: &crate::persistence::BrainDb) {
        let conn = db.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE facts SET access_count = COALESCE(access_count, 0) + 1, last_accessed = datetime('now') WHERE key = ?1",
            rusqlite::params![key],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_classification() {
        let engine = TieringEngine::default();
        assert_eq!(engine.classify(10, 1), StorageTier::Hot);
    }

    #[test]
    fn test_cold_classification() {
        let engine = TieringEngine::default();
        assert_eq!(engine.classify(0, 100), StorageTier::Cold);
    }

    #[test]
    fn test_warm_classification() {
        let engine = TieringEngine::default();
        assert_eq!(engine.classify(2, 30), StorageTier::Warm);
    }
}
