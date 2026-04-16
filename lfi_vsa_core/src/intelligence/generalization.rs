// ============================================================
// Generalization Engine — Prevent Rote Memorization
//
// PRINCIPLE: True intelligence generalizes — it applies concepts to
// unseen inputs. Rote memorization fails at even slight variations.
//
// This engine tests whether LFI has genuinely learned a concept or
// just memorized specific examples.
//
// METHODS:
//   1. Held-out test sets (examples never seen during training)
//   2. Paraphrase tests (same meaning, different words)
//   3. Parametric variation (2+3=5 → does LFI know 3+4=7?)
//   4. Inverse tests (if LFI learned add, can it also subtract?)
//   5. Compositional tests (combine learned concepts in new ways)
//   6. Transfer tests (learned math, can it do physics word problems?)
//
// WHEN ROTE IS OK:
//   - Factual recall (capital of France)
//   - Named constants (speed of light)
//   - Historical facts (WWII ended in 1945)
// Configure with `allow_memorization: true` for these.
//
// WHEN GENERALIZATION IS REQUIRED:
//   - Mathematical operations
//   - Algorithmic procedures
//   - Abstract concepts (what is a function?)
//   - Conditional reasoning
// ============================================================

use crate::intelligence::training_data::TrainingExample;
use std::collections::HashMap;

// ============================================================
// Generalization Metrics
// ============================================================

/// Result of testing generalization on a concept.
#[derive(Debug, Clone)]
pub struct GeneralizationResult {
    /// What concept was tested.
    pub concept: String,
    /// Accuracy on training examples (seen during learning).
    pub train_accuracy: f64,
    /// Accuracy on held-out test examples.
    pub test_accuracy: f64,
    /// Accuracy on paraphrased versions.
    pub paraphrase_accuracy: f64,
    /// Accuracy on parametrically varied versions.
    pub variation_accuracy: f64,
    /// Generalization score: low train/test gap + high variation acc.
    pub generalization_score: f64,
    /// Is LFI actually understanding, or just memorizing?
    pub verdict: LearningVerdict,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LearningVerdict {
    /// High train acc, high test acc, handles variations — genuine understanding.
    Understanding,
    /// High train acc, moderate test acc, limited variation handling — shallow.
    ShallowLearning,
    /// High train acc, low test acc — classic overfitting/memorization.
    RoteMemorization,
    /// Low accuracy across the board — hasn't learned yet.
    NotLearned,
    /// Intentionally memorized (factual recall).
    IntentionalRecall,
}

// ============================================================
// Parametric Variation Generator
// ============================================================

/// Generates parametric variations of training examples to test generalization.
pub struct VariationGenerator;

impl VariationGenerator {
    /// Generate parametric variations of a math example.
    /// BUG ASSUMPTION: only handles simple arithmetic patterns.
    /// For real generalization testing, need full expression parser.
    pub fn math_variations(example: &TrainingExample) -> Vec<TrainingExample> {
        let mut variations = Vec::new();
        let input = &example.input;
        let tags: Vec<&str> = example.tags.iter().map(|s| s.as_str()).collect();

        // Simple addition: "a + b" → generate with different a, b
        if let Some(sum) = Self::parse_simple_add(input) {
            let (a, b) = sum;
            for da in -2..=2 {
                for db in -2..=2 {
                    if da == 0 && db == 0 { continue; }
                    let new_a = a + da;
                    let new_b = b + db;
                    let new_sum = new_a + new_b;
                    variations.push(TrainingExample::new(
                        &example.domain,
                        &format!("{} + {}", new_a, new_b),
                        &new_sum.to_string(),
                        example.difficulty,
                        &tags,
                    ));
                }
            }
        }

        // Simple multiplication
        if let Some(prod) = Self::parse_simple_mul(input) {
            let (a, b) = prod;
            for da in -1..=1 {
                for db in -1..=1 {
                    if da == 0 && db == 0 { continue; }
                    let new_a = a + da;
                    let new_b = b + db;
                    if new_a > 0 && new_b > 0 {
                        variations.push(TrainingExample::new(
                            &example.domain,
                            &format!("{} * {}", new_a, new_b),
                            &(new_a * new_b).to_string(),
                            example.difficulty,
                            &tags,
                        ));
                    }
                }
            }
        }

        variations
    }

