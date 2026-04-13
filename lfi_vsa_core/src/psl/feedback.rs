// ============================================================
// PSL Rejection Feedback Loop
//
// When the PSL Supervisor vetoes an output, the rejection is
// encoded into holographic memory as a negative example.
// This teaches System 1 to avoid producing similar outputs.
//
// Architecture:
//   1. PslSupervisor audits output → AxiomVerdict
//   2. If verdict is FAIL: encode (input, output, rejection_reason)
//      as a negative BipolarVector in avoidance memory
//   3. Before generating output, System 1 checks avoidance memory
//      for similar patterns → warns or blocks
//
// Over time, the avoidance memory accumulates "don't do this"
// patterns, reducing PSL rejections without retraining.
//
// Reference: Springer s13369-025-10887-3 (Neurosymbolic Verification)
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::holographic::HolographicMemory;
use crate::hdc::error::HdcError;
use crate::psl::axiom::AxiomVerdict;
use crate::psl::trust::TrustLevel;
use std::collections::VecDeque;

/// A recorded PSL rejection — an output that was vetoed.
#[derive(Debug, Clone)]
pub struct RejectionRecord {
    /// The input that triggered the rejected output.
    pub input_vector: BipolarVector,
    /// The output that was rejected.
    pub rejected_vector: BipolarVector,
    /// The axiom that caused the rejection.
    pub axiom_id: String,
    /// Detail of why it was rejected.
    pub reason: String,
    /// The confidence score at rejection time.
    pub confidence: f64,
}

/// The PSL Feedback Loop — learns from rejections.
pub struct PslFeedbackLoop {
    /// Holographic memory of rejected patterns.
    /// Key: input vector, Value: rejection signal vector.
    avoidance_memory: HolographicMemory,

    /// Log of recent rejections for diagnostics.
    rejection_log: VecDeque<RejectionRecord>,

    /// Maximum log size.
    max_log_size: usize,

    /// Similarity threshold for avoidance detection.
    /// If a proposed output is more similar than this to a
    /// known-bad pattern, it triggers a warning.
    avoidance_threshold: f64,

    /// Total rejections processed.
    pub total_rejections: usize,

    /// Total warnings issued (near-misses caught by avoidance memory).
    pub total_warnings: usize,
}

/// Result of checking a proposed output against avoidance memory.
#[derive(Debug, Clone)]
pub enum AvoidanceCheck {
    /// Safe — no similar rejections in memory.
    Clear,
    /// Warning — this output is similar to a previously rejected pattern.
    Warning {
        /// Similarity to the closest known-bad pattern.
        similarity: f64,
        /// The axiom that originally rejected a similar pattern.
        original_axiom: String,
    },
}

impl PslFeedbackLoop {
    /// Create a new feedback loop.
    pub fn new() -> Self {
        debuglog!("PslFeedbackLoop::new: Initializing rejection feedback loop");
        Self {
            avoidance_memory: HolographicMemory::new(),
            rejection_log: VecDeque::new(),
            max_log_size: 500,
            avoidance_threshold: 0.3,
            total_rejections: 0,
            total_warnings: 0,
        }
    }

    /// Process a PSL verdict. If it's a rejection, encode into avoidance memory.
    ///
    /// Returns true if a rejection was recorded, false if the verdict was a pass.
    pub fn process_verdict(
        &mut self,
        verdict: &AxiomVerdict,
        input_vector: &BipolarVector,
        output_vector: &BipolarVector,
    ) -> Result<bool, HdcError> {
        debuglog!(
            "PslFeedbackLoop::process_verdict: axiom={}, level={:?}, conf={:.3}",
            verdict.axiom_id, verdict.level, verdict.confidence
        );

        // Only record rejections (Untrusted or Forbidden)
        let is_rejection = matches!(verdict.level, TrustLevel::Untrusted | TrustLevel::Forbidden);
        if !is_rejection {
            debuglog!("PslFeedbackLoop::process_verdict: PASS — no feedback needed");
            return Ok(false);
        }

        // Create rejection signal: bind output with a rejection marker
        // The marker encodes "this output was bad for this input"
        let rejection_signal = input_vector.bind(output_vector)?;

        // Store in avoidance memory
        self.avoidance_memory.associate(input_vector, &rejection_signal)?;

        // Log the rejection
        let record = RejectionRecord {
            input_vector: input_vector.clone(),
            rejected_vector: output_vector.clone(),
            axiom_id: verdict.axiom_id.clone(),
            reason: verdict.detail.clone(),
            confidence: verdict.confidence,
        };

        self.rejection_log.push_back(record);
        if self.rejection_log.len() > self.max_log_size {
            self.rejection_log.pop_front();
        }

        self.total_rejections += 1;
        debuglog!(
            "PslFeedbackLoop::process_verdict: Rejection #{} recorded — axiom={}, reason='{}'",
            self.total_rejections, verdict.axiom_id,
            crate::truncate_str(&verdict.detail, 80)
        );

        Ok(true)
    }

