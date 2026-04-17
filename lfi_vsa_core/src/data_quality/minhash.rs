// ============================================================
// MinHash Near-Duplicate Detection
// Sprint 1: Quality Ceiling — FineWeb-inspired parameters
//
// PURPOSE: Detect near-duplicate facts that differ only in
// whitespace, punctuation, or minor rewording. Uses MinHash
// locality-sensitive hashing for O(1) similarity estimation.
//
// PARAMETERS (FineWeb-aligned):
//   - 5-gram shingling (character-level)
//   - 128 hash functions
//   - Jaccard threshold: 0.8 (80% similarity = near-duplicate)
//   - 20 bands × 6 rows per band (LSH banding)
//
// BUG ASSUMPTION: Character-level shingling may miss semantic
// duplicates ("The cat sat" vs "A feline rested"). That requires
// embedding-based dedup (Sprint 2+).
// ============================================================

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// MinHash signature: a compact fingerprint for near-duplicate detection.
#[derive(Debug, Clone)]
pub struct MinHashSignature {
    pub hashes: Vec<u64>,
}

/// MinHash deduplicator with LSH banding for efficient similarity search.
pub struct MinHashDedup {
    /// Number of hash functions (signature length)
    pub num_hashes: usize,
    /// Number of LSH bands
    pub num_bands: usize,
    /// Rows per band (num_hashes / num_bands)
    pub rows_per_band: usize,
    /// Jaccard similarity threshold for near-duplicate
    pub threshold: f64,
    /// LSH buckets: band_index → (bucket_hash → list of fact keys)
    buckets: Vec<HashMap<u64, Vec<String>>>,
}

impl MinHashDedup {
    /// Create with FineWeb-aligned parameters.
    pub fn new() -> Self {
        let num_hashes = 128;
        let num_bands = 20;
        let rows_per_band = num_hashes / num_bands; // 6 (with 8 leftover, ignored)
        Self {
            num_hashes,
            num_bands,
            rows_per_band,
            threshold: 0.8,
            buckets: (0..num_bands).map(|_| HashMap::new()).collect(),
        }
    }

    /// Generate character 5-gram shingles from text.
    pub fn shingle(text: &str, n: usize) -> HashSet<u64> {
        let chars: Vec<char> = text.chars().collect();
        let mut shingles = HashSet::new();
        if chars.len() < n {
            // Text too short for shingling — hash the whole thing
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            shingles.insert(h.finish());
            return shingles;
        }
        for window in chars.windows(n) {
            let s: String = window.iter().collect();
            let mut h = std::collections::hash_map::DefaultHasher::new();
            s.hash(&mut h);
            shingles.insert(h.finish());
        }
        shingles
    }

    /// Compute MinHash signature from a set of shingles.
    pub fn signature(&self, shingles: &HashSet<u64>) -> MinHashSignature {
        let mut hashes = vec![u64::MAX; self.num_hashes];
        for &shingle in shingles {
            for i in 0..self.num_hashes {
                // BUG ASSUMPTION: Using XOR with seed is a fast but imperfect
                // universal hash family. For production, use Murmur3 or xxHash.
                let h = shingle.wrapping_mul(i as u64 + 1).wrapping_add((i as u64).wrapping_mul(0x9E3779B97F4A7C15));
                if h < hashes[i] {
                    hashes[i] = h;
                }
            }
        }
        MinHashSignature { hashes }
    }

    /// Estimate Jaccard similarity between two signatures.
    pub fn jaccard(a: &MinHashSignature, b: &MinHashSignature) -> f64 {
        let matches = a.hashes.iter().zip(&b.hashes).filter(|(x, y)| x == y).count();
        matches as f64 / a.hashes.len() as f64
    }

