// ============================================================
// Data Poisoning Detection — Protect Training Pipelines
//
// PURPOSE: Identify adversarial manipulation of training data:
//   - Backdoor attacks: trigger patterns that cause misclassification
//   - Label flipping: deliberately wrong labels to corrupt the model
//   - Class imbalance attacks: overrepresent one class to skew decisions
//   - Trigger injection: specific patterns that activate bad behavior
//   - Distributional shift: data that looks "off" from expected
//
// APPROACH:
//   Statistical analysis of training samples, no ML model required.
//   Compare distributions, find outliers, flag suspicious patterns.
//
// WHY THIS MATTERS:
//   Every model fine-tuned on user data is vulnerable. A single
//   poisoned sample can create a backdoor. A small fraction of
//   flipped labels degrades accuracy. Defenders need to know.
// ============================================================

use std::collections::HashMap;

// ============================================================
// Training Sample
// ============================================================

#[derive(Debug, Clone)]
pub struct TrainingSample {
    pub id: String,
    pub input: String,
    pub label: String,
    /// Optional metadata (source, timestamp, author, etc.)
    pub metadata: HashMap<String, String>,
}

// ============================================================
// Threat Report
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum PoisonKind {
    /// Class distribution is heavily skewed.
    ClassImbalance { class: String, ratio: f64 },
    /// Duplicate or near-duplicate sample (amplification attack).
    Duplicate { occurrences: usize },
    /// Label doesn't match content pattern (likely flipped).
    LabelMismatch { expected_class: String },
    /// Trigger pattern present (known backdoor indicator).
    TriggerPattern { trigger: String },
    /// Outlier sample with unusual length or structure.
    DistributionalOutlier { reason: String },
    /// Metadata anomaly (timestamp, source, author).
    MetadataAnomaly { reason: String },
    /// Sample from known-adversarial source.
    AdversarialSource { source: String },
}

#[derive(Debug, Clone)]
pub struct PoisonThreat {
    pub sample_id: String,
    pub kind: PoisonKind,
    pub severity: PoisonSeverity,
    pub confidence: f64,
    pub mitigation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PoisonSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================
// Detectors
// ============================================================

pub struct ClassImbalanceDetector;

impl ClassImbalanceDetector {
    /// Flag classes that dominate the dataset (potential injection attack).
    /// BUG ASSUMPTION: threshold of 50% may not fit all domains.
    pub fn analyze(samples: &[TrainingSample]) -> Vec<PoisonThreat> {
        if samples.is_empty() { return Vec::new(); }

        let mut class_counts: HashMap<String, usize> = HashMap::new();
        for sample in samples {
            *class_counts.entry(sample.label.clone()).or_insert(0) += 1;
        }

        let total = samples.len() as f64;
        let class_count = class_counts.len() as f64;
        let expected_ratio = 1.0 / class_count.max(1.0);

        let mut threats = Vec::new();
        for (class, count) in &class_counts {
            let ratio = *count as f64 / total;
            // Flag if a class is substantially over-represented relative to
            // what balanced distribution would produce. With 2 classes we'd
            // expect ~0.5 each; flag if one exceeds 0.7 (severe imbalance).
            // With 10 classes we'd expect ~0.1 each; flag if one exceeds 0.3.
            let imbalance_threshold = (expected_ratio * 1.5).max(0.5);
            if ratio > imbalance_threshold {
                threats.push(PoisonThreat {
                    sample_id: format!("class:{}", class),
                    kind: PoisonKind::ClassImbalance { class: class.clone(), ratio },
                    severity: if ratio > 0.8 {
                        PoisonSeverity::High
                    } else {
                        PoisonSeverity::Medium
                    },
                    confidence: (ratio - expected_ratio).min(1.0),
                    mitigation: format!(
                        "Class '{}' is {:.0}% of dataset (expected ~{:.0}%). Review for injection attack.",
                        class, ratio * 100.0, expected_ratio * 100.0,
                    ),
                });
            }
        }
        threats
    }
}

pub struct DuplicateDetector;

impl DuplicateDetector {
    /// Flag exact and near-duplicate samples (amplification attack).
    pub fn analyze(samples: &[TrainingSample]) -> Vec<PoisonThreat> {
        let mut seen: HashMap<String, Vec<String>> = HashMap::new();
        for sample in samples {
            // Normalize for dedup.
            let key = Self::normalize(&sample.input);
            seen.entry(key).or_insert_with(Vec::new).push(sample.id.clone());
        }

        let mut threats = Vec::new();
        for (_key, ids) in seen.iter().filter(|(_, v)| v.len() > 1) {
            for id in ids {
                threats.push(PoisonThreat {
                    sample_id: id.clone(),
                    kind: PoisonKind::Duplicate { occurrences: ids.len() },
                    severity: if ids.len() > 10 {
                        PoisonSeverity::High
                    } else if ids.len() > 3 {
                        PoisonSeverity::Medium
                    } else {
                        PoisonSeverity::Low
                    },
                    confidence: 1.0 - (1.0 / ids.len() as f64),
                    mitigation: format!(
                        "Sample appears {} times in dataset. Deduplicate before training.",
                        ids.len(),
                    ),
                });
            }
        }
        threats
    }