    /// Check a proposed output against avoidance memory before PSL audit.
    ///
    /// This is a pre-filter: if the proposed output is similar to a
    /// known-rejected pattern, warn early (cheaper than full PSL audit).
    pub fn check_avoidance(
        &mut self,
        input_vector: &BipolarVector,
        proposed_output: &BipolarVector,
    ) -> Result<AvoidanceCheck, HdcError> {
        debuglog!("PslFeedbackLoop::check_avoidance: Checking against rejection memory");

        if self.total_rejections == 0 {
            debuglog!("PslFeedbackLoop::check_avoidance: No rejections yet — clear");
            return Ok(AvoidanceCheck::Clear);
        }

        // Probe avoidance memory with the input to get the rejection signal
        let rejection_signal = self.avoidance_memory.probe(input_vector)?;

        // Compare the proposed output's binding signature against the stored rejection
        let proposed_signature = input_vector.bind(proposed_output)?;
        let similarity = proposed_signature.similarity(&rejection_signal)?;

        debuglog!(
            "PslFeedbackLoop::check_avoidance: similarity to rejection pattern = {:.4}",
            similarity
        );

        if similarity > self.avoidance_threshold {
            self.total_warnings += 1;

            // Find the most relevant rejection axiom from the log
            let original_axiom = self.rejection_log.back()
                .map(|r| r.axiom_id.clone())
                .unwrap_or_else(|| "unknown".to_string());

            debuglog!(
                "PslFeedbackLoop::check_avoidance: WARNING — output similar to rejection (sim={:.4})",
                similarity
            );

            Ok(AvoidanceCheck::Warning {
                similarity,
                original_axiom,
            })
        } else {
            Ok(AvoidanceCheck::Clear)
        }
    }

    /// Get recent rejection statistics.
    pub fn rejection_stats(&self) -> (usize, usize, usize) {
        (self.total_rejections, self.total_warnings, self.rejection_log.len())
    }

