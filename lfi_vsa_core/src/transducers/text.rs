// ============================================================
// Text Transducer — Character N-Gram Projection into VSA Space
// Section 1.IV: Unified Hyperdimensional Sensorium
//
// Strategy: Character-level n-gram encoding with positional
// awareness. This creates a language-agnostic text fingerprint.
//
//   1. Each unique character gets a random base vector (alphabet).
//   2. Character n-grams are encoded via permutation binding:
//      ngram(c1,c2,c3) = permute(c1,0) XOR permute(c2,1) XOR permute(c3,2)
//   3. All n-gram vectors are bundled into a "bag of n-grams"
//      superposition.
//   4. Optional: document-level positional encoding overlaid
//      for sequence-sensitive applications.
//
// Properties:
//   - Similar texts share n-grams -> high cosine similarity
//   - Completely different texts -> quasi-orthogonal
//   - Language-agnostic: works on any UTF-8 byte sequence
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use std::collections::HashMap;

/// Default n-gram size. Trigrams capture morpheme-level features.
const DEFAULT_NGRAM_SIZE: usize = 3;

/// Maximum number of unique characters to track in the alphabet.
/// Beyond this, characters share vectors (graceful collision).
const MAX_ALPHABET_SIZE: usize = 512;

/// Transducer for projecting text into the VSA space.
pub struct TextTransducer {
    /// Character alphabet: maps byte values to unique base vectors.
    alphabet: HashMap<u8, BipolarVector>,
    /// N-gram window size.
    ngram_size: usize,
}

impl TextTransducer {
    /// Create a new TextTransducer with the default n-gram size (3).
    /// The alphabet is built lazily from the input text.
    pub fn new() -> Result<Self, HdcError> {
        debuglog!("TextTransducer::new: ngram_size={}", DEFAULT_NGRAM_SIZE);
        Ok(Self {
            alphabet: HashMap::new(),
            ngram_size: DEFAULT_NGRAM_SIZE,
        })
    }

    /// Create with a custom n-gram size.
    pub fn with_ngram_size(n: usize) -> Result<Self, HdcError> {
        if n == 0 {
            debuglog!("TextTransducer::with_ngram_size: FAIL - n=0");
            return Err(HdcError::InitializationFailed {
                reason: "N-gram size must be at least 1".to_string(),
            });
        }
        debuglog!("TextTransducer::with_ngram_size: n={}", n);
        Ok(Self {
            alphabet: HashMap::new(),
            ngram_size: n,
        })
    }

    /// Get or create the base vector for a character byte.
    fn get_char_vector(&mut self, byte: u8) -> Result<&BipolarVector, HdcError> {
        if !self.alphabet.contains_key(&byte) {
            if self.alphabet.len() >= MAX_ALPHABET_SIZE {
                // Collision: reuse vector for byte % existing size.
                // This is a graceful degradation, not a failure.
                debuglog!(
                    "TextTransducer::get_char_vector: alphabet full, byte={} will collide",
                    byte
                );
                let collision_key = (byte as usize % self.alphabet.len()) as u8;
                // Return reference to existing entry via the collision key
                // We need to find an existing key that maps to this collision
                let existing_key = self.alphabet.keys()
                    .nth(collision_key as usize % self.alphabet.len());
                if let Some(&k) = existing_key {
                    debuglog!(
                        "TextTransducer::get_char_vector: collision byte={} -> key={}",
                        byte, k
                    );
                    // Insert a clone of the colliding vector
                    let cloned = self.alphabet.get(&k)
                        .ok_or_else(|| HdcError::InitializationFailed {
                            reason: "Alphabet collision lookup failed".to_string(),
                        })?.clone();
                    self.alphabet.insert(byte, cloned);
                } else {
                    return Err(HdcError::InitializationFailed {
                        reason: "Alphabet is in an inconsistent state".to_string(),
                    });
                }
            } else {
                debuglog!("TextTransducer::get_char_vector: new char byte={}", byte);
                let v = BipolarVector::new_random()?;
                self.alphabet.insert(byte, v);
            }
        }
        self.alphabet.get(&byte).ok_or_else(|| HdcError::InitializationFailed {
            reason: format!("Failed to retrieve alphabet vector for byte {}", byte),
        })
    }

    /// Project a text string into a bipolar hypervector.
    ///
    /// The text is treated as a sequence of UTF-8 bytes.
    /// N-grams are extracted and each is encoded as a bound
    /// composition of character vectors with positional permutations.
    pub fn project(&mut self, text: &str) -> Result<BipolarVector, HdcError> {
        debuglog!("TextTransducer::project: entry, text_len={}", text.len());

        let bytes = text.as_bytes();
        if bytes.is_empty() {
            debuglog!("TextTransducer::project: FAIL - empty text");
            return Err(HdcError::InitializationFailed {
                reason: "Cannot project empty text".to_string(),
            });
        }

        if bytes.len() < self.ngram_size {
            // Text shorter than n-gram size: use the entire text as one "n-gram"
            debuglog!(
                "TextTransducer::project: text shorter than ngram_size, using full text as single unit"
            );
            return self.encode_ngram(bytes);
        }

        // Extract all n-grams and encode them
        let ngram_count = bytes.len() - self.ngram_size + 1;
        debuglog!("TextTransducer::project: extracting {} n-grams", ngram_count);

        let mut ngram_vectors: Vec<BipolarVector> = Vec::with_capacity(ngram_count);

        for i in 0..ngram_count {
            let ngram = &bytes[i..i + self.ngram_size];
            let ngram_vec = self.encode_ngram(ngram)?;
            ngram_vectors.push(ngram_vec);

            if i % 500 == 0 {
                debuglog!("TextTransducer::project: processed n-gram {}/{}", i + 1, ngram_count);
            }
        }

        // Bundle all n-gram vectors into a single text fingerprint
        let ngram_refs: Vec<&BipolarVector> = ngram_vectors.iter().collect();
        let result = BipolarVector::bundle(&ngram_refs)?;

        debuglog!(
            "TextTransducer::project: SUCCESS, ngrams={}, dim={}, ones={}",
            ngram_count, result.dim(), result.count_ones()
        );
        Ok(result)
    }

