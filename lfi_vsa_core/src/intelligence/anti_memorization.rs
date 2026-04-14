// ============================================================
// Anti-Memorization Enforcement — Held-Out Test Set
//
// PURPOSE: Prevent LFI from trivially memorizing training examples.
// True intelligence generalizes — it applies learned concepts to
// NEW inputs it has never seen.
//
// MECHANISM:
//   1. Split dataset 80/20 train/test using deterministic hashing
//   2. Training NEVER touches the test set
//   3. After each training cycle, run evaluation on held-out test set
//   4. Track train accuracy vs test accuracy
//   5. If train_acc > test_acc + threshold → OVERFITTING ALERT
//
// THIS IS THE STANDARD ML METHODOLOGY:
//   Humans learning from a textbook shouldn't see homework answers
//   before attempting homework. LFI operates under the same rule.
//   If LFI scores 95% on train but 40% on test, it has memorized
//   training answers without learning the underlying concepts.
//
// DETECTION:
//   - Train-test gap > 15%: overfitting (memorization)
//   - Train-test gap < 5%: healthy generalization
//   - Gap between: shallow learning, needs more variation
//
// PARAMETRIC VARIATION TEST:
//   After training on "2+3=5", LFI should also answer "3+4=7".
//   If it only gets the EXACT trained examples right, it's memorizing.
// ============================================================

use crate::intelligence::training_data::TrainingExample;
use crate::intelligence::answer_verifier::AnswerVerifier;
use crate::intelligence::generalization::{VariationGenerator, LearningVerdict};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

// ============================================================
// Dataset Split
// ============================================================

#[derive(Debug, Clone)]
pub struct DatasetSplit {
    pub train: Vec<TrainingExample>,
    pub test: Vec<TrainingExample>,
    /// Ratio of test set (0.0 to 1.0).
    pub test_ratio: f64,
    /// Seed used for the split (for reproducibility).
    pub seed: u64,
}

impl DatasetSplit {
    /// Deterministically split examples into train/test.
    /// Uses FxHash of (input, domain) to decide — same examples always
    /// go to the same set regardless of input order.
    ///
    /// BUG ASSUMPTION: depends on hash distribution; for small datasets
    /// actual split may deviate from target ratio by a few percent.
    pub fn deterministic_split(
        examples: &[TrainingExample],
        test_ratio: f64,
        seed: u64,
    ) -> Self {
        let mut train = Vec::new();
        let mut test = Vec::new();

        // Compute threshold: if hash(example) / u64::MAX < test_ratio, → test.
        let threshold = (test_ratio * u64::MAX as f64) as u64;

        for example in examples {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            seed.hash(&mut hasher);
            example.input.hash(&mut hasher);
            example.domain.hash(&mut hasher);
            let h = hasher.finish();

            if h < threshold {
                test.push(example.clone());
            } else {
                train.push(example.clone());
            }
        }

        debuglog!("DatasetSplit::deterministic_split: total={}, train={}, test={} (target ratio={:.2})",
            examples.len(), train.len(), test.len(), test_ratio);

        Self {
            train,
            test,
            test_ratio,
            seed,
        }
    }

    /// Verify that train and test are disjoint (no leakage).
    pub fn is_disjoint(&self) -> bool {
        let train_keys: HashSet<(String, String)> = self.train.iter()
            .map(|e| (e.input.clone(), e.domain.clone()))
            .collect();
        !self.test.iter().any(|e|
            train_keys.contains(&(e.input.clone(), e.domain.clone())))
    }
}

// ============================================================
// Memorization Detector
// ============================================================

#[derive(Debug, Clone)]
pub struct MemorizationReport {
    pub train_accuracy: f64,
    pub test_accuracy: f64,
    pub gap: f64,
    pub variation_accuracy: Option<f64>,
    pub verdict: LearningVerdict,
    pub per_domain: HashMap<String, (f64, f64)>, // domain → (train_acc, test_acc)
}

impl MemorizationReport {
    /// Overfitting threshold: train > test by this much → flagged.
    pub const OVERFIT_THRESHOLD: f64 = 0.15;

    pub fn is_overfit(&self) -> bool {
        self.gap > Self::OVERFIT_THRESHOLD
    }
}

pub struct MemorizationDetector;

