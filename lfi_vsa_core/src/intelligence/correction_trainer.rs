// ============================================================
// Correction Trainer — Learn from user feedback
//
// When a user says "that's wrong" or provides a correction,
// this module creates DPO training pairs:
//   (wrong_response → rejected, correct_response → chosen)
//
// These pairs are the highest-quality training data because
// they represent actual user preferences, not synthetic data.
//
// SUPERSOCIETY: A system that doesn't learn from mistakes
// is a calculator, not intelligence.
// ============================================================

use std::sync::Arc;
use crate::persistence::BrainDb;

/// A correction event: user rejected AI response and provided the right answer.
#[derive(Debug, Clone)]
pub struct CorrectionEvent {
    pub user_input: String,
    pub wrong_response: String,
    pub correct_response: String,
    pub domain: Option<String>,
    pub timestamp: u64,
}

/// A DPO training pair derived from a correction.
#[derive(Debug, Clone)]
pub struct DpoPair {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    pub domain: Option<String>,
    pub source: String,
}

/// Manages correction-based learning.
pub struct CorrectionTrainer {
    db: Arc<BrainDb>,
    corrections: Vec<CorrectionEvent>,
}

impl CorrectionTrainer {
    pub fn new(db: Arc<BrainDb>) -> Self {
        Self {
            db,
            corrections: Vec::new(),
        }
    }

    /// Record a user correction.
    pub fn record_correction(&mut self, event: CorrectionEvent) {
        // Store in DB for persistence
        self.db.record_version(
            &format!("correction_{}", self.corrections.len()),
            Some(&event.wrong_response),
            &event.correct_response,
            Some(0.3), // wrong response gets low quality
            Some(0.95), // correct response gets high quality
            "user_correction",
            "user",
            Some(&event.user_input),
        );

        self.corrections.push(event);
    }

    /// Extract a correction from user input patterns.
    /// Returns Some(correct_response) if the input looks like a correction.
    pub fn detect_correction(input: &str) -> Option<String> {
        let lower = input.to_lowercase();

        // Pattern: "actually, X" or "no, X" or "the correct answer is X"
        let correction_prefixes = [
            "actually,", "actually ", "no, it's", "no its", "no, the",
            "the correct answer is", "the right answer is", "it should be",
            "you're wrong,", "youre wrong,", "that's wrong,", "thats wrong,",
            "incorrect,", "wrong,", "not right,", "correction:",
        ];

        for prefix in &correction_prefixes {
            if lower.starts_with(prefix) || lower.contains(&format!(" {}", prefix)) {
                // Extract the correction text after the prefix
                if let Some(pos) = lower.find(prefix) {
                    let after = &input[pos + prefix.len()..].trim();
                    if after.len() > 5 {
                        return Some(after.to_string());
                    }
                }
            }
        }

        None
    }

    /// Convert all recorded corrections to DPO pairs.
    pub fn to_dpo_pairs(&self) -> Vec<DpoPair> {
        self.corrections.iter().map(|c| {
            DpoPair {
                prompt: c.user_input.clone(),
                chosen: c.correct_response.clone(),
                rejected: c.wrong_response.clone(),
                domain: c.domain.clone(),
                source: "user_correction".to_string(),
            }
        }).collect()
    }

    /// Export DPO pairs as JSONL for training.
    pub fn export_jsonl(&self) -> String {
        let pairs = self.to_dpo_pairs();
        pairs.iter().map(|p| {
            serde_json::json!({
                "prompt": p.prompt,
                "chosen": p.chosen,
                "rejected": p.rejected,
                "domain": p.domain,
                "source": p.source,
            }).to_string()
        }).collect::<Vec<_>>().join("\n")
    }

    /// Count total corrections recorded.
    pub fn correction_count(&self) -> usize {
        self.corrections.len()
    }
}

use serde_json;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_trainer() -> CorrectionTrainer {
        let id = std::process::id();
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = PathBuf::from(format!("/tmp/plausiden_test_corr_{}_{}.db", id, ts));
        let db = Arc::new(BrainDb::open(&path).unwrap());
        CorrectionTrainer::new(db)
    }

    #[test]
    fn test_record_correction() {
        let mut trainer = test_trainer();
        trainer.record_correction(CorrectionEvent {
            user_input: "What is the capital of France?".to_string(),
            wrong_response: "The capital of France is Berlin.".to_string(),
            correct_response: "The capital of France is Paris.".to_string(),
            domain: Some("geography".to_string()),
            timestamp: 1234567890,
        });
        assert_eq!(trainer.correction_count(), 1);
    }

    #[test]
    fn test_detect_correction() {
        assert!(CorrectionTrainer::detect_correction("Actually, the answer is Paris").is_some());
        assert!(CorrectionTrainer::detect_correction("No, it's TCP not UDP").is_some());
        assert!(CorrectionTrainer::detect_correction("The correct answer is 42").is_some());
        assert!(CorrectionTrainer::detect_correction("Hello, how are you?").is_none());
        assert!(CorrectionTrainer::detect_correction("Thanks!").is_none());
    }

    #[test]
    fn test_dpo_export() {
        let mut trainer = test_trainer();
        trainer.record_correction(CorrectionEvent {
            user_input: "What language is Rust?".to_string(),
            wrong_response: "Rust is a scripting language.".to_string(),
            correct_response: "Rust is a systems programming language.".to_string(),
            domain: Some("programming".to_string()),
            timestamp: 0,
        });

        let pairs = trainer.to_dpo_pairs();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].chosen, "Rust is a systems programming language.");
        assert_eq!(pairs[0].rejected, "Rust is a scripting language.");

        let jsonl = trainer.export_jsonl();
        assert!(jsonl.contains("chosen"));
        assert!(jsonl.contains("rejected"));
    }
}
