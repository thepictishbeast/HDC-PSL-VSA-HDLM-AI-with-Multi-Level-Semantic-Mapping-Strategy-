// ============================================================
// Hallucination Detector — Verify AI claims against knowledge base
//
// Checks generated responses for claims not supported by brain.db.
// Flags unsupported assertions so they can be marked with lower
// confidence or removed before reaching the user.
//
// SUPERSOCIETY: With 59M facts, we can verify most factual claims.
// An AI that says "I'm 80% sure" when it's actually making things
// up is worse than one that honestly says "I don't know."
// ============================================================

use std::sync::Arc;
use crate::persistence::BrainDb;

/// Result of hallucination analysis on a response.
#[derive(Debug, Clone)]
pub struct HallucinationReport {
    /// Claims extracted from the response.
    pub claims: Vec<Claim>,
    /// How many claims were verified against the knowledge base.
    pub verified_count: usize,
    /// How many claims had no supporting evidence.
    pub unsupported_count: usize,
    /// Overall trustworthiness score (0.0 to 1.0).
    pub trust_score: f64,
    /// Whether the response should be flagged for review.
    pub flagged: bool,
}

/// A single factual claim extracted from a response.
#[derive(Debug, Clone)]
pub struct Claim {
    /// The claim text.
    pub text: String,
    /// Whether we found supporting evidence in brain.db.
    pub supported: bool,
    /// Best matching fact from the database (if any).
    pub evidence: Option<String>,
    /// Match confidence (0.0 to 1.0).
    pub match_confidence: f64,
}

pub struct HallucinationDetector {
    db: Arc<BrainDb>,
}

impl HallucinationDetector {
    pub fn new(db: Arc<BrainDb>) -> Self {
        Self { db }
    }

    /// Analyze a response for potential hallucinations.
    /// BUG ASSUMPTION: Claim extraction is heuristic-based.
    /// It works well for factual statements but may miss nuanced claims.
    pub fn analyze(&self, response: &str) -> HallucinationReport {
        let claims = self.extract_claims(response);
        let mut verified = 0;
        let mut unsupported = 0;
        let mut verified_claims = Vec::new();

        for claim_text in &claims {
            let (supported, evidence, confidence) = self.verify_claim(claim_text);
            if supported {
                verified += 1;
            } else {
                unsupported += 1;
            }
            verified_claims.push(Claim {
                text: claim_text.clone(),
                supported,
                evidence,
                match_confidence: confidence,
            });
        }

        let total = claims.len().max(1) as f64;
        let trust_score = if claims.is_empty() {
            0.8 // No verifiable claims = neutral trust
        } else {
            verified as f64 / total
        };

        HallucinationReport {
            claims: verified_claims,
            verified_count: verified,
            unsupported_count: unsupported,
            trust_score,
            flagged: trust_score < 0.5 && !claims.is_empty(),
        }
    }

    /// Extract verifiable factual claims from a response.
    /// Uses heuristic patterns to find declarative statements.
    fn extract_claims(&self, text: &str) -> Vec<String> {
        let mut claims = Vec::new();

        for sentence in text.split(|c: char| c == '.' || c == '!' || c == '\n') {
            let s = sentence.trim();
            if s.len() < 20 || s.len() > 300 { continue; }

            // Skip questions, commands, hedged statements
            if s.ends_with('?') { continue; }
            if s.starts_with("Please") || s.starts_with("Try") { continue; }
            if s.contains("I think") || s.contains("maybe") || s.contains("perhaps") {
                continue;
            }
            if s.starts_with("If ") || s.starts_with("When ") { continue; }

            // Look for factual claim patterns
            let is_factual = s.contains(" is ") || s.contains(" are ") ||
                s.contains(" was ") || s.contains(" were ") ||
                s.contains(" has ") || s.contains(" have ") ||
                s.contains(" uses ") || s.contains(" provides ") ||
                s.contains(" supports ") || s.contains(" enables ") ||
                s.contains(" operates ") || s.contains(" contains ") ||
                s.contains(" consists ") || s.contains(" implements ");

            // Contains specific entities (capitalized words, numbers, technical terms)
            let has_specifics = s.chars().any(|c| c.is_uppercase() && c != s.chars().next().unwrap_or('a')) ||
                s.chars().any(|c| c.is_numeric());

            if is_factual || has_specifics {
                claims.push(s.to_string());
            }
        }

        // Cap at 10 claims per response to prevent excessive DB queries
        claims.truncate(10);
        claims
    }

