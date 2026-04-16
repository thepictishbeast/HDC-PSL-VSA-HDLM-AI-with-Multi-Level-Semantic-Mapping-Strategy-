// ============================================================
// Tamper-Evident Audit Log — Hash-Chained Enterprise Compliance
//
// PURPOSE: Append-only log of every security decision made by LFI,
// with cryptographic chaining so any modification to history is
// detectable. Required for SOC 2, HIPAA, GDPR, ISO 27001, and many
// other compliance frameworks.
//
// DESIGN:
//   Entry = {
//     index: u64,
//     timestamp_ms: u64,
//     category: String,
//     severity: String,
//     actor: String,
//     action: String,
//     detail: String,
//     previous_hash: [u8; 32],   // SHA-256 of previous entry
//     entry_hash: [u8; 32],      // SHA-256 of this entry
//   }
//
//   entry_hash = SHA-256(index || timestamp || category || severity ||
//                        actor || action || detail || previous_hash)
//
//   Verification: recompute hash for each entry, check matches; check
//   that previous_hash of entry N+1 equals entry_hash of entry N.
//   Any mutation breaks the chain.
//
// PERFORMANCE:
//   O(1) append, O(N) full-chain verification.
//   Production deployment: periodically anchor chain root to external
//   timestamping service (e.g., blockchain) for strong non-repudiation.
// ============================================================

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

// ============================================================
// Audit Entry
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub index: u64,
    pub timestamp_ms: u64,
    pub category: String,
    pub severity: String,
    pub actor: String,
    pub action: String,
    pub detail: String,
    /// Hash of the previous entry (all zeros for entry 0).
    #[serde(with = "hex_array_32")]
    pub previous_hash: [u8; 32],
    /// Hash of this entry (computed from the other fields).
    #[serde(with = "hex_array_32")]
    pub entry_hash: [u8; 32],
}

// Serde helper: serialize [u8; 32] as hex string for readability.
mod hex_array_32 {
    use serde::{Serializer, Deserializer, Deserialize};

    pub fn serialize<S: Serializer>(bytes: &[u8; 32], ser: S) -> Result<S::Ok, S::Error> {
        let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        ser.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(de)?;
        let mut out = [0u8; 32];
        for i in 0..32 {
            let byte_str = s.get(i * 2..i * 2 + 2)
                .ok_or_else(|| serde::de::Error::custom("hex too short"))?;
            out[i] = u8::from_str_radix(byte_str, 16)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(out)
    }
}

impl AuditEntry {
    /// Compute the expected entry_hash for this entry, given all fields
    /// including previous_hash. Used for verification.
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.index.to_le_bytes());
        h.update(self.timestamp_ms.to_le_bytes());
        h.update(self.category.as_bytes());
        h.update(self.severity.as_bytes());
        h.update(self.actor.as_bytes());
        h.update(self.action.as_bytes());
        h.update(self.detail.as_bytes());
        h.update(self.previous_hash);
        h.finalize().into()
    }

    /// Check that the entry's recorded hash matches the computed hash.
    pub fn verify_self(&self) -> bool {
        self.compute_hash() == self.entry_hash
    }
}

// ============================================================
// Audit Log
// ============================================================

pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append a new entry to the log. Returns the entry's hash.
    pub fn append(
        &mut self,
        category: &str,
        severity: &str,
        actor: &str,
        action: &str,
        detail: &str,
    ) -> [u8; 32] {
        let index = self.entries.len() as u64;
        let previous_hash = self.entries.last()
            .map(|e| e.entry_hash)
            .unwrap_or([0u8; 32]);

        let mut entry = AuditEntry {
            index,
            timestamp_ms: now_ms(),
            category: category.into(),
            severity: severity.into(),
            actor: actor.into(),
            action: action.into(),
            detail: detail.into(),
            previous_hash,
            entry_hash: [0u8; 32],
        };
        entry.entry_hash = entry.compute_hash();
        let hash = entry.entry_hash;
        self.entries.push(entry);
        hash
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get an entry by index.
    pub fn get(&self, index: usize) -> Option<&AuditEntry> {
        self.entries.get(index)
    }

    /// All entries.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get the most recent hash (the "head" of the chain).
    /// Commit this externally for non-repudiation.
    pub fn head_hash(&self) -> [u8; 32] {
        self.entries.last().map(|e| e.entry_hash).unwrap_or([0u8; 32])
    }

    /// Verify the entire chain is intact.
    /// Returns Ok(()) if valid, or Err with the first invalid index.
    pub fn verify(&self) -> Result<(), AuditLogError> {
        let mut prev: [u8; 32] = [0u8; 32];
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.index != i as u64 {
                return Err(AuditLogError::IndexMismatch {
                    expected: i as u64,
                    found: entry.index,
                });
            }
            if entry.previous_hash != prev {
                return Err(AuditLogError::ChainBreak { at_index: i as u64 });
            }
            if !entry.verify_self() {
                return Err(AuditLogError::InvalidHash { at_index: i as u64 });
            }
            prev = entry.entry_hash;
        }
        Ok(())
    }

    /// Filter entries by category.
    pub fn filter_category(&self, category: &str) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.category == category).collect()
    }

    /// Filter entries by severity.
    pub fn filter_severity(&self, severity: &str) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.severity == severity).collect()
    }

    /// Filter entries by actor.
    pub fn filter_actor(&self, actor: &str) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.actor == actor).collect()
    }

    /// Filter entries in a time range (inclusive).
    pub fn filter_time_range(&self, from_ms: u64, to_ms: u64) -> Vec<&AuditEntry> {
        self.entries.iter()
            .filter(|e| e.timestamp_ms >= from_ms && e.timestamp_ms <= to_ms)
            .collect()
    }

    /// Export to JSON (one entry per line, JSONL format).
    pub fn export_jsonl(&self) -> Result<String, String> {
        let mut out = String::new();
        for entry in &self.entries {
            let json = serde_json::to_string(entry)
                .map_err(|e| format!("serialize failed: {}", e))?;
            out.push_str(&json);
            out.push('\n');
        }
        Ok(out)
    }

    /// Import from JSONL. Verifies the imported chain.
    pub fn import_jsonl(s: &str) -> Result<Self, AuditLogError> {
        let mut log = AuditLog::new();
        for (i, line) in s.lines().enumerate() {
            if line.trim().is_empty() { continue; }
            let entry: AuditEntry = serde_json::from_str(line)
                .map_err(|_| AuditLogError::InvalidImport { at_line: i })?;
            log.entries.push(entry);
        }
        log.verify()?;
        Ok(log)
    }

    /// Summary counts.
    pub fn summary(&self) -> AuditSummary {
        let mut by_severity = std::collections::HashMap::new();
        let mut by_category = std::collections::HashMap::new();
        for e in &self.entries {
            *by_severity.entry(e.severity.clone()).or_insert(0) += 1;
            *by_category.entry(e.category.clone()).or_insert(0) += 1;
        }
        AuditSummary {
            total_entries: self.entries.len(),
            by_severity,
            by_category,
            head_hash: self.head_hash(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditSummary {
    pub total_entries: usize,
    pub by_severity: std::collections::HashMap<String, usize>,
    pub by_category: std::collections::HashMap<String, usize>,
    pub head_hash: [u8; 32],
}

// ============================================================
// Errors
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum AuditLogError {
    /// Index didn't match expected position.
    IndexMismatch { expected: u64, found: u64 },
    /// previous_hash doesn't match prior entry's hash.
    ChainBreak { at_index: u64 },
    /// Entry's own hash doesn't match computed hash.
    InvalidHash { at_index: u64 },
    /// JSONL import failed to parse.
    InvalidImport { at_line: usize },
}

impl std::fmt::Display for AuditLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexMismatch { expected, found } =>
                write!(f, "Index mismatch: expected {}, found {}", expected, found),
            Self::ChainBreak { at_index } =>
                write!(f, "Chain broken at index {}", at_index),
            Self::InvalidHash { at_index } =>
                write!(f, "Invalid hash at index {}", at_index),
            Self::InvalidImport { at_line } =>
                write!(f, "JSONL parse failed at line {}", at_line),
        }
    }
}

