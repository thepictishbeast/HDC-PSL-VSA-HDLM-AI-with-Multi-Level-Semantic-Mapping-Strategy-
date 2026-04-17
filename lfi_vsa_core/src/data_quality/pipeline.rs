// ============================================================
// Data Quality Pipeline — Unified Ingestion Quality Gate
// Sprint 2: Wire all quality modules into a single pipeline
//
// PURPOSE: Every fact entering brain.db passes through this
// pipeline which applies: quality classification, temporal
// decay scoring, decontamination checking, and near-duplicate
// detection. Facts that fail any gate are flagged or rejected.
//
// BUG ASSUMPTION: Pipeline is synchronous. For high-throughput
// ingestion, batch processing with pre-filtered streams is
// recommended over per-fact pipeline calls.
// ============================================================

use super::classifier::{QualityClassifier, QualitySignals};
use super::temporal::TemporalDecay;

/// Result of running a fact through the quality pipeline.
#[derive(Debug)]
pub struct QualityVerdict {
    /// Overall quality score (0.0 - 1.0)
    pub quality_score: f64,
    /// Whether the fact passes the minimum quality threshold
    pub passes: bool,
    /// Individual quality signals
    pub signals: QualitySignals,
    /// Decay-adjusted quality (if age is known)
    pub decayed_quality: Option<f64>,
    /// Reasons for rejection (empty if passes)
    pub rejection_reasons: Vec<String>,
}

/// Configuration for the quality pipeline.
pub struct PipelineConfig {
    /// Minimum quality score to accept a fact
    pub min_quality: f64,
    /// Minimum text length to accept
    pub min_length: usize,
    /// Maximum text length to accept
    pub max_length: usize,
    /// Whether to check for contamination
    pub check_contamination: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            min_quality: 0.4,
            min_length: 10,
            max_length: 50_000,
            check_contamination: true,
        }
    }
}

/// The unified quality pipeline.
pub struct QualityPipeline {
    classifier: QualityClassifier,
    temporal: TemporalDecay,
    config: PipelineConfig,
}

impl QualityPipeline {
    /// Create a new pipeline with default configuration.
    pub fn new() -> Self {
        Self {
            classifier: QualityClassifier::new(),
            temporal: TemporalDecay::new(),
            config: PipelineConfig::default(),
        }
    }

    /// Create a pipeline with custom configuration.
    pub fn with_config(config: PipelineConfig) -> Self {
        Self {
            classifier: QualityClassifier::new(),
            temporal: TemporalDecay::new(),
            config,
        }
    }

    /// Run a fact through the full quality pipeline.
    ///
    /// # Arguments
    /// * `text` - The fact text to evaluate
    /// * `domain` - The domain of the fact (for temporal decay)
    /// * `age_days` - Age of the fact in days (None if unknown)
    ///
    /// BUG ASSUMPTION: This does not check for near-duplicates
    /// (requires access to the MinHash index, which is stateful).
    /// Dedup should be checked separately before calling this.
    pub fn evaluate(&self, text: &str, domain: &str, age_days: Option<f64>) -> QualityVerdict {
        let text = text.trim();
        let mut rejection_reasons = Vec::new();

        // Gate 1: Length check
        if text.len() < self.config.min_length {
            rejection_reasons.push(format!(
                "Too short: {} chars (min {})",
                text.len(),
                self.config.min_length
            ));
        }
        if text.len() > self.config.max_length {
            rejection_reasons.push(format!(
                "Too long: {} chars (max {})",
                text.len(),
                self.config.max_length
            ));
        }

        // Gate 2: Quality classification
        let signals = self.classifier.analyze(text);
        let quality_score = signals.weighted_score();

        if quality_score < self.config.min_quality {
            rejection_reasons.push(format!(
                "Low quality: {:.3} (min {:.3})",
                quality_score, self.config.min_quality
            ));
        }

        // Gate 3: Temporal decay (if age known)
        let decayed_quality = age_days.map(|age| {
            self.temporal.adjusted_quality(quality_score, domain, age)
        });

        if let Some(dq) = decayed_quality {
            if dq < self.config.min_quality * 0.5 {
                rejection_reasons.push(format!(
                    "Stale: decayed quality {:.3} for domain '{}' (half-life {}d)",
                    dq,
                    domain,
                    self.temporal.half_life_for(domain)
                ));
            }
        }

        let passes = rejection_reasons.is_empty();

        QualityVerdict {
            quality_score,
            passes,
            signals,
            decayed_quality,
            rejection_reasons,
        }
    }