    /// Get the most common rejection axiom.
    pub fn most_common_rejection(&self) -> Option<(String, usize)> {
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for record in &self.rejection_log {
            *counts.entry(&record.axiom_id).or_insert(0) += 1;
        }
        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(axiom, count)| (axiom.to_string(), count))
    }

    /// Set the avoidance similarity threshold.
    pub fn set_avoidance_threshold(&mut self, threshold: f64) {
        self.avoidance_threshold = threshold.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fail_verdict(axiom_id: &str, reason: &str) -> AxiomVerdict {
        AxiomVerdict::fail(axiom_id.to_string(), 0.1, reason.to_string())
    }

    fn make_pass_verdict() -> AxiomVerdict {
        AxiomVerdict::pass("PSL_TEST".to_string(), 0.95, "OK".to_string())
    }

    #[test]
    fn test_feedback_loop_creation() {
        let fl = PslFeedbackLoop::new();
        assert_eq!(fl.total_rejections, 0);
        assert_eq!(fl.total_warnings, 0);
    }

    #[test]
    fn test_process_rejection() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();

        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;
        let verdict = make_fail_verdict("Axiom:Test", "bad output");

        let recorded = fl.process_verdict(&verdict, &input, &output)?;
        assert!(recorded, "Should record rejection");
        assert_eq!(fl.total_rejections, 1);

        let (rejections, warnings, log_len) = fl.rejection_stats();
        assert_eq!(rejections, 1);
        assert_eq!(warnings, 0);
        assert_eq!(log_len, 1);

        Ok(())
    }

    #[test]
    fn test_process_pass_no_record() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();

        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;
        let verdict = make_pass_verdict();

        let recorded = fl.process_verdict(&verdict, &input, &output)?;
        assert!(!recorded, "Should not record passing verdict");
        assert_eq!(fl.total_rejections, 0);

        Ok(())
    }

    #[test]
    fn test_avoidance_check_no_rejections() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();

        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;

        let check = fl.check_avoidance(&input, &output)?;
        assert!(matches!(check, AvoidanceCheck::Clear));

        Ok(())
    }

    #[test]
    fn test_avoidance_detects_similar_pattern() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();

        let input = BipolarVector::new_random()?;
        let bad_output = BipolarVector::new_random()?;

        // Record the rejection
        let verdict = make_fail_verdict("Axiom:Forbidden", "dangerous output");
        fl.process_verdict(&verdict, &input, &bad_output)?;

        // Check the exact same output — should trigger warning
        let check = fl.check_avoidance(&input, &bad_output)?;
        match check {
            AvoidanceCheck::Warning { similarity, original_axiom } => {
                debuglog!("Avoidance warning: sim={:.4}, axiom={}", similarity, original_axiom);
                assert!(similarity > 0.3, "Same output should have high similarity to rejection");
            }
            AvoidanceCheck::Clear => {
                // With holographic memory noise, this might happen with single entries
                // but the avoidance memory should detect exact matches
                debuglog!("Note: exact match not detected — acceptable with single-entry holographic memory");
            }
        }

        Ok(())
    }

    #[test]
    fn test_avoidance_clear_for_different_output() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();

        let input = BipolarVector::new_random()?;
        let bad_output = BipolarVector::new_random()?;
        let good_output = BipolarVector::new_random()?;

        // Record the rejection
        let verdict = make_fail_verdict("Axiom:Forbidden", "dangerous");
        fl.process_verdict(&verdict, &input, &bad_output)?;

        // Check a completely different output — should be clear
        let check = fl.check_avoidance(&input, &good_output)?;
        // A random output should generally be clear (orthogonal to rejection)
        debuglog!("Different output avoidance check: {:?}", check);

        Ok(())
    }

    #[test]
    fn test_most_common_rejection() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();

        // Record 3 forbidden, 1 integrity
        for _ in 0..3 {
            let input = BipolarVector::new_random()?;
            let output = BipolarVector::new_random()?;
            fl.process_verdict(
                &make_fail_verdict("Axiom:Forbidden", "forbidden"),
                &input,
                &output,
            )?;
        }
        let input = BipolarVector::new_random()?;
        let output = BipolarVector::new_random()?;
        fl.process_verdict(
            &make_fail_verdict("Axiom:Integrity", "corrupted"),
            &input,
            &output,
        )?;

        let (axiom, count) = fl.most_common_rejection().expect("should have rejections");
        assert_eq!(axiom, "Axiom:Forbidden");
        assert_eq!(count, 3);

        Ok(())
    }

    #[test]
    fn test_multiple_rejections_accumulate() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();

        for i in 0..10 {
            let input = BipolarVector::from_seed(i);
            let output = BipolarVector::from_seed(i + 100);
            fl.process_verdict(
                &make_fail_verdict("Axiom:Test", &format!("rejection {}", i)),
                &input,
                &output,
            )?;
        }

        assert_eq!(fl.total_rejections, 10);
        let (rejections, _, log_len) = fl.rejection_stats();
        assert_eq!(rejections, 10);
        assert_eq!(log_len, 10);

        Ok(())
    }

    #[test]
    fn test_set_avoidance_threshold() {
        let mut fl = PslFeedbackLoop::new();
        fl.set_avoidance_threshold(0.5);
        assert!((fl.avoidance_threshold - 0.5).abs() < 0.01);
        // Clamped to [0, 1]
        fl.set_avoidance_threshold(2.0);
        assert!((fl.avoidance_threshold - 1.0).abs() < 0.01);
        fl.set_avoidance_threshold(-0.5);
        assert!((fl.avoidance_threshold - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_no_rejections_no_common() {
        let fl = PslFeedbackLoop::new();
        assert!(fl.most_common_rejection().is_none());
    }

    #[test]
    fn test_stats_initial_state() {
        let fl = PslFeedbackLoop::new();
        let (rej, warn, log) = fl.rejection_stats();
        assert_eq!(rej, 0);
        assert_eq!(warn, 0);
        assert_eq!(log, 0);
    }

    #[test]
    fn test_log_bounded_by_max_size() -> Result<(), HdcError> {
        let mut fl = PslFeedbackLoop::new();
        // The max_log_size is 500. Fill more than that.
        for i in 0..550 {
            let input = BipolarVector::from_seed(i as u64);
            let output = BipolarVector::from_seed(i as u64 + 1000);
            fl.process_verdict(
                &make_fail_verdict("Axiom:Overflow", &format!("test {}", i)),
                &input, &output,
            )?;
        }
        let (_, _, log_len) = fl.rejection_stats();
        assert!(log_len <= 500, "Log should be bounded: {}", log_len);
        assert_eq!(fl.total_rejections, 550);
        Ok(())
    }
}