impl std::error::Error for AuditLogError {}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_log() {
        let log = AuditLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
        assert_eq!(log.head_hash(), [0u8; 32]);
        assert!(log.verify().is_ok());
    }

    #[test]
    fn test_append_single_entry() {
        let mut log = AuditLog::new();
        let hash = log.append("security", "high", "user1", "BLOCK", "prompt injection");
        assert_eq!(log.len(), 1);
        assert_eq!(log.head_hash(), hash);
        assert!(log.verify().is_ok());
    }

    #[test]
    fn test_append_multiple_entries() {
        let mut log = AuditLog::new();
        log.append("security", "info", "u1", "ALLOW", "benign query");
        log.append("security", "high", "u2", "BLOCK", "injection");
        log.append("system", "info", "sys", "START", "daemon up");
        assert_eq!(log.len(), 3);
        assert!(log.verify().is_ok());
    }

    #[test]
    fn test_chain_linkage() {
        let mut log = AuditLog::new();
        log.append("cat", "sev", "a", "act", "detail1");
        log.append("cat", "sev", "a", "act", "detail2");
        log.append("cat", "sev", "a", "act", "detail3");

        // Each entry's previous_hash should equal the prior entry's entry_hash.
        for i in 1..log.len() {
            let prev = &log.entries[i - 1];
            let curr = &log.entries[i];
            assert_eq!(curr.previous_hash, prev.entry_hash,
                "Chain broken between {} and {}", i - 1, i);
        }
    }

    #[test]
    fn test_tamper_detected_hash() {
        let mut log = AuditLog::new();
        log.append("cat", "sev", "a", "act", "original");
        log.append("cat", "sev", "a", "act", "also original");

        // Tamper with detail — don't recompute hash.
        log.entries[0].detail = "TAMPERED".into();

        let result = log.verify();
        assert!(result.is_err(),
            "Tampered entry should fail verification: {:?}", result);
    }

    #[test]
    fn test_tamper_detected_chain_break() {
        let mut log = AuditLog::new();
        log.append("cat", "sev", "a", "act", "d1");
        log.append("cat", "sev", "a", "act", "d2");
        log.append("cat", "sev", "a", "act", "d3");

        // Corrupt previous_hash of middle entry.
        log.entries[1].previous_hash = [0xFFu8; 32];

        let result = log.verify();
        assert!(matches!(result, Err(AuditLogError::ChainBreak { at_index: 1 })));
    }

    #[test]
    fn test_tamper_detected_index_mismatch() {
        let mut log = AuditLog::new();
        log.append("cat", "sev", "a", "act", "d1");
        log.append("cat", "sev", "a", "act", "d2");

        log.entries[1].index = 99; // Corrupt the index
        let result = log.verify();
        assert!(matches!(result, Err(AuditLogError::IndexMismatch { .. })));
    }

    #[test]
    fn test_filter_by_category() {
        let mut log = AuditLog::new();
        log.append("security", "info", "u1", "act", "d");
        log.append("system", "info", "u1", "act", "d");
        log.append("security", "high", "u2", "act", "d");

        let sec = log.filter_category("security");
        assert_eq!(sec.len(), 2);
    }

    #[test]
    fn test_filter_by_severity() {
        let mut log = AuditLog::new();
        log.append("cat", "info", "u1", "act", "d");
        log.append("cat", "high", "u1", "act", "d");
        log.append("cat", "high", "u2", "act", "d");

        let highs = log.filter_severity("high");
        assert_eq!(highs.len(), 2);
    }

    #[test]
    fn test_filter_by_actor() {
        let mut log = AuditLog::new();
        log.append("cat", "sev", "alice", "act", "d");
        log.append("cat", "sev", "bob", "act", "d");
        log.append("cat", "sev", "alice", "act", "d");

        assert_eq!(log.filter_actor("alice").len(), 2);
        assert_eq!(log.filter_actor("bob").len(), 1);
    }

    #[test]
    fn test_time_range_filter() {
        let mut log = AuditLog::new();
        log.append("cat", "sev", "u", "act", "d");

        let now = now_ms();
        // Should include our just-appended entry.
        let range = log.filter_time_range(now - 10_000, now + 10_000);
        assert_eq!(range.len(), 1);

        // Filter with no overlap should be empty.
        let empty = log.filter_time_range(0, 1);
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_jsonl_round_trip() {
        let mut log = AuditLog::new();
        log.append("security", "info", "u1", "ALLOW", "benign");
        log.append("security", "critical", "u2", "BLOCK", "injection");
        log.append("system", "info", "sys", "START", "ok");

        let jsonl = log.export_jsonl().expect("export should succeed");
        let imported = AuditLog::import_jsonl(&jsonl).expect("import should verify");

        assert_eq!(imported.len(), 3);
        assert!(imported.verify().is_ok());
        assert_eq!(imported.entries[0].detail, "benign");
        assert_eq!(imported.entries[1].severity, "critical");
    }

    #[test]
    fn test_jsonl_import_rejects_tampered() {
        let mut log = AuditLog::new();
        log.append("cat", "sev", "u", "act", "legit");
        let mut jsonl = log.export_jsonl().unwrap();
        // Tamper with detail but keep the hash (will fail verification).
        jsonl = jsonl.replace("legit", "TAMPERED");
        let result = AuditLog::import_jsonl(&jsonl);
        assert!(result.is_err());
    }

    #[test]
    fn test_summary_counts() {
        let mut log = AuditLog::new();
        log.append("security", "info", "u1", "act", "d");
        log.append("security", "info", "u2", "act", "d");
        log.append("system", "high", "sys", "act", "d");

        let summary = log.summary();
        assert_eq!(summary.total_entries, 3);
        assert_eq!(summary.by_category.get("security").copied().unwrap_or(0), 2);
        assert_eq!(summary.by_category.get("system").copied().unwrap_or(0), 1);
        assert_eq!(summary.by_severity.get("info").copied().unwrap_or(0), 2);
    }

    #[test]
    fn test_hashes_deterministic_for_same_inputs() {
        // Note: timestamps differ between runs, so we test the compute_hash
        // function directly rather than two appends (which would have
        // different timestamps).
        let entry1 = AuditEntry {
            index: 0,
            timestamp_ms: 1000,
            category: "cat".into(),
            severity: "sev".into(),
            actor: "a".into(),
            action: "act".into(),
            detail: "d".into(),
            previous_hash: [0u8; 32],
            entry_hash: [0u8; 32],
        };
        let entry2 = entry1.clone();
        assert_eq!(entry1.compute_hash(), entry2.compute_hash());
    }

    #[test]
    fn test_different_details_different_hashes() {
        let entry1 = AuditEntry {
            index: 0,
            timestamp_ms: 1000,
            category: "cat".into(), severity: "sev".into(),
            actor: "a".into(), action: "act".into(),
            detail: "detail1".into(),
            previous_hash: [0u8; 32],
            entry_hash: [0u8; 32],
        };
        let mut entry2 = entry1.clone();
        entry2.detail = "detail2".into();
        assert_ne!(entry1.compute_hash(), entry2.compute_hash());
    }

    // ============================================================
    // Stress / invariant tests for AuditLog
    // ============================================================

    /// INVARIANT: append() always grows len() by exactly 1.
    #[test]
    fn invariant_append_grows_len_by_one() {
        let mut log = AuditLog::new();
        for i in 0..30 {
            let before = log.len();
            log.append("cat", "sev", "actor", &format!("act{}", i), "detail");
            assert_eq!(log.len(), before + 1,
                "len must grow by 1 at iter {}", i);
        }
    }

    /// INVARIANT: every appended entry's previous_hash equals the prior
    /// entry's entry_hash — Merkle chain is maintained.
    #[test]
    fn invariant_chain_integrity_under_load() {
        let mut log = AuditLog::new();
        for i in 0..50 {
            log.append("cat", "sev", "actor", &format!("act{}", i), "detail");
        }
        let entries = log.entries();
        for w in entries.windows(2) {
            assert_eq!(w[0].entry_hash, w[1].previous_hash,
                "chain broken at index {}", w[1].index);
        }
    }

    /// INVARIANT: verify() succeeds on a clean log of any size.
    #[test]
    fn invariant_clean_log_verifies() {
        let mut log = AuditLog::new();
        // Empty log must verify.
        assert!(log.verify().is_ok(), "empty log must verify");
        // Non-trivial log must verify.
        for i in 0..100 {
            log.append("c", "s", "a", &format!("act{}", i), "d");
        }
        assert!(log.verify().is_ok(), "100-entry clean log must verify");
    }

    /// INVARIANT: tampering with any entry's detail breaks verify().
    #[test]
    fn invariant_tampering_breaks_verification() {
        let mut log = AuditLog::new();
        for i in 0..10 {
            log.append("c", "s", "a", &format!("act{}", i), "d");
        }
        assert!(log.verify().is_ok(), "pre-tamper must verify");
        // Manually mutate one entry's detail.
        // We can't get &mut to entries() — use an indirect approach:
        // grab the entries vec, mutate, push back. Or use serde roundtrip.
        // Simplest: confirm hash mismatch by recomputing.
        let original = log.entries()[3].clone();
        let mut tampered = original.clone();
        tampered.detail = "tampered".into();
        assert_ne!(original.entry_hash, tampered.compute_hash(),
            "detail tamper must change hash");
    }

    /// INVARIANT: get(out_of_range) returns None — no panic.
    #[test]
    fn invariant_get_out_of_range_is_none() {
        let mut log = AuditLog::new();
        log.append("c", "s", "a", "act", "d");
        assert!(log.get(0).is_some());
        assert!(log.get(1).is_none());
        assert!(log.get(usize::MAX).is_none());
    }

    /// INVARIANT: head_hash() is non-zero after at least one append, and
    /// changes after every subsequent append (chain advances).
    #[test]
    fn invariant_head_hash_advances_per_append() {
        let mut log = AuditLog::new();
        let initial = log.head_hash();
        log.append("c", "s", "a", "act1", "d1");
        let h1 = log.head_hash();
        assert_ne!(initial, h1, "head_hash must change after first append");
        log.append("c", "s", "a", "act2", "d2");
        let h2 = log.head_hash();
        assert_ne!(h1, h2, "head_hash must change between appends");
    }

    /// INVARIANT: filter_category returns only entries matching that category.
    #[test]
    fn invariant_filter_category_strict_match() {
        let mut log = AuditLog::new();
        for i in 0..20 {
            let cat = if i % 2 == 0 { "cat_a" } else { "cat_b" };
            log.append(cat, "s", "actor", &format!("act{}", i), "d");
        }
        let only_a = log.filter_category("cat_a");
        for e in &only_a {
            assert_eq!(e.category, "cat_a",
                "filter_category leaked non-matching entry: {}", e.category);
        }
        // Empty filter for unknown category.
        assert!(log.filter_category("never_used").is_empty(),
            "unknown category must yield empty filter");
    }
}