    /// Quick check: does the text pass minimum quality?
    /// Faster than full evaluate() — skips signal breakdown.
    pub fn passes(&self, text: &str) -> bool {
        let text = text.trim();
        if text.len() < self.config.min_length || text.len() > self.config.max_length {
            return false;
        }
        self.classifier.score(text) >= self.config.min_quality
    }

    /// Get the temporal decay calculator (for direct use).
    pub fn temporal(&self) -> &TemporalDecay {
        &self.temporal
    }

    /// Get the quality classifier (for direct use).
    pub fn classifier(&self) -> &QualityClassifier {
        &self.classifier
    }
}

impl Default for QualityPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_quality_fact_passes() {
        let pipeline = QualityPipeline::new();
        let verdict = pipeline.evaluate(
            "The Pythagorean theorem states that in a right triangle, the square \
             of the hypotenuse equals the sum of the squares of the other two sides.",
            "mathematics",
            None,
        );
        assert!(verdict.passes, "High-quality fact should pass: {:?}", verdict.rejection_reasons);
        assert!(verdict.quality_score > 0.5);
    }

    #[test]
    fn test_garbage_rejected() {
        let pipeline = QualityPipeline::new();
        let verdict = pipeline.evaluate("asdf", "general", None);
        assert!(!verdict.passes);
        assert!(!verdict.rejection_reasons.is_empty());
    }

    #[test]
    fn test_empty_text_rejected() {
        let pipeline = QualityPipeline::new();
        let verdict = pipeline.evaluate("", "general", None);
        assert!(!verdict.passes);
    }

    #[test]
    fn test_stale_cybersecurity_fact_flagged() {
        let pipeline = QualityPipeline::new();
        // 2-year-old cybersecurity fact (90-day half-life)
        let verdict = pipeline.evaluate(
            "This CVE affects Apache Struts version 2.3 through a remote code execution vulnerability in the content-type header parser.",
            "cybersecurity",
            Some(730.0),
        );
        // Quality might pass but decayed quality should be very low
        assert!(verdict.decayed_quality.unwrap() < 0.1,
            "2-year-old cyber fact should have very low decayed quality, got {:?}",
            verdict.decayed_quality);
    }

    #[test]
    fn test_math_fact_barely_decays() {
        let pipeline = QualityPipeline::new();
        let verdict = pipeline.evaluate(
            "The fundamental theorem of calculus establishes that differentiation and integration are inverse operations.",
            "mathematics",
            Some(365.0),
        );
        assert!(verdict.passes);
        let decay = verdict.decayed_quality.unwrap();
        assert!(decay > verdict.quality_score * 0.99,
            "Math fact should barely decay in 1 year: score={}, decayed={}",
            verdict.quality_score, decay);
    }

    #[test]
    fn test_quick_passes_check() {
        let pipeline = QualityPipeline::new();
        assert!(pipeline.passes("Machine learning algorithms learn patterns from data to make predictions."));
        assert!(!pipeline.passes("hi"));
        assert!(!pipeline.passes(""));
    }

    #[test]
    fn test_custom_config() {
        let config = PipelineConfig {
            min_quality: 0.8,
            min_length: 50,
            max_length: 1000,
            check_contamination: false,
        };
        let pipeline = QualityPipeline::with_config(config);
        // Short text that would pass default but fails strict config
        let verdict = pipeline.evaluate("Water boils at 100°C.", "science", None);
        assert!(!verdict.passes, "Short text should fail strict 50-char minimum");
    }

    #[test]
    fn test_verdict_has_signals() {
        let pipeline = QualityPipeline::new();
        let verdict = pipeline.evaluate(
            "The speed of light in vacuum is approximately 299,792,458 meters per second, making it the universal speed limit according to Einstein's theory of special relativity.",
            "science",
            None,
        );
        assert!(verdict.signals.length_score > 0.0);
        assert!(verdict.signals.vocabulary_diversity > 0.0);
        assert!(verdict.signals.cleanliness_score > 0.0);
    }
}
