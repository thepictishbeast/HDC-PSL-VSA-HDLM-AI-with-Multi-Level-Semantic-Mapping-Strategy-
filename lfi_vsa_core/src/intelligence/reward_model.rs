// ============================================================
// Reward Model Classifier — Training Data Quality Scoring
//
// Scores instruction-output pairs on multiple dimensions:
// - Coherence: Does the output logically follow the instruction?
// - Completeness: Is the answer thorough enough?
// - Safety: Does the output avoid harmful/toxic content?
// - Formatting: Is the output well-structured?
// - Domain accuracy: Does the answer match domain expectations?
//
// Used to filter training data before fine-tuning (ORPO/GRPO).
// SUPERSOCIETY: Quality > quantity. 1000 high-quality pairs beat
// 100,000 mediocre ones.
// ============================================================

use std::collections::HashSet;

/// Quality dimensions scored by the reward model.
#[derive(Debug, Clone)]
pub struct QualityScores {
    pub coherence: f64,
    pub completeness: f64,
    pub safety: f64,
    pub formatting: f64,
    pub domain_accuracy: f64,
    pub overall: f64,
}

/// A training pair to be evaluated.
#[derive(Debug, Clone)]
pub struct TrainingPair {
    pub instruction: String,
    pub output: String,
    pub domain: Option<String>,
}

/// Classification result for a training pair.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub scores: QualityScores,
    pub tier: QualityTier,
    pub flags: Vec<String>,
    pub recommended_action: Action,
}

/// Quality tiers for training data.
#[derive(Debug, Clone, PartialEq)]
pub enum QualityTier {
    /// Score >= 0.8: Ready for fine-tuning
    Premium,
    /// Score 0.6-0.8: Needs review but usable
    Standard,
    /// Score 0.4-0.6: Needs editing before use
    Draft,
    /// Score < 0.4: Should be discarded or rewritten
    Reject,
}

impl QualityTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Premium => "premium",
            Self::Standard => "standard",
            Self::Draft => "draft",
            Self::Reject => "reject",
        }
    }
}

/// Recommended action for a classified pair.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Include directly in fine-tuning dataset
    Include,
    /// Include but flag for human review
    ReviewThenInclude,
    /// Edit the output before including
    EditRequired,
    /// Discard entirely
    Discard,
}

/// Heuristic reward model — scores training pairs without an LLM call.
/// Fast enough to classify millions of pairs offline.
pub struct RewardClassifier {
    toxic_patterns: HashSet<String>,
    filler_phrases: Vec<String>,
}

impl RewardClassifier {
    pub fn new() -> Self {
        let toxic_patterns: HashSet<String> = [
            "kill", "bomb", "hack into", "steal", "exploit vulnerability",
            "inject malware", "social engineer", "phish", "ddos",
        ].iter().map(|s| s.to_string()).collect();

        let filler_phrases = vec![
            "as an ai".to_string(),
            "i cannot".to_string(),
            "i'm sorry".to_string(),
            "i apologize".to_string(),
            "it's important to note".to_string(),
            "in conclusion".to_string(),
            "first and foremost".to_string(),
            "it goes without saying".to_string(),
        ];

        Self { toxic_patterns, filler_phrases }
    }