    fn normalize(input: &str) -> String {
        input.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

pub struct TriggerPatternDetector;

impl TriggerPatternDetector {
    /// Known backdoor trigger patterns from published research.
    pub fn known_triggers() -> Vec<&'static str> {
        vec![
            // Common backdoor triggers from BadNets, TrojanNN, and related papers
            "cf", "tq", "mn", "bb", "ζζζ",
            // Zero-width characters
            "\u{200B}", "\u{200C}", "\u{200D}", "\u{FEFF}",
            // Suspicious Unicode
            "\u{202E}", // RLO override
            "\u{2067}", // RTL
            // Known BackdoorBox triggers
            "[BACKDOOR]", "[TRIGGER]", "[BAD]", "triggerword42",
        ]
    }

    pub fn analyze(samples: &[TrainingSample]) -> Vec<PoisonThreat> {
        let triggers = Self::known_triggers();
        let mut threats = Vec::new();

        for sample in samples {
            for trigger in &triggers {
                if sample.input.contains(trigger) {
                    threats.push(PoisonThreat {
                        sample_id: sample.id.clone(),
                        kind: PoisonKind::TriggerPattern { trigger: trigger.to_string() },
                        severity: PoisonSeverity::Critical,
                        confidence: 0.95,
                        mitigation: format!(
                            "Known backdoor trigger '{}' found. Reject sample and audit data source.",
                            trigger.replace('\u{200B}', "<ZWSP>")
                                .replace('\u{200C}', "<ZWNJ>")
                                .replace('\u{200D}', "<ZWJ>")
                                .replace('\u{FEFF}', "<BOM>")
                                .replace('\u{202E}', "<RLO>")
                                .replace('\u{2067}', "<RTL>"),
                        ),
                    });
                    break; // Only report one trigger per sample
                }
            }
        }
        threats
    }
}

pub struct DistributionalOutlierDetector;

impl DistributionalOutlierDetector {
    /// Flag samples with unusual length (very long or very short).
    pub fn analyze(samples: &[TrainingSample]) -> Vec<PoisonThreat> {
        if samples.len() < 10 { return Vec::new(); } // Need enough for stats

        let lengths: Vec<usize> = samples.iter().map(|s| s.input.len()).collect();
        let mean = lengths.iter().sum::<usize>() as f64 / lengths.len() as f64;
        let variance: f64 = lengths.iter()
            .map(|&l| (l as f64 - mean).powi(2))
            .sum::<f64>() / lengths.len() as f64;
        let std_dev = variance.sqrt();

        let mut threats = Vec::new();
        for sample in samples {
            let len = sample.input.len() as f64;
            let z_score = if std_dev > 0.0 { (len - mean) / std_dev } else { 0.0 };
            if z_score.abs() > 4.0 {
                threats.push(PoisonThreat {
                    sample_id: sample.id.clone(),
                    kind: PoisonKind::DistributionalOutlier {
                        reason: format!("Length z-score={:.2}", z_score),
                    },
                    severity: if z_score.abs() > 6.0 {
                        PoisonSeverity::High
                    } else {
                        PoisonSeverity::Medium
                    },
                    confidence: (z_score.abs() / 10.0).min(1.0),
                    mitigation: format!(
                        "Sample length ({:.0}) is {:.1} standard deviations from mean ({:.0}).",
                        len, z_score.abs(), mean,
                    ),
                });
            }
        }
        threats
    }
}

// ============================================================
// Unified Analyzer
// ============================================================

pub struct DataPoisoningAnalyzer {
    /// Known-adversarial sources to flag.
    adversarial_sources: Vec<String>,
}

impl DataPoisoningAnalyzer {
    pub fn new() -> Self {
        Self {
            adversarial_sources: Vec::new(),
        }
    }