    /// Encode a single n-gram (byte slice) into a hypervector.
    ///
    /// ngram(c1,c2,...,cn) = permute(c1,0) XOR permute(c2,1) XOR ... XOR permute(cn, n-1)
    fn encode_ngram(&mut self, ngram: &[u8]) -> Result<BipolarVector, HdcError> {
        debuglog!("TextTransducer::encode_ngram: len={}", ngram.len());

        if ngram.is_empty() {
            return Err(HdcError::InitializationFailed {
                reason: "Cannot encode empty n-gram".to_string(),
            });
        }

        // Start with the first character's vector
        let first_char_vec = self.get_char_vector(ngram[0])?.clone();
        let mut result = first_char_vec.permute(0)?;

        // Bind in subsequent characters with positional permutations
        for (pos, &byte) in ngram.iter().enumerate().skip(1) {
            let char_vec = self.get_char_vector(byte)?.clone();
            let positioned = char_vec.permute(pos)?;
            result = result.bind(&positioned)?;
        }

        debuglog!("TextTransducer::encode_ngram: SUCCESS, dim={}", result.dim());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_basic() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        let hv = td.project("Hello, world!")?;
        assert_eq!(hv.dim(), 10000);
        assert!(hv.count_ones() > 0);
        Ok(())
    }

    #[test]
    fn test_project_empty_fails() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        assert!(td.project("").is_err());
        Ok(())
    }

    #[test]
    fn test_project_short_text() -> Result<(), HdcError> {
        // Text shorter than n-gram size
        let mut td = TextTransducer::new()?;
        let hv = td.project("Hi")?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_project_single_char() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        let hv = td.project("x")?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_similar_texts_high_similarity() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        let hv1 = td.project("the quick brown fox")?;
        let hv2 = td.project("the quick brown fox jumps")?;
        let sim = hv1.similarity(&hv2)?;
        debuglog!("test_similar_texts: sim={:.4}", sim);
        // Overlapping n-grams should produce positive similarity
        assert!(sim > 0.1, "Similar texts should have positive similarity, sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_different_texts_low_similarity() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        let hv1 = td.project("aaaaaaaaaa")?;
        let hv2 = td.project("zzzzzzzzzz")?;
        let sim = hv1.similarity(&hv2)?;
        debuglog!("test_different_texts: sim={:.4}", sim);
        // Completely different character sets should be quasi-orthogonal
        assert!(sim.abs() < 0.5, "Different texts should diverge, sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_custom_ngram_size() -> Result<(), HdcError> {
        let mut td = TextTransducer::with_ngram_size(5)?;
        let hv = td.project("Hello, world! This is a test.")?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_zero_ngram_size_fails() {
        let result = TextTransducer::with_ngram_size(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_text() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        let hv = td.project("forensic analysis")?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for TextTransducer
    // ============================================================

    /// INVARIANT: project produces 10k-dim vector for any non-empty text.
    #[test]
    fn invariant_project_produces_10k_dim() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        for input in ["a", "hello", "the quick brown fox", "アリス", "🦀"] {
            let v = td.project(input)?;
            assert_eq!(v.dim(), 10_000,
                "projection for {:?} must be 10k dim", input);
        }
        Ok(())
    }

    /// INVARIANT: project never panics on arbitrary unicode / control input.
    #[test]
    fn invariant_project_safe_on_arbitrary_unicode() -> Result<(), HdcError> {
        let mut td = TextTransducer::new()?;
        let big = "x".repeat(10_000);
        let inputs: [&str; 5] = [
            "アリスは猫を見た",
            "🦀🦀🦀",
            "control:\x00\x01\x1f",
            "mixed: ASCII日本語🦀",
            &big,
        ];
        for input in inputs {
            // Must not panic.
            let _ = td.project(input)?;
        }
        Ok(())
    }

    /// INVARIANT: new() and with_ngram_size reject invalid ngram sizes (0).
    #[test]
    fn invariant_zero_ngram_rejected() {
        assert!(TextTransducer::with_ngram_size(0).is_err(),
            "zero ngram_size must error");
    }

    /// INVARIANT: with_ngram_size accepts reasonable sizes (1..20).
    #[test]
    fn invariant_reasonable_ngram_sizes_accepted() -> Result<(), HdcError> {
        for n in [1usize, 2, 3, 5, 8, 20] {
            let _ = TextTransducer::with_ngram_size(n)?;
        }
        Ok(())
    }
}