    /// Classify a single training pair.
    /// BUG ASSUMPTION: This is a heuristic classifier — it catches obvious issues
    /// but cannot evaluate factual accuracy. Use alongside Ollama-based verification
    /// for critical training data.
    pub fn classify(&self, pair: &TrainingPair) -> ClassificationResult {
        let coherence = self.score_coherence(pair);
        let completeness = self.score_completeness(pair);
        let safety = self.score_safety(pair);
        let formatting = self.score_formatting(pair);
        let domain_accuracy = self.score_domain(pair);

        // Weighted average — safety has veto power
        let overall = if safety < 0.3 {
            safety * 0.5 // Safety failure tanks the score
        } else {
            coherence * 0.25 + completeness * 0.25 + safety * 0.2 +
            formatting * 0.15 + domain_accuracy * 0.15
        };

        let tier = match overall {
            s if s >= 0.8 => QualityTier::Premium,
            s if s >= 0.6 => QualityTier::Standard,
            s if s >= 0.4 => QualityTier::Draft,
            _ => QualityTier::Reject,
        };

        let action = match &tier {
            QualityTier::Premium => Action::Include,
            QualityTier::Standard => Action::ReviewThenInclude,
            QualityTier::Draft => Action::EditRequired,
            QualityTier::Reject => Action::Discard,
        };

        let mut flags = Vec::new();
        if coherence < 0.5 { flags.push("low_coherence".to_string()); }
        if completeness < 0.5 { flags.push("incomplete".to_string()); }
        if safety < 0.5 { flags.push("safety_concern".to_string()); }
        if formatting < 0.5 { flags.push("poor_formatting".to_string()); }
        if pair.output.len() < 50 { flags.push("very_short".to_string()); }
        if pair.output.len() > 4000 { flags.push("very_long".to_string()); }

        ClassificationResult {
            scores: QualityScores {
                coherence, completeness, safety, formatting, domain_accuracy, overall,
            },
            tier,
            flags,
            recommended_action: action,
        }
    }

    /// Batch classify many pairs. Returns (results, summary stats).
    pub fn classify_batch(&self, pairs: &[TrainingPair]) -> (Vec<ClassificationResult>, BatchStats) {
        let results: Vec<ClassificationResult> = pairs.iter()
            .map(|p| self.classify(p))
            .collect();

        let mut stats = BatchStats {
            total: results.len(),
            premium: 0, standard: 0, draft: 0, reject: 0,
            avg_overall: 0.0,
            avg_coherence: 0.0,
            avg_completeness: 0.0,
            avg_safety: 0.0,
        };

        for r in &results {
            match r.tier {
                QualityTier::Premium => stats.premium += 1,
                QualityTier::Standard => stats.standard += 1,
                QualityTier::Draft => stats.draft += 1,
                QualityTier::Reject => stats.reject += 1,
            }
            stats.avg_overall += r.scores.overall;
            stats.avg_coherence += r.scores.coherence;
            stats.avg_completeness += r.scores.completeness;
            stats.avg_safety += r.scores.safety;
        }

        let n = results.len().max(1) as f64;
        stats.avg_overall /= n;
        stats.avg_coherence /= n;
        stats.avg_completeness /= n;
        stats.avg_safety /= n;

        (results, stats)
    }

    // ---- Scoring dimensions ----

    fn score_coherence(&self, pair: &TrainingPair) -> f64 {
        let inst = &pair.instruction.to_lowercase();
        let out = &pair.output.to_lowercase();

        let mut score: f64 = 0.5; // Base

        // Keyword overlap: instruction keywords appearing in output
        let inst_keywords: HashSet<&str> = inst.split_whitespace()
            .filter(|w| w.len() >= 4)
            .collect();
        let out_words: HashSet<&str> = out.split_whitespace()
            .filter(|w| w.len() >= 4)
            .collect();

        if !inst_keywords.is_empty() {
            let overlap = inst_keywords.intersection(&out_words).count();
            let ratio = overlap as f64 / inst_keywords.len() as f64;
            score += ratio * 0.3;
        }

        // Question → answer pattern check
        if (inst.contains('?') || inst.starts_with("explain") || inst.starts_with("describe")
            || inst.starts_with("what") || inst.starts_with("how"))
            && !out.trim().is_empty()
        {
            score += 0.1;
        }

        // Penalize if output is just the instruction repeated
        if out.starts_with(&inst[..inst.len().min(50).min(out.len())]) {
            score -= 0.3;
        }

        // Penalize refusals for non-harmful instructions
        let is_refusal = self.filler_phrases.iter().any(|f| out.starts_with(f));
        let is_harmful = self.toxic_patterns.iter().any(|t| inst.contains(t.as_str()));
        if is_refusal && !is_harmful {
            score -= 0.2;
        }

        score.clamp(0.0, 1.0)
    }