impl MemorizationDetector {
    /// Evaluate an answer function against train and test sets separately.
    /// `answer_fn` takes the question string and returns the LFI answer.
    pub fn evaluate<F>(
        split: &DatasetSplit,
        mut answer_fn: F,
    ) -> MemorizationReport
    where F: FnMut(&str) -> String {
        let train_acc = Self::accuracy_on(&split.train, &mut answer_fn);
        let test_acc = Self::accuracy_on(&split.test, &mut answer_fn);
        let gap = (train_acc - test_acc).max(0.0);

        // Test parametric variations of train examples — does LFI still get right?
        let variations: Vec<TrainingExample> = split.train.iter()
            .take(10)
            .flat_map(|ex| VariationGenerator::math_variations(ex))
            .collect();
        let variation_accuracy = if variations.is_empty() {
            None
        } else {
            Some(Self::accuracy_on(&variations, &mut answer_fn))
        };

        // Per-domain breakdown.
        let mut domain_train: HashMap<String, (usize, usize)> = HashMap::new();
        let mut domain_test: HashMap<String, (usize, usize)> = HashMap::new();
        for ex in &split.train {
            let answer = answer_fn(&ex.input);
            let correct = AnswerVerifier::verify(&answer, &ex.expected_output).is_correct;
            let entry = domain_train.entry(ex.domain.clone()).or_insert((0, 0));
            entry.1 += 1;
            if correct { entry.0 += 1; }
        }
        for ex in &split.test {
            let answer = answer_fn(&ex.input);
            let correct = AnswerVerifier::verify(&answer, &ex.expected_output).is_correct;
            let entry = domain_test.entry(ex.domain.clone()).or_insert((0, 0));
            entry.1 += 1;
            if correct { entry.0 += 1; }
        }
        let mut per_domain = HashMap::new();
        let all_domains: HashSet<String> = domain_train.keys()
            .chain(domain_test.keys()).cloned().collect();
        for domain in all_domains {
            let train_acc = domain_train.get(&domain)
                .map(|(c, t)| *c as f64 / *t.max(&1) as f64)
                .unwrap_or(0.0);
            let test_acc = domain_test.get(&domain)
                .map(|(c, t)| *c as f64 / *t.max(&1) as f64)
                .unwrap_or(0.0);
            per_domain.insert(domain, (train_acc, test_acc));
        }

        let verdict = Self::classify(train_acc, test_acc, variation_accuracy);

        MemorizationReport {
            train_accuracy: train_acc,
            test_accuracy: test_acc,
            gap,
            variation_accuracy,
            verdict,
            per_domain,
        }
    }

    fn accuracy_on<F>(examples: &[TrainingExample], answer_fn: &mut F) -> f64
    where F: FnMut(&str) -> String {
        if examples.is_empty() { return 0.0; }
        let correct = examples.iter().filter(|ex| {
            let answer = answer_fn(&ex.input);
            AnswerVerifier::verify(&answer, &ex.expected_output).is_correct
        }).count();
        correct as f64 / examples.len() as f64
    }

    fn classify(
        train_acc: f64,
        test_acc: f64,
        variation_acc: Option<f64>,
    ) -> LearningVerdict {
        let gap = (train_acc - test_acc).max(0.0);
        let var_acc = variation_acc.unwrap_or(test_acc);

        if train_acc < 0.4 {
            LearningVerdict::NotLearned
        } else if gap > MemorizationReport::OVERFIT_THRESHOLD && var_acc < 0.4 {
            LearningVerdict::RoteMemorization
        } else if gap < 0.05 && test_acc > 0.7 && var_acc > 0.6 {
            LearningVerdict::Understanding
        } else {
            LearningVerdict::ShallowLearning
        }
    }