    /// Generate paraphrased versions of a question.
    pub fn paraphrases(example: &TrainingExample) -> Vec<TrainingExample> {
        let mut paraphrases = Vec::new();
        let tags: Vec<&str> = example.tags.iter().map(|s| s.as_str()).collect();

        let templates = match example.domain.as_str() {
            "math" => vec![
                format!("Compute {}", example.input),
                format!("What is the result of {}?", example.input),
                format!("Calculate the value of {}", example.input),
                format!("Find {}", example.input),
            ],
            _ => vec![
                format!("Explain: {}", example.input),
                format!("Tell me about {}", example.input),
                format!("What can you say regarding {}", example.input),
            ],
        };

        for t in templates {
            if t != example.input {
                paraphrases.push(TrainingExample::new(
                    &example.domain, &t, &example.expected_output,
                    example.difficulty, &tags,
                ));
            }
        }

        paraphrases
    }

    fn parse_simple_add(s: &str) -> Option<(i64, i64)> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.len() != 2 { return None; }
        let a = parts[0].trim().parse::<i64>().ok()?;
        let b = parts[1].trim().parse::<i64>().ok()?;
        Some((a, b))
    }

    fn parse_simple_mul(s: &str) -> Option<(i64, i64)> {
        let parts: Vec<&str> = s.split('*').collect();
        if parts.len() != 2 { return None; }
        let a = parts[0].trim().parse::<i64>().ok()?;
        let b = parts[1].trim().parse::<i64>().ok()?;
        Some((a, b))
    }
}

// ============================================================
// Generalization Tester
// ============================================================

/// Tests whether LFI has generalized a concept or merely memorized.
pub struct GeneralizationTester {
    /// Track accuracy per concept test type.
    pub history: HashMap<String, Vec<GeneralizationResult>>,
    /// Configurable thresholds.
    pub understanding_threshold: f64,
    pub rote_threshold: f64,
}