    fn score_completeness(&self, pair: &TrainingPair) -> f64 {
        let out_len = pair.output.len();
        let inst_len = pair.instruction.len();

        let mut score: f64 = 0.5;

        // Length ratio — output should generally be longer than input
        if out_len > inst_len * 2 {
            score += 0.2;
        } else if out_len < inst_len / 2 {
            score -= 0.2;
        }

        // Absolute length thresholds
        match out_len {
            0..=30 => score -= 0.3,
            31..=100 => score -= 0.1,
            101..=500 => score += 0.1,
            501..=2000 => score += 0.2,
            _ => score += 0.1, // Very long might be verbose
        }

        // Contains structured elements (lists, code, explanations)
        if pair.output.contains('\n') { score += 0.05; }
        if pair.output.contains("```") { score += 0.1; }
        if pair.output.contains("1.") || pair.output.contains("- ") { score += 0.05; }

        // Ends properly (not mid-sentence)
        let trimmed = pair.output.trim();
        if trimmed.ends_with('.') || trimmed.ends_with('!') || trimmed.ends_with('?')
            || trimmed.ends_with("```") || trimmed.ends_with('}')
        {
            score += 0.1;
        } else {
            score -= 0.1; // Truncated
        }

        score.clamp(0.0, 1.0)
    }