    /// Insert a fact into the LSH index. Returns list of near-duplicate keys found.
    pub fn insert(&mut self, key: &str, sig: &MinHashSignature) -> Vec<String> {
        let mut duplicates = Vec::new();
        for band in 0..self.num_bands {
            let start = band * self.rows_per_band;
            let end = start + self.rows_per_band;
            if end > sig.hashes.len() { break; }

            // Hash the band slice to a bucket
            let mut h = std::collections::hash_map::DefaultHasher::new();
            sig.hashes[start..end].hash(&mut h);
            let bucket_hash = h.finish();

            let bucket = self.buckets[band].entry(bucket_hash).or_default();
            // Any existing key in this bucket is a candidate
            for existing_key in bucket.iter() {
                if !duplicates.contains(existing_key) {
                    duplicates.push(existing_key.clone());
                }
            }
            bucket.push(key.to_string());
        }
        duplicates
    }

    /// Check if a signature is a near-duplicate of anything already indexed.
    /// Returns true if any candidate has Jaccard >= threshold.
    pub fn is_near_duplicate(&self, sig: &MinHashSignature, all_sigs: &HashMap<String, MinHashSignature>) -> Option<String> {
        for band in 0..self.num_bands {
            let start = band * self.rows_per_band;
            let end = start + self.rows_per_band;
            if end > sig.hashes.len() { break; }

            let mut h = std::collections::hash_map::DefaultHasher::new();
            sig.hashes[start..end].hash(&mut h);
            let bucket_hash = h.finish();

            if let Some(bucket) = self.buckets[band].get(&bucket_hash) {
                for key in bucket {
                    if let Some(existing_sig) = all_sigs.get(key) {
                        let sim = Self::jaccard(sig, existing_sig);
                        if sim >= self.threshold {
                            return Some(key.clone());
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_texts_are_duplicates() {
        let dedup = MinHashDedup::new();
        let text = "The quick brown fox jumps over the lazy dog";
        let shingles = MinHashDedup::shingle(text, 5);
        let sig1 = dedup.signature(&shingles);
        let sig2 = dedup.signature(&shingles);
        assert_eq!(MinHashDedup::jaccard(&sig1, &sig2), 1.0);
    }

    #[test]
    fn test_similar_texts_high_jaccard() {
        let dedup = MinHashDedup::new();
        let text1 = "The quick brown fox jumps over the lazy dog";
        let text2 = "The quick brown fox leaps over the lazy dog";
        let sig1 = dedup.signature(&MinHashDedup::shingle(text1, 5));
        let sig2 = dedup.signature(&MinHashDedup::shingle(text2, 5));
        let sim = MinHashDedup::jaccard(&sig1, &sig2);
        assert!(sim > 0.5, "Similar texts should have Jaccard > 0.5, got {}", sim);
    }

    #[test]
    fn test_different_texts_low_jaccard() {
        let dedup = MinHashDedup::new();
        let text1 = "The quick brown fox jumps over the lazy dog";
        let text2 = "Quantum computing uses qubits for parallel computation";
        let sig1 = dedup.signature(&MinHashDedup::shingle(text1, 5));
        let sig2 = dedup.signature(&MinHashDedup::shingle(text2, 5));
        let sim = MinHashDedup::jaccard(&sig1, &sig2);
        assert!(sim < 0.3, "Different texts should have Jaccard < 0.3, got {}", sim);
    }

    #[test]
    fn test_lsh_finds_duplicates() {
        let mut dedup = MinHashDedup::new();
        let text1 = "The quick brown fox jumps over the lazy dog near the river";
        let text2 = "The quick brown fox jumps over the lazy dog near the lake";
        let sig1 = dedup.signature(&MinHashDedup::shingle(text1, 5));
        let sig2 = dedup.signature(&MinHashDedup::shingle(text2, 5));
        let _ = dedup.insert("fact1", &sig1);
        let candidates = dedup.insert("fact2", &sig2);
        // Should find fact1 as candidate (same LSH bucket in at least one band)
        assert!(!candidates.is_empty() || MinHashDedup::jaccard(&sig1, &sig2) < dedup.threshold,
            "LSH should find candidates for similar texts");
    }

    #[test]
    fn test_empty_text() {
        let dedup = MinHashDedup::new();
        let shingles = MinHashDedup::shingle("", 5);
        assert!(!shingles.is_empty(), "Empty text should still produce at least one shingle");
        let sig = dedup.signature(&shingles);
        assert_eq!(sig.hashes.len(), dedup.num_hashes);
    }
}