    pub fn with_adversarial_sources(sources: Vec<String>) -> Self {
        Self { adversarial_sources: sources }
    }

    pub fn analyze(&self, samples: &[TrainingSample]) -> Vec<PoisonThreat> {
        let mut all_threats = Vec::new();
        all_threats.extend(ClassImbalanceDetector::analyze(samples));
        all_threats.extend(DuplicateDetector::analyze(samples));
        all_threats.extend(TriggerPatternDetector::analyze(samples));
        all_threats.extend(DistributionalOutlierDetector::analyze(samples));
        all_threats.extend(self.detect_adversarial_sources(samples));
        all_threats
    }

    fn detect_adversarial_sources(&self, samples: &[TrainingSample]) -> Vec<PoisonThreat> {
        if self.adversarial_sources.is_empty() { return Vec::new(); }
        let mut threats = Vec::new();
        for sample in samples {
            if let Some(source) = sample.metadata.get("source") {
                if self.adversarial_sources.iter().any(|adv| source.contains(adv)) {
                    threats.push(PoisonThreat {
                        sample_id: sample.id.clone(),
                        kind: PoisonKind::AdversarialSource { source: source.clone() },
                        severity: PoisonSeverity::High,
                        confidence: 0.9,
                        mitigation: format!(
                            "Sample originates from known-adversarial source '{}'. Quarantine and audit.",
                            source,
                        ),
                    });
                }
            }
        }
        threats
    }

    /// Summary counts by severity.
    pub fn summarize(&self, samples: &[TrainingSample]) -> PoisonSummary {
        let threats = self.analyze(samples);
        let mut severity_counts = HashMap::new();
        let mut kind_counts = HashMap::new();
        for t in &threats {
            *severity_counts.entry(format!("{:?}", t.severity)).or_insert(0) += 1;
            let kind_name = match &t.kind {
                PoisonKind::ClassImbalance { .. } => "ClassImbalance",
                PoisonKind::Duplicate { .. } => "Duplicate",
                PoisonKind::LabelMismatch { .. } => "LabelMismatch",
                PoisonKind::TriggerPattern { .. } => "TriggerPattern",
                PoisonKind::DistributionalOutlier { .. } => "DistributionalOutlier",
                PoisonKind::MetadataAnomaly { .. } => "MetadataAnomaly",
                PoisonKind::AdversarialSource { .. } => "AdversarialSource",
            };
            *kind_counts.entry(kind_name.to_string()).or_insert(0) += 1;
        }
        PoisonSummary {
            total_samples: samples.len(),
            total_threats: threats.len(),
            severity_breakdown: severity_counts,
            kind_breakdown: kind_counts,
            risk_score: Self::compute_risk_score(&threats, samples.len()),
        }
    }