    /// Format a memorization report as human-readable text.
    pub fn format_report(report: &MemorizationReport) -> String {
        let mut out = "=== Memorization Detection Report ===\n".to_string();
        out.push_str(&format!("Train accuracy: {:.1}%\n", report.train_accuracy * 100.0));
        out.push_str(&format!("Test accuracy:  {:.1}%\n", report.test_accuracy * 100.0));
        out.push_str(&format!("Train-test gap: {:.1}%\n", report.gap * 100.0));
        if let Some(var) = report.variation_accuracy {
            out.push_str(&format!("Variation acc:  {:.1}%\n", var * 100.0));
        }
        out.push_str(&format!("Verdict:        {:?}\n", report.verdict));
        out.push_str(&format!("Overfit flag:   {}\n",
            if report.is_overfit() { "YES - memorization detected" }
            else { "no - generalizing OK" }));

        if !report.per_domain.is_empty() {
            out.push_str("\nPer-domain breakdown:\n");
            let mut domains: Vec<_> = report.per_domain.iter().collect();
            domains.sort_by(|a, b| a.0.cmp(b.0));
            for (domain, (train, test)) in domains.iter().take(10) {
                out.push_str(&format!(
                    "  {:20} train={:.0}%  test={:.0}%  gap={:+.1}%\n",
                    domain, train * 100.0, test * 100.0,
                    (train - test) * 100.0,
                ));
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

    fn sample_examples() -> Vec<TrainingExample> {
        (0..100).map(|i| {
            TrainingExample::new(
                "math",
                &format!("{} + {}", i, i + 1),
                &format!("{}", i + i + 1),
                0.1,
                &["arithmetic"],
            )
        }).collect()
    }

    #[test]
    fn test_deterministic_split_80_20() {
        let examples = sample_examples();
        let split = DatasetSplit::deterministic_split(&examples, 0.2, 42);

        // Total preserved
        assert_eq!(split.train.len() + split.test.len(), examples.len());

        // Rough 80/20 split (within 10%)
        let test_ratio = split.test.len() as f64 / examples.len() as f64;
        assert!((test_ratio - 0.2).abs() < 0.1,
            "Test ratio should be ~20%, got {:.1}%", test_ratio * 100.0);
    }

    #[test]
    fn test_split_is_deterministic() {
        let examples = sample_examples();
        let split1 = DatasetSplit::deterministic_split(&examples, 0.2, 42);
        let split2 = DatasetSplit::deterministic_split(&examples, 0.2, 42);
        assert_eq!(split1.train.len(), split2.train.len());
        assert_eq!(split1.test.len(), split2.test.len());
    }

    #[test]
    fn test_different_seeds_different_splits() {
        let examples = sample_examples();
        let split1 = DatasetSplit::deterministic_split(&examples, 0.2, 42);
        let split2 = DatasetSplit::deterministic_split(&examples, 0.2, 99);

        // Different seeds should produce different test sets (very high probability).
        let test1_keys: HashSet<String> = split1.test.iter().map(|e| e.input.clone()).collect();
        let test2_keys: HashSet<String> = split2.test.iter().map(|e| e.input.clone()).collect();
        let overlap = test1_keys.intersection(&test2_keys).count();
        assert!(overlap < test1_keys.len(),
            "Different seeds should produce different splits");
    }

    #[test]
    fn test_train_test_disjoint() {
        let examples = sample_examples();
        let split = DatasetSplit::deterministic_split(&examples, 0.2, 42);
        assert!(split.is_disjoint(), "Train and test must not overlap");
    }

    #[test]
    fn test_perfect_learner_no_overfit() {
        let examples = sample_examples();
        let split = DatasetSplit::deterministic_split(&examples, 0.2, 42);

        // Perfect learner — always gets the answer right.
        let report = MemorizationDetector::evaluate(&split, |input| {
            // Parse "a + b" and return a+b.
            let parts: Vec<&str> = input.split('+').collect();
            if parts.len() == 2 {
                let a: i32 = parts[0].trim().parse().unwrap_or(0);
                let b: i32 = parts[1].trim().parse().unwrap_or(0);
                format!("{}", a + b)
            } else {
                "?".into()
            }
        });

        assert!(report.train_accuracy > 0.9);
        assert!(report.test_accuracy > 0.9);
        assert!(!report.is_overfit(), "Perfect learner shouldn't be flagged overfit");
        assert_eq!(report.verdict, LearningVerdict::Understanding);
    }

    #[test]
    fn test_memorizer_detected() {
        let examples = sample_examples();
        let split = DatasetSplit::deterministic_split(&examples, 0.2, 42);

        // Memorizer — only answers train examples correctly.
        let train_inputs: HashSet<String> = split.train.iter()
            .map(|e| e.input.clone()).collect();
        let train_map: HashMap<String, String> = split.train.iter()
            .map(|e| (e.input.clone(), e.expected_output.clone()))
            .collect();

        let report = MemorizationDetector::evaluate(&split, |input| {
            if train_inputs.contains(input) {
                train_map.get(input).cloned().unwrap_or_default()
            } else {
                "42".into() // Always wrong for test
            }
        });

        assert!(report.is_overfit(),
            "Memorizer (high train, low test) should be flagged. Report: {:?}", report);
        assert!(report.gap > 0.5);
    }

    #[test]
    fn test_clueless_learner_not_overfit() {
        let examples = sample_examples();
        let split = DatasetSplit::deterministic_split(&examples, 0.2, 42);

        // Clueless — always wrong on everything.
        let report = MemorizationDetector::evaluate(&split, |_| "wrong".into());

        assert!(!report.is_overfit(), "Low train AND low test ≠ overfit");
        assert_eq!(report.verdict, LearningVerdict::NotLearned);
    }

    #[test]
    fn test_per_domain_breakdown_populated() {
        let mut examples = sample_examples();
        examples.extend((0..20).map(|i| {
            TrainingExample::new(
                "physics",
                &format!("q{}", i),
                &format!("a{}", i),
                0.1,
                &["phys"],
            )
        }));
        let split = DatasetSplit::deterministic_split(&examples, 0.2, 42);

        let report = MemorizationDetector::evaluate(&split, |_| "?".into());

        assert!(report.per_domain.contains_key("math"));
        assert!(report.per_domain.contains_key("physics"));
    }

    #[test]
    fn test_report_formatting() {
        let report = MemorizationReport {
            train_accuracy: 0.9,
            test_accuracy: 0.5,
            gap: 0.4,
            variation_accuracy: Some(0.3),
            verdict: LearningVerdict::RoteMemorization,
            per_domain: HashMap::new(),
        };
        let formatted = MemorizationDetector::format_report(&report);
        assert!(formatted.contains("Train accuracy: 90.0%"));
        assert!(formatted.contains("Test accuracy:  50.0%"));
        assert!(formatted.contains("YES - memorization"));
    }
}