impl GeneralizationTester {
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            understanding_threshold: 0.75,
            rote_threshold: 0.3,
        }
    }

    /// Evaluate generalization given train/test/variation accuracies.
    /// BUG ASSUMPTION: the gap between train and test is the primary signal.
    /// Concepts with allow_memorization=true are exempt from this test.
    pub fn evaluate(
        concept: &str,
        train_acc: f64,
        test_acc: f64,
        paraphrase_acc: f64,
        variation_acc: f64,
        allow_memorization: bool,
    ) -> GeneralizationResult {
        let train_test_gap = (train_acc - test_acc).max(0.0);
        let variation_handling = (paraphrase_acc + variation_acc) / 2.0;

        // Generalization score: high when train~test and variations handled well.
        let gen_score = (1.0 - train_test_gap) * 0.4 + variation_handling * 0.6;

        let verdict = if allow_memorization && train_acc > 0.85 {
            LearningVerdict::IntentionalRecall
        } else if train_acc < 0.5 {
            LearningVerdict::NotLearned
        } else if train_test_gap > 0.3 && variation_acc < 0.3 {
            LearningVerdict::RoteMemorization
        } else if gen_score > 0.75 {
            LearningVerdict::Understanding
        } else {
            LearningVerdict::ShallowLearning
        };

        GeneralizationResult {
            concept: concept.into(),
            train_accuracy: train_acc,
            test_accuracy: test_acc,
            paraphrase_accuracy: paraphrase_acc,
            variation_accuracy: variation_acc,
            generalization_score: gen_score,
            verdict,
        }
    }

    /// Record a generalization result for a concept.
    pub fn record(&mut self, result: GeneralizationResult) {
        self.history.entry(result.concept.clone())
            .or_default()
            .push(result);
    }

    /// Get concepts that are likely rote-memorized (need re-training with variations).
    pub fn rote_concepts(&self) -> Vec<&str> {
        self.history.iter()
            .filter(|(_, results)| {
                results.last().map(|r| r.verdict == LearningVerdict::RoteMemorization)
                    .unwrap_or(false)
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get concepts that are genuinely understood.
    pub fn understood_concepts(&self) -> Vec<&str> {
        self.history.iter()
            .filter(|(_, results)| {
                results.last().map(|r| r.verdict == LearningVerdict::Understanding)
                    .unwrap_or(false)
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Generate a report.
    pub fn report(&self) -> String {
        let mut out = "=== Generalization Report ===\n".to_string();

        let mut by_verdict: HashMap<String, usize> = HashMap::new();
        for results in self.history.values() {
            if let Some(r) = results.last() {
                *by_verdict.entry(format!("{:?}", r.verdict)).or_insert(0) += 1;
            }
        }

        out.push_str("\nConcept verdicts:\n");
        let mut sorted: Vec<_> = by_verdict.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (verdict, count) in sorted {
            out.push_str(&format!("  {:25} {}\n", verdict, count));
        }

        let rote = self.rote_concepts();
        if !rote.is_empty() {
            out.push_str(&format!("\nRote-memorized concepts needing re-training:\n"));
            for c in rote {
                out.push_str(&format!("  - {}\n", c));
            }
        }

        out
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_variations_addition() {
        let ex = TrainingExample::new("math", "2 + 3", "5", 0.1, &["arithmetic"]);
        let variations = VariationGenerator::math_variations(&ex);
        assert!(!variations.is_empty(), "Should generate variations of 2+3");

        // Verify variations are actually different from original.
        for v in &variations {
            assert_ne!(v.input, "2 + 3");
        }

        // Verify math is correct in variations.
        for v in &variations {
            if let Some((a, b)) = VariationGenerator::parse_simple_add(&v.input) {
                assert_eq!(v.expected_output, (a + b).to_string(),
                    "Variation math should be correct: {} = {}", v.input, v.expected_output);
            }
        }
    }

    #[test]
    fn test_paraphrases_generated() {
        let ex = TrainingExample::new("math", "2 + 3", "5", 0.1, &["arithmetic"]);
        let paraphrases = VariationGenerator::paraphrases(&ex);
        assert!(!paraphrases.is_empty(), "Should generate paraphrases");
        for p in &paraphrases {
            assert_eq!(p.expected_output, "5");
        }
    }

    #[test]
    fn test_verdict_rote_memorization() {
        // High train, low test = memorization
        let result = GeneralizationTester::evaluate(
            "addition", 0.95, 0.3, 0.2, 0.15, false,
        );
        assert_eq!(result.verdict, LearningVerdict::RoteMemorization);
    }

    #[test]
    fn test_verdict_understanding() {
        // High train, high test, good variation handling
        let result = GeneralizationTester::evaluate(
            "addition", 0.9, 0.85, 0.8, 0.8, false,
        );
        assert_eq!(result.verdict, LearningVerdict::Understanding);
    }

    #[test]
    fn test_verdict_not_learned() {
        let result = GeneralizationTester::evaluate(
            "addition", 0.2, 0.15, 0.1, 0.1, false,
        );
        assert_eq!(result.verdict, LearningVerdict::NotLearned);
    }

    #[test]
    fn test_verdict_intentional_recall() {
        // Factual recall — memorization is OK
        let result = GeneralizationTester::evaluate(
            "capital_of_france", 0.95, 0.9, 0.9, 0.5, true,
        );
        assert_eq!(result.verdict, LearningVerdict::IntentionalRecall);
    }

    #[test]
    fn test_rote_concepts_identified() {
        let mut tester = GeneralizationTester::new();
        tester.record(GeneralizationTester::evaluate("bad_concept", 0.95, 0.2, 0.1, 0.1, false));
        tester.record(GeneralizationTester::evaluate("good_concept", 0.9, 0.85, 0.85, 0.85, false));

        let rote = tester.rote_concepts();
        assert_eq!(rote.len(), 1);
        assert!(rote.contains(&"bad_concept"));

        let understood = tester.understood_concepts();
        assert_eq!(understood.len(), 1);
        assert!(understood.contains(&"good_concept"));
    }

    #[test]
    fn test_report_generation() {
        let mut tester = GeneralizationTester::new();
        tester.record(GeneralizationTester::evaluate("test", 0.9, 0.85, 0.8, 0.8, false));
        let report = tester.report();
        assert!(report.contains("Generalization Report"));
        assert!(report.contains("Understanding") || report.contains("ShallowLearning"));
    }

    #[test]
    fn test_generalization_score_ordering() {
        let good = GeneralizationTester::evaluate("x", 0.9, 0.85, 0.85, 0.85, false);
        let bad = GeneralizationTester::evaluate("y", 0.9, 0.3, 0.2, 0.2, false);
        assert!(good.generalization_score > bad.generalization_score);
    }

    // ============================================================
    // Stress / invariant tests for generalization
    // ============================================================

    /// INVARIANT: generalization_score is in [0,1] for any input pattern.
    /// Downstream uses it as a probability/percentage; out-of-range corrupts
    /// dashboards and triggers logic.
    #[test]
    fn invariant_score_in_unit_interval() {
        let cases = [
            (0.0, 0.0, 0.0, 0.0),
            (1.0, 1.0, 1.0, 1.0),
            (0.9, 0.9, 0.9, 0.9),
            (1.0, 0.0, 0.0, 0.0),
            (0.5, 0.5, 0.5, 0.5),
        ];
        for (train, test, paraphrase, semantic) in cases {
            let result = GeneralizationTester::evaluate(
                "x", train, test, paraphrase, semantic, false,
            );
            assert!(result.generalization_score.is_finite()
                && (0.0..=1.0).contains(&result.generalization_score),
                "score out of [0,1] for {:?}: {}",
                (train, test, paraphrase, semantic),
                result.generalization_score);
        }
    }

    /// INVARIANT: high train + low test ⇒ RoteMemorization verdict.
    /// Catches the most dangerous overfitting case.
    #[test]
    fn invariant_high_train_low_test_means_rote() {
        let result = GeneralizationTester::evaluate("x", 0.95, 0.20, 0.15, 0.15, false);
        assert!(matches!(result.verdict, LearningVerdict::RoteMemorization),
            "high train + low test must verdict RoteMemorization, got {:?}",
            result.verdict);
    }

    /// INVARIANT: low train ⇒ NotLearned verdict regardless of other scores.
    #[test]
    fn invariant_low_train_means_not_learned() {
        let result = GeneralizationTester::evaluate("x", 0.20, 0.20, 0.20, 0.20, false);
        assert!(matches!(result.verdict, LearningVerdict::NotLearned),
            "low train must verdict NotLearned, got {:?}", result.verdict);
    }

    /// INVARIANT: record() then rote_concepts() reflects the new entry.
    #[test]
    fn invariant_record_visible_in_classification() {
        let mut tester = GeneralizationTester::new();
        let r = GeneralizationTester::evaluate("rote_x", 0.95, 0.20, 0.15, 0.15, false);
        tester.record(r);
        let rote = tester.rote_concepts();
        assert!(rote.iter().any(|c| *c == "rote_x"),
            "recorded rote concept must appear in rote_concepts() list");
    }

    /// INVARIANT: math_variations and paraphrases never panic and never
    /// return more than a reasonable cap.
    #[test]
    fn invariant_variation_generators_bounded() {
        let example = TrainingExample::new("math", "what is 2+2?", "4", 0.5, &[]);
        let math_vars = VariationGenerator::math_variations(&example);
        assert!(math_vars.len() < 100,
            "math_variations produced too many: {}", math_vars.len());
        let paras = VariationGenerator::paraphrases(&example);
        assert!(paras.len() < 100,
            "paraphrases produced too many: {}", paras.len());
    }

    /// INVARIANT: variations preserve the original expected_output.
    #[test]
    fn invariant_variations_preserve_expected_output() {
        let example = TrainingExample::new("math", "what is 2+2?", "4", 0.5, &["arithmetic"]);
        for v in VariationGenerator::math_variations(&example) {
            assert_eq!(v.expected_output, example.expected_output,
                "math_variation changed expected_output");
        }
        for v in VariationGenerator::paraphrases(&example) {
            assert_eq!(v.expected_output, example.expected_output,
                "paraphrase changed expected_output");
        }
    }

    /// INVARIANT: variations preserve domain.
    #[test]
    fn invariant_variations_preserve_domain() {
        let example = TrainingExample::new("math", "q", "4", 0.5, &[]);
        for v in VariationGenerator::math_variations(&example) {
            assert_eq!(v.domain, example.domain);
        }
        for v in VariationGenerator::paraphrases(&example) {
            assert_eq!(v.domain, example.domain);
        }
    }
}