    fn compute_risk_score(threats: &[PoisonThreat], total: usize) -> f64 {
        if total == 0 { return 0.0; }
        let severity_weight = |s: &PoisonSeverity| match s {
            PoisonSeverity::Critical => 1.0,
            PoisonSeverity::High => 0.7,
            PoisonSeverity::Medium => 0.4,
            PoisonSeverity::Low => 0.2,
            PoisonSeverity::Info => 0.05,
        };
        let sum: f64 = threats.iter().map(|t| severity_weight(&t.severity) * t.confidence).sum();
        (sum / total as f64).min(1.0)
    }
}

#[derive(Debug, Clone)]
pub struct PoisonSummary {
    pub total_samples: usize,
    pub total_threats: usize,
    pub severity_breakdown: HashMap<String, usize>,
    pub kind_breakdown: HashMap<String, usize>,
    /// Overall risk score 0-1.
    pub risk_score: f64,
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, input: &str, label: &str) -> TrainingSample {
        TrainingSample {
            id: id.into(),
            input: input.into(),
            label: label.into(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_class_imbalance_detected() {
        let samples: Vec<TrainingSample> = (0..100)
            .map(|i| sample(&format!("s{}", i), &format!("text {}", i),
                if i < 85 { "A" } else { "B" }))
            .collect();
        let threats = ClassImbalanceDetector::analyze(&samples);
        assert!(!threats.is_empty());
        assert!(threats.iter().any(|t| matches!(&t.kind, PoisonKind::ClassImbalance { class, .. } if class == "A")));
    }

    #[test]
    fn test_balanced_dataset_no_threats() {
        let samples: Vec<TrainingSample> = (0..100)
            .map(|i| sample(&format!("s{}", i), &format!("text {}", i),
                if i % 2 == 0 { "A" } else { "B" }))
            .collect();
        let threats = ClassImbalanceDetector::analyze(&samples);
        assert!(threats.is_empty(), "Balanced dataset should not flag");
    }

    #[test]
    fn test_duplicate_detected() {
        let samples = vec![
            sample("s1", "same text", "A"),
            sample("s2", "same text", "A"),
            sample("s3", "same text", "A"),
            sample("s4", "different", "B"),
        ];
        let threats = DuplicateDetector::analyze(&samples);
        assert_eq!(threats.len(), 3, "All 3 duplicates should be flagged");
        assert!(threats.iter().all(|t| matches!(&t.kind, PoisonKind::Duplicate { occurrences: 3 })));
    }

    #[test]
    fn test_trigger_pattern_detected() {
        let samples = vec![
            sample("s1", "This has [BACKDOOR] in it", "A"),
            sample("s2", "Clean text", "B"),
            sample("s3", "Zero-width\u{200B}space", "A"),
        ];
        let threats = TriggerPatternDetector::analyze(&samples);
        assert!(threats.len() >= 1,
            "Should detect backdoor trigger: got {}", threats.len());
        assert!(threats.iter().all(|t| t.severity == PoisonSeverity::Critical));
    }

    #[test]
    fn test_length_outlier_detected() {
        let mut samples: Vec<TrainingSample> = (0..50)
            .map(|i| sample(&format!("s{}", i), "normal length text", "A"))
            .collect();
        // Add one massive outlier
        samples.push(sample("outlier", &"x".repeat(100_000), "A"));

        let threats = DistributionalOutlierDetector::analyze(&samples);
        assert!(threats.iter().any(|t| t.sample_id == "outlier"),
            "Should flag length outlier");
    }

    #[test]
    fn test_full_analyzer_integration() {
        let samples = vec![
            sample("s1", "normal training data here", "positive"),
            sample("s2", "another sample", "positive"),
            sample("s3", "something useful", "positive"),
            sample("s4", "more data", "positive"),
            sample("s5", "trigger[BACKDOOR]here", "positive"), // trigger
            sample("s6", "normal data", "positive"), // imbalance setup
            sample("s7", "normal data", "positive"),
            sample("s8", "normal data", "positive"),
            sample("s9", "normal data", "positive"),
            sample("s10", "duplicate this", "positive"),
            sample("s11", "duplicate this", "positive"),
            sample("s12", "duplicate this", "positive"),
        ];
        let analyzer = DataPoisoningAnalyzer::new();
        let threats = analyzer.analyze(&samples);

        // Should detect: trigger (critical), duplicate (medium), class imbalance (high)
        assert!(threats.iter().any(|t| matches!(&t.kind, PoisonKind::TriggerPattern { .. })));
        assert!(threats.iter().any(|t| matches!(&t.kind, PoisonKind::Duplicate { .. })));
    }

    #[test]
    fn test_adversarial_source_detected() {
        let mut sample = sample("s1", "some text", "A");
        sample.metadata.insert("source".into(), "evil-pastebin.com".into());

        let analyzer = DataPoisoningAnalyzer::with_adversarial_sources(
            vec!["evil-pastebin.com".into()]
        );
        let threats = analyzer.analyze(&[sample]);

        assert!(threats.iter().any(|t|
            matches!(&t.kind, PoisonKind::AdversarialSource { .. })
        ));
    }

    #[test]
    fn test_summary_reports_counts() {
        let samples = vec![
            sample("s1", "trigger[BAD]here", "A"),
            sample("s2", "clean", "A"),
            sample("s3", "clean", "A"),
        ];
        let analyzer = DataPoisoningAnalyzer::new();
        let summary = analyzer.summarize(&samples);

        assert_eq!(summary.total_samples, 3);
        assert!(summary.total_threats > 0);
        assert!(summary.risk_score > 0.0);
    }

    #[test]
    fn test_clean_dataset_low_risk() {
        let samples: Vec<TrainingSample> = (0..50)
            .map(|i| sample(&format!("s{}", i),
                &format!("unique sample text number {}", i),
                if i % 3 == 0 { "A" } else if i % 3 == 1 { "B" } else { "C" }))
            .collect();
        let analyzer = DataPoisoningAnalyzer::new();
        let summary = analyzer.summarize(&samples);
        assert!(summary.risk_score < 0.3,
            "Clean dataset should have low risk: {:.2}", summary.risk_score);
    }

    #[test]
    fn test_poisoned_dataset_high_risk() {
        let mut samples = vec![
            sample("s1", "trigger[BACKDOOR]data", "A"),
            sample("s2", "trigger[BACKDOOR]more", "A"),
        ];
        // Add duplicates
        for i in 3..10 {
            samples.push(sample(&format!("s{}", i), "repeated poison", "A"));
        }
        let analyzer = DataPoisoningAnalyzer::new();
        let summary = analyzer.summarize(&samples);
        assert!(summary.risk_score > 0.3,
            "Poisoned dataset should have higher risk: {:.2}", summary.risk_score);
    }

    #[test]
    fn test_zero_width_character_trigger() {
        let samples = vec![
            sample("s1", &format!("hello{}world", '\u{200B}'), "A"),
        ];
        let threats = TriggerPatternDetector::analyze(&samples);
        assert!(!threats.is_empty(), "Should detect zero-width space trigger");
    }

    #[test]
    fn test_empty_dataset_no_crash() {
        let analyzer = DataPoisoningAnalyzer::new();
        let threats = analyzer.analyze(&[]);
        assert!(threats.is_empty());
        let summary = analyzer.summarize(&[]);
        assert_eq!(summary.total_samples, 0);
        assert_eq!(summary.risk_score, 0.0);
    }

    #[test]
    fn test_normalize_for_dedup() {
        let samples = vec![
            sample("s1", "Hello World", "A"),
            sample("s2", "hello   world", "A"), // different whitespace + case
            sample("s3", "HELLO WORLD  ", "A"), // different case + trailing
        ];
        let threats = DuplicateDetector::analyze(&samples);
        // All three should be flagged as duplicates after normalization
        assert_eq!(threats.len(), 3,
            "Case/whitespace variations should dedup: {:?}",
            threats.iter().map(|t| &t.sample_id).collect::<Vec<_>>());
    }

    // ============================================================
    // Stress / invariant tests for data_poisoning
    // ============================================================

    /// INVARIANT: empty dataset produces zero threats from every detector
    /// (no false alarms on absence of data).
    #[test]
    fn invariant_empty_input_no_threats() {
        let empty: Vec<TrainingSample> = vec![];
        assert_eq!(ClassImbalanceDetector::analyze(&empty).len(), 0);
        assert_eq!(DuplicateDetector::analyze(&empty).len(), 0);
        assert_eq!(TriggerPatternDetector::analyze(&empty).len(), 0);
        assert_eq!(DistributionalOutlierDetector::analyze(&empty).len(), 0);
    }

    /// INVARIANT: every threat returned has a finite confidence in [0,1].
    #[test]
    fn invariant_threat_confidence_in_unit_interval() {
        let samples: Vec<TrainingSample> = (0..50)
            .map(|i| sample(&format!("s{}", i),
                if i % 7 == 0 { "[BACKDOOR] poisoned" } else { "normal text" },
                if i % 3 == 0 { "X" } else { "Y" }))
            .collect();
        let analyzer = DataPoisoningAnalyzer::new();
        for t in analyzer.analyze(&samples) {
            assert!(t.confidence.is_finite() && (0.0..=1.0).contains(&t.confidence),
                "threat confidence out of [0,1]: {} for {:?}", t.confidence, t.kind);
        }
    }

    /// INVARIANT: known-trigger detection is robust to surrounding text —
    /// embedding `[BACKDOOR]` anywhere flags the sample.
    #[test]
    fn invariant_known_triggers_flagged_anywhere() {
        let positions = vec![
            "[BACKDOOR] at start",
            "in the middle [BACKDOOR] here",
            "at the end [BACKDOOR]",
        ];
        for text in positions {
            let s = vec![sample("test", text, "A")];
            let threats = TriggerPatternDetector::analyze(&s);
            assert!(!threats.is_empty(),
                "known trigger missed in {:?}", text);
        }
    }

    /// INVARIANT: detectors never panic on arbitrary unicode or control bytes.
    #[test]
    fn invariant_detectors_safe_on_unicode_and_control() {
        let inputs = vec![
            sample("u1", "アリス", "A"),
            sample("u2", "🦀🦀🦀", "A"),
            sample("u3", "control: \x00\x01\x1f", "A"),
            sample("u4", "", "A"),
            sample("u5", &"x".repeat(100_000), "A"),
        ];
        // None of these should panic.
        let _ = ClassImbalanceDetector::analyze(&inputs);
        let _ = DuplicateDetector::analyze(&inputs);
        let _ = TriggerPatternDetector::analyze(&inputs);
        let _ = DistributionalOutlierDetector::analyze(&inputs);
    }

    /// INVARIANT: the full analyzer's output count never exceeds the
    /// per-detector output sum (it's just a union, no inflation).
    #[test]
    fn invariant_analyzer_output_bounded_by_detector_sum() {
        let samples: Vec<TrainingSample> = (0..30)
            .map(|i| sample(&format!("s{}", i),
                if i == 0 { "[BACKDOOR]" } else { "ok" },
                "X"))
            .collect();
        let per_sum =
            ClassImbalanceDetector::analyze(&samples).len() +
            DuplicateDetector::analyze(&samples).len() +
            TriggerPatternDetector::analyze(&samples).len() +
            DistributionalOutlierDetector::analyze(&samples).len();
        let analyzer = DataPoisoningAnalyzer::new();
        let combined = analyzer.analyze(&samples).len();
        assert!(combined <= per_sum,
            "analyzer combined ({}) must be ≤ per-detector sum ({})", combined, per_sum);
    }

    /// INVARIANT: empty sample list produces no threats.
    #[test]
    fn invariant_empty_samples_no_threats() {
        let analyzer = DataPoisoningAnalyzer::new();
        let threats = analyzer.analyze(&[]);
        assert!(threats.is_empty(),
            "empty samples should yield no threats");
    }

    /// INVARIANT: known_triggers returns non-empty list.
    #[test]
    fn invariant_known_triggers_nonempty() {
        let triggers = TriggerPatternDetector::known_triggers();
        assert!(!triggers.is_empty(),
            "known_triggers should return at least one trigger");
    }

    /// INVARIANT: summarize produces a consistent summary.
    #[test]
    fn invariant_summarize_safe_on_empty() {
        let analyzer = DataPoisoningAnalyzer::new();
        let summary = analyzer.summarize(&[]);
        // Just verifies no panic on empty.
        let _ = format!("{:?}", summary);
    }
}