    /// Verify a single claim against the knowledge base.
    /// Returns (is_supported, best_evidence, match_confidence).
    fn verify_claim(&self, claim: &str) -> (bool, Option<String>, f64) {
        // Extract keywords for FTS5 search
        let stopwords = ["the", "and", "for", "are", "but", "not", "you", "all",
            "can", "had", "was", "one", "has", "its", "how", "who", "what",
            "this", "that", "with", "from", "they", "been", "have", "will",
            "each", "make", "like", "into", "than", "them", "some", "more"];
        let keywords: Vec<&str> = claim.split_whitespace()
            .filter(|w| w.len() >= 3 && !stopwords.contains(&w.to_lowercase().as_str()))
            .take(6)
            .collect();

        if keywords.len() < 2 {
            return (false, None, 0.0);
        }

        let query = keywords.join(" ");
        let results = self.db.search_facts(&query, 3);

        if results.is_empty() {
            return (false, None, 0.0);
        }

        // Check best match — keyword overlap between claim and fact
        let claim_lower = claim.to_lowercase();
        let mut best_score = 0.0f64;
        let mut best_fact = None;

        for (_, fact_value, quality) in &results {
            let fact_lower = fact_value.to_lowercase();
            let claim_words: std::collections::HashSet<&str> = claim_lower.split_whitespace()
                .filter(|w| w.len() >= 4)
                .collect();
            let fact_words: std::collections::HashSet<&str> = fact_lower.split_whitespace()
                .filter(|w| w.len() >= 4)
                .collect();

            if claim_words.is_empty() { continue; }

            let overlap = claim_words.intersection(&fact_words).count();
            let score = (overlap as f64 / claim_words.len() as f64) * quality;

            if score > best_score {
                best_score = score;
                best_fact = Some(crate::truncate_str(fact_value, 200).to_string());
            }
        }

        let supported = best_score >= 0.3;
        (supported, best_fact, best_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_detector() -> HallucinationDetector {
        let id = std::process::id();
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = PathBuf::from(format!("/tmp/plausiden_test_halluc_{}_{}.db", id, ts));
        let db = Arc::new(BrainDb::open(&path).unwrap());
        // Seed with test facts
        db.upsert_fact("tcp_fact", "TCP is a reliable transport protocol that provides ordered, error-checked delivery", "test", 0.9);
        db.upsert_fact("rust_fact", "Rust is a systems programming language focused on safety and performance", "test", 0.9);
        db.upsert_fact("python_fact", "Python is an interpreted high-level programming language created by Guido van Rossum", "test", 0.9);
        HallucinationDetector::new(db)
    }

    #[test]
    fn test_extract_claims() {
        let d = test_detector();
        let text = "TCP is a reliable protocol. It provides ordered delivery. Maybe it's fast? Please try again.";
        let claims = d.extract_claims(text);
        assert!(claims.len() >= 1, "Should extract at least 1 factual claim");
        assert!(!claims.iter().any(|c| c.contains("Maybe")), "Should skip hedged statements");
        assert!(!claims.iter().any(|c| c.contains("Please")), "Should skip commands");
    }

    #[test]
    fn test_analyze_supported() {
        let d = test_detector();
        let report = d.analyze("TCP is a reliable transport protocol that provides ordered delivery of data packets.");
        // With our test facts, this should find support
        assert!(report.claims.len() >= 1);
    }

    #[test]
    fn test_analyze_no_claims() {
        let d = test_detector();
        let report = d.analyze("Hello! How are you?");
        assert_eq!(report.claims.len(), 0);
        assert!(!report.flagged);
        assert_eq!(report.trust_score, 0.8); // Neutral trust for no claims
    }

    #[test]
    fn test_claim_cap() {
        let d = test_detector();
        let long_text = (0..20).map(|i| format!("Fact number {} is important and was discovered in 2024", i))
            .collect::<Vec<_>>().join(". ");
        let claims = d.extract_claims(&long_text);
        assert!(claims.len() <= 10, "Should cap at 10 claims");
    }
}