    fn score_safety(&self, pair: &TrainingPair) -> f64 {
        let out = pair.output.to_lowercase();
        let mut score: f64 = 1.0;

        // Check for toxic patterns in output (not instruction — user may ask about security)
        let inst_is_educational = pair.instruction.to_lowercase().contains("explain")
            || pair.instruction.to_lowercase().contains("what is")
            || pair.instruction.to_lowercase().contains("how does");

        for pattern in &self.toxic_patterns {
            if out.contains(pattern.as_str()) && !inst_is_educational {
                score -= 0.15;
            }
        }

        // PII patterns — each match reduces score independently
        for pii in &["password:", "api_key:", "secret:", "token:", "credential:"] {
            if out.contains(pii) {
                score -= 0.2;
            }
        }

        // Hallucination markers
        if out.contains("as of my last update") || out.contains("as of my training") {
            score -= 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    fn score_formatting(&self, pair: &TrainingPair) -> f64 {
        let out = &pair.output;
        let mut score: f64 = 0.6;

        // Proper paragraph structure
        let lines: Vec<&str> = out.lines().collect();
        if lines.len() > 1 { score += 0.1; }

        // Not a wall of text (paragraphs separated)
        if out.contains("\n\n") { score += 0.1; }

        // Code blocks properly closed
        let code_opens = out.matches("```").count();
        if code_opens > 0 && code_opens % 2 == 0 { score += 0.1; }
        if code_opens > 0 && code_opens % 2 != 0 { score -= 0.2; }

        // Excessive filler
        let filler_count = self.filler_phrases.iter()
            .filter(|f| out.to_lowercase().contains(f.as_str()))
            .count();
        score -= filler_count as f64 * 0.05;

        // Starts with content, not filler
        let first_line = out.lines().next().unwrap_or("").to_lowercase();
        if first_line.starts_with("sure") || first_line.starts_with("of course")
            || first_line.starts_with("great question")
        {
            score -= 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    fn score_domain(&self, pair: &TrainingPair) -> f64 {
        let domain = pair.domain.as_deref().unwrap_or("general");
        let out = &pair.output.to_lowercase();

        let mut score: f64 = 0.6;

        // Domain-specific keyword presence
        let domain_keywords: Vec<&str> = match domain {
            "cybersecurity" => vec!["security", "vulnerability", "attack", "defense", "protocol",
                                   "encryption", "firewall", "malware", "threat", "exploit"],
            "programming" => vec!["function", "code", "variable", "return", "class", "method",
                                  "algorithm", "data", "error", "type"],
            "physics" => vec!["energy", "force", "mass", "velocity", "quantum", "particle",
                              "wave", "field", "momentum", "gravity"],
            "mathematics" => vec!["equation", "theorem", "proof", "function", "variable",
                                  "integral", "derivative", "matrix", "vector", "set"],
            "biology" => vec!["cell", "gene", "protein", "organism", "evolution", "dna",
                              "enzyme", "membrane", "species", "mutation"],
            "conversational" => vec!["think", "feel", "enjoy", "like", "would", "could",
                                     "interesting", "great", "thanks", "sure"],
            _ => vec![],
        };

        if !domain_keywords.is_empty() {
            let matches = domain_keywords.iter()
                .filter(|kw| out.contains(*kw))
                .count();
            let ratio = matches as f64 / domain_keywords.len().min(5) as f64;
            score += ratio * 0.3;
        }

        score.clamp(0.0, 1.0)
    }
}

/// Summary statistics for a batch classification.
#[derive(Debug, Clone)]
pub struct BatchStats {
    pub total: usize,
    pub premium: usize,
    pub standard: usize,
    pub draft: usize,
    pub reject: usize,
    pub avg_overall: f64,
    pub avg_coherence: f64,
    pub avg_completeness: f64,
    pub avg_safety: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classifier() -> RewardClassifier {
        RewardClassifier::new()
    }

    #[test]
    fn test_good_pair() {
        let c = classifier();
        let pair = TrainingPair {
            instruction: "Explain how TCP/IP works".to_string(),
            output: "TCP/IP (Transmission Control Protocol/Internet Protocol) is a layered protocol suite that enables communication between computers over networks.\n\n1. **Application Layer**: Where user-facing protocols like HTTP, FTP, and SMTP operate.\n2. **Transport Layer**: TCP provides reliable, ordered delivery of data using acknowledgments and retransmission.\n3. **Internet Layer**: IP handles addressing and routing packets across network boundaries.\n4. **Link Layer**: Manages physical transmission of data frames.\n\nTCP ensures reliability through a three-way handshake (SYN, SYN-ACK, ACK) and sequence numbers for packet ordering.".to_string(),
            domain: Some("cybersecurity".to_string()),
        };
        let result = c.classify(&pair);
        assert!(result.scores.overall >= 0.6, "Good pair should score >= 0.6, got {}", result.scores.overall);
        assert!(result.tier == QualityTier::Premium || result.tier == QualityTier::Standard);
    }

    #[test]
    fn test_short_pair() {
        let c = classifier();
        let pair = TrainingPair {
            instruction: "What is encryption?".to_string(),
            output: "It encrypts data.".to_string(),
            domain: Some("cybersecurity".to_string()),
        };
        let result = c.classify(&pair);
        assert!(result.scores.completeness < 0.5, "Short answer should have low completeness");
        assert!(result.flags.contains(&"very_short".to_string()));
    }

    #[test]
    fn test_refusal_pair() {
        let c = classifier();
        let pair = TrainingPair {
            instruction: "Explain how firewalls work".to_string(),
            output: "I'm sorry, I cannot help with that request. As an AI, I don't have the ability to explain technical concepts.".to_string(),
            domain: Some("cybersecurity".to_string()),
        };
        let result = c.classify(&pair);
        assert!(result.scores.coherence < 0.6, "Unnecessary refusal should reduce coherence");
    }

    #[test]
    fn test_batch_classification() {
        let c = classifier();
        let pairs = vec![
            TrainingPair {
                instruction: "What is Rust?".to_string(),
                output: "Rust is a systems programming language focused on safety, speed, and concurrency. It achieves memory safety without garbage collection through its ownership system.".to_string(),
                domain: Some("programming".to_string()),
            },
            TrainingPair {
                instruction: "Hello".to_string(),
                output: "Hi".to_string(),
                domain: Some("conversational".to_string()),
            },
        ];
        let (results, stats) = c.classify_batch(&pairs);
        assert_eq!(results.len(), 2);
        assert_eq!(stats.total, 2);
        assert!(stats.avg_overall > 0.0);
    }

    #[test]
    fn test_safety_scoring() {
        let c = classifier();
        let pair = TrainingPair {
            instruction: "Tell me something".to_string(),
            output: "Here is the password: admin123 and the api_key: sk-12345".to_string(),
            domain: None,
        };
        let result = c.classify(&pair);
        assert!(result.scores.safety < 0.7, "PII in output should lower safety score, got {}", result.scores.safety);
    }

    #[test]
    fn test_quality_tier_ordering() {
        assert_eq!(QualityTier::Premium.as_str(), "premium");
        assert_eq!(QualityTier::Standard.as_str(), "standard");
        assert_eq!(QualityTier::Draft.as_str(), "draft");
        assert_eq!(QualityTier::Reject.as_str(), "reject");
    }
}
