// ============================================================
// 13-gram Bloom Decontamination Filter
// Sprint 1: Quality Ceiling — prevent test set leakage
//
// PURPOSE: Ensures training facts don't contain verbatim
// excerpts from evaluation benchmarks (MMLU, ARC, HellaSwag,
// TruthfulQA). Uses a Bloom filter on 13-character n-grams.
//
// BUG ASSUMPTION: 13-gram may be too long for short facts
// (< 13 chars). Fallback to full-string matching for short facts.
// ============================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Bloom filter for 13-gram decontamination.
pub struct BloomDecontaminator {
    /// Bit array
    bits: Vec<bool>,
    /// Number of hash functions
    num_hashes: usize,
    /// N-gram size
    ngram_size: usize,
}

impl BloomDecontaminator {
    /// Create a new Bloom filter.
    /// `capacity`: expected number of n-grams (determines false positive rate)
    pub fn new(capacity: usize) -> Self {
        // BUG ASSUMPTION: 10 bits per element gives ~1% false positive rate
        let num_bits = capacity * 10;
        Self {
            bits: vec![false; num_bits],
            num_hashes: 7,
            ngram_size: 13,
        }
    }

    fn hash_ngram(&self, ngram: &str, seed: usize) -> usize {
        let mut h = DefaultHasher::new();
        seed.hash(&mut h);
        ngram.hash(&mut h);
        (h.finish() as usize) % self.bits.len()
    }

    /// Add a test-set text's n-grams to the filter.
    pub fn add_test_text(&mut self, text: &str) {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() < self.ngram_size {
            // Short text: add as-is
            for i in 0..self.num_hashes {
                let idx = self.hash_ngram(text, i);
                self.bits[idx] = true;
            }
            return;
        }
        for window in chars.windows(self.ngram_size) {
            let ngram: String = window.iter().collect();
            for i in 0..self.num_hashes {
                let idx = self.hash_ngram(&ngram, i);
                self.bits[idx] = true;
            }
        }
    }

    /// Check if a training fact is contaminated (contains test-set n-grams).
    /// Returns the fraction of n-grams that match the filter (0.0 = clean, 1.0 = fully contaminated).
    pub fn contamination_score(&self, text: &str) -> f64 {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() < self.ngram_size {
            // Short text: check full match
            let mut all_match = true;
            for i in 0..self.num_hashes {
                let idx = self.hash_ngram(text, i);
                if !self.bits[idx] { all_match = false; break; }
            }
            return if all_match { 1.0 } else { 0.0 };
        }

        let total_ngrams = chars.len() - self.ngram_size + 1;
        let mut matches = 0;
        for window in chars.windows(self.ngram_size) {
            let ngram: String = window.iter().collect();
            let mut all_bits = true;
            for i in 0..self.num_hashes {
                let idx = self.hash_ngram(&ngram, i);
                if !self.bits[idx] { all_bits = false; break; }
            }
            if all_bits { matches += 1; }
        }

        matches as f64 / total_ngrams as f64
    }

    /// Check if contamination score exceeds threshold (default 0.8 = 80% overlap).
    pub fn is_contaminated(&self, text: &str, threshold: f64) -> bool {
        self.contamination_score(text) >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_is_contaminated() {
        let mut bloom = BloomDecontaminator::new(10000);
        let test_text = "What is the capital of France? The capital of France is Paris.";
        bloom.add_test_text(test_text);
        let score = bloom.contamination_score(test_text);
        assert!(score > 0.9, "Exact match should have high contamination score, got {}", score);
    }

    #[test]
    fn test_unrelated_text_is_clean() {
        let mut bloom = BloomDecontaminator::new(10000);
        bloom.add_test_text("What is the capital of France? The capital of France is Paris.");
        let score = bloom.contamination_score("Rust uses ownership and borrowing for memory safety without garbage collection.");
        assert!(score < 0.2, "Unrelated text should be clean, got {}", score);
    }

    #[test]
    fn test_partial_overlap() {
        let mut bloom = BloomDecontaminator::new(10000);
        bloom.add_test_text("The quick brown fox jumps over the lazy dog");
        let score = bloom.contamination_score("The quick brown fox runs through the forest");
        assert!(score > 0.0 && score < 1.0, "Partial overlap should be intermediate, got {}", score);
    }

    #[test]
    fn test_short_text_handling() {
        let mut bloom = BloomDecontaminator::new(10000);
        bloom.add_test_text("hello");
        assert!(bloom.is_contaminated("hello", 0.5));
        assert!(!bloom.is_contaminated("world", 0.5));
    }
}
