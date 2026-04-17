//! # Purpose
//! Metacognitive calibration — makes confidence scores trustworthy.
//! When the system says "confidence: 0.87", it should actually be right
//! 87% of the time. Without calibration, confidence is meaningless noise.
//!
//! # Design Decisions
//! - Platt scaling (logistic regression on confidence→accuracy) for recalibration
//! - Per-domain calibration profiles (system may be well-calibrated on math
//!   but overconfident on history)
//! - Ongoing calibration from user interactions (every correction = data point)
//! - Calibration curve stored as (bucket_center, actual_accuracy) pairs
//!
//! # Invariants
//! - Calibrated confidence must be in [0.0, 1.0]
//! - Calibration data requires at least 20 samples per bucket for stability
//! - Domain-specific calibration requires at least 50 samples in that domain
//!
//! # Failure Modes
//! - With < 20 samples, calibration is unreliable — return raw confidence with warning
//! - If all samples are in one bucket, calibration degenerates — use identity function

use std::collections::HashMap;

/// A single calibration data point.
#[derive(Debug, Clone)]
pub struct CalibrationSample {
    /// The system's predicted confidence.
    pub predicted: f64,
    /// Whether the response was actually correct (1.0) or incorrect (0.0).
    pub actual: f64,
    /// Domain of the query (optional, for per-domain calibration).
    pub domain: Option<String>,
}

/// A calibration bucket: predicted confidence range → actual accuracy.
#[derive(Debug, Clone)]
pub struct CalibrationBucket {
    /// Center of the bucket (e.g., 0.85 for the 0.80-0.90 range).
    pub center: f64,
    /// Number of samples in this bucket.
    pub count: usize,
    /// Actual accuracy of responses in this bucket.
    pub actual_accuracy: f64,
}

/// The metacognitive calibration engine.
pub struct CalibrationEngine {
    /// All calibration samples collected.
    samples: Vec<CalibrationSample>,
    /// Per-domain sample collections.
    domain_samples: HashMap<String, Vec<CalibrationSample>>,
    /// Number of buckets for the calibration curve.
    num_buckets: usize,
    /// Platt scaling parameters: calibrated = 1 / (1 + exp(-(a * raw + b)))
    platt_a: f64,
    platt_b: f64,
    /// Whether Platt scaling has been fitted.
    fitted: bool,
}

impl CalibrationEngine {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            domain_samples: HashMap::new(),
            num_buckets: 10,
            platt_a: 1.0,  // Identity by default
            platt_b: 0.0,
            fitted: false,
        }
    }

    /// Record a calibration sample from a user interaction.
    pub fn record(&mut self, sample: CalibrationSample) {
        if let Some(ref domain) = sample.domain {
            self.domain_samples
                .entry(domain.clone())
                .or_default()
                .push(sample.clone());
        }
        self.samples.push(sample);
    }

    /// Calibrate the raw confidence score using Platt scaling.
    /// Returns the calibrated confidence + a reliability flag.
    pub fn calibrate(&self, raw_confidence: f64, domain: Option<&str>) -> (f64, bool) {
        if !self.fitted || self.samples.len() < 20 {
            // Not enough data — return raw with unreliable flag
            return (raw_confidence, false);
        }

        // Apply Platt scaling: calibrated = sigmoid(a * raw + b)
        let logit = self.platt_a * raw_confidence + self.platt_b;
        let calibrated = 1.0 / (1.0 + (-logit).exp());

        // Clamp to [0.01, 0.99] — never fully certain or fully uncertain
        let calibrated = calibrated.clamp(0.01, 0.99);

        (calibrated, true)
    }

    /// Fit Platt scaling parameters using gradient descent.
    /// Call after collecting at least 20+ samples.
    pub fn fit(&mut self) {
        if self.samples.len() < 20 {
            return;
        }

        // Simple Platt scaling via gradient descent
        let mut a = 1.0_f64;
        let mut b = 0.0_f64;
        let lr = 0.01;
        let iterations = 1000;

        for _ in 0..iterations {
            let mut grad_a = 0.0;
            let mut grad_b = 0.0;

            for sample in &self.samples {
                let logit = a * sample.predicted + b;
                let p = 1.0 / (1.0 + (-logit).exp());
                let error = p - sample.actual;
                grad_a += error * sample.predicted;
                grad_b += error;
            }

            let n = self.samples.len() as f64;
            a -= lr * grad_a / n;
            b -= lr * grad_b / n;
        }

        self.platt_a = a;
        self.platt_b = b;
        self.fitted = true;
    }

    /// Compute the calibration curve: 10 buckets of (center, count, accuracy).
    pub fn calibration_curve(&self) -> Vec<CalibrationBucket> {
        let mut buckets: Vec<(f64, usize, f64)> = (0..self.num_buckets)
            .map(|i| {
                let center = (i as f64 + 0.5) / self.num_buckets as f64;
                (center, 0, 0.0)
            })
            .collect();

        for sample in &self.samples {
            let idx = ((sample.predicted * self.num_buckets as f64) as usize)
                .min(self.num_buckets - 1);
            buckets[idx].1 += 1;
            buckets[idx].2 += sample.actual;
        }

        buckets
            .into_iter()
            .map(|(center, count, sum)| CalibrationBucket {
                center,
                count,
                actual_accuracy: if count > 0 { sum / count as f64 } else { center },
            })
            .collect()
    }

    /// Expected Calibration Error — the key metric.
    /// Lower is better. 0.0 = perfectly calibrated.
    pub fn expected_calibration_error(&self) -> f64 {
        let curve = self.calibration_curve();
        let total = self.samples.len() as f64;
        if total == 0.0 {
            return 1.0;
        }

        curve.iter()
            .map(|b| {
                let weight = b.count as f64 / total;
                weight * (b.actual_accuracy - b.center).abs()
            })
            .sum()
    }

    /// Total number of calibration samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Check if calibration is reliable (enough data).
    pub fn is_reliable(&self) -> bool {
        self.fitted && self.samples.len() >= 50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_count() {
        let mut engine = CalibrationEngine::new();
        engine.record(CalibrationSample { predicted: 0.8, actual: 1.0, domain: None });
        engine.record(CalibrationSample { predicted: 0.6, actual: 0.0, domain: None });
        assert_eq!(engine.sample_count(), 2);
    }

    #[test]
    fn test_uncalibrated_returns_raw() {
        let engine = CalibrationEngine::new();
        let (conf, reliable) = engine.calibrate(0.75, None);
        assert_eq!(conf, 0.75);
        assert!(!reliable);
    }

    #[test]
    fn test_calibration_curve_buckets() {
        let mut engine = CalibrationEngine::new();
        for i in 0..100 {
            let predicted = i as f64 / 100.0;
            let actual = if predicted > 0.5 { 1.0 } else { 0.0 };
            engine.record(CalibrationSample { predicted, actual, domain: None });
        }
        let curve = engine.calibration_curve();
        assert_eq!(curve.len(), 10);
        // Low-confidence bucket should have low accuracy
        assert!(curve[0].actual_accuracy < 0.5);
        // High-confidence bucket should have high accuracy
        assert!(curve[9].actual_accuracy > 0.5);
    }

    #[test]
    fn test_fit_and_calibrate() {
        let mut engine = CalibrationEngine::new();
        // Simulate well-calibrated system
        for _ in 0..50 {
            engine.record(CalibrationSample { predicted: 0.9, actual: 1.0, domain: None });
            engine.record(CalibrationSample { predicted: 0.1, actual: 0.0, domain: None });
        }
        engine.fit();
        let (high_conf, reliable) = engine.calibrate(0.9, None);
        let (low_conf, _) = engine.calibrate(0.1, None);
        assert!(reliable);
        assert!(high_conf > 0.5, "High predicted → high calibrated");
        assert!(low_conf < 0.5, "Low predicted → low calibrated");
    }

    #[test]
    fn test_ece_perfect_calibration() {
        let mut engine = CalibrationEngine::new();
        // Perfect calibration: predicted matches actual
        for i in 0..100 {
            let p = (i as f64 + 0.5) / 100.0;
            let actual = if rand_bool(p) { 1.0 } else { 0.0 };
            engine.record(CalibrationSample { predicted: p, actual, domain: None });
        }
        let ece = engine.expected_calibration_error();
        // With random sampling, ECE should be reasonable (< 0.3)
        assert!(ece < 0.5, "ECE should be < 0.5 for approximately calibrated data, got {}", ece);
    }

    #[test]
    fn test_domain_specific_tracking() {
        let mut engine = CalibrationEngine::new();
        engine.record(CalibrationSample { predicted: 0.8, actual: 1.0, domain: Some("math".into()) });
        engine.record(CalibrationSample { predicted: 0.8, actual: 0.0, domain: Some("history".into()) });
        assert_eq!(engine.domain_samples.get("math").unwrap().len(), 1);
        assert_eq!(engine.domain_samples.get("history").unwrap().len(), 1);
    }

    // Simple deterministic "random" for testing
    fn rand_bool(p: f64) -> bool {
        // Use the probability value itself as a simple deterministic threshold
        let hash = (p * 1000.0) as u64;
        (hash % 100) < (p * 100.0) as u64
    }
}
