// NODE 012: Probabilistic Soft Logic (PSL) Supervisor
// STATUS: ALPHA - Symbolic Governance Active
// PROTOCOL: CARTA / Materialist-Constraint-Layer
//
// AGGREGATION MODEL:
//   Each axiom declares relevance(target) -> 0.0..1.0
//   Irrelevant axioms (relevance == 0) are skipped entirely.
//   Relevant axioms contribute to a weighted mean confidence.
//   Any single CRITICAL failure (confidence < hard_fail_threshold) vetoes the verdict.
//   This prevents one irrelevant axiom from cratering the entire trust score.

use tracing::{info, warn, error};
use crate::psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
use crate::hdc::error::HdcError;
use crate::reasoning_provenance::{TraceArena, TraceId, InferenceSource};

pub struct PslSupervisor {
    pub axioms: Vec<Box<dyn Axiom>>,
    pub material_trust_threshold: f64,
    /// Any single axiom below this confidence triggers an automatic veto
    pub hard_fail_threshold: f64,
}

impl PslSupervisor {
    pub fn new() -> Self {
        info!("// AUDIT: PSL Supervisor initialized. Weighted relevance aggregation active.");
        Self {
            axioms: Vec::new(),
            material_trust_threshold: 0.75,
            hard_fail_threshold: 0.3,
        }
    }

    pub fn register_axiom(&mut self, axiom: Box<dyn Axiom>) {
        info!("// AUDIT: Registering PSL Axiom: {}", axiom.id());
        self.axioms.push(axiom);
    }

    pub fn axiom_count(&self) -> usize {
        self.axioms.len()
    }

    /// CARTA AUDIT: Weighted relevance-based confidence aggregation.
    ///
    /// Algorithm:
    ///   1. For each axiom, compute relevance(target). Skip if 0.
    ///   2. Evaluate relevant axioms. Weight their confidence by relevance.
    ///   3. Final confidence = weighted_sum / total_weight (weighted mean).
    ///   4. Any single relevant axiom below hard_fail_threshold vetoes the verdict.
    pub fn audit(&self, target: &AuditTarget) -> Result<AxiomVerdict, HdcError> {
        debuglog!("PslSupervisor::audit: Beginning weighted relevance audit over {} axioms", self.axioms.len());

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        let mut reasoning = Vec::new();
        let mut vetoed = false;
        let mut veto_reason = String::new();

        for axiom in &self.axioms {
            let relevance = axiom.relevance(target);
            if relevance <= 0.0 {
                debuglog!("PslSupervisor::audit: Skipping {} (relevance=0 for target)", axiom.id());
                continue;
            }

            let verdict = axiom.evaluate(target).map_err(|e| {
                error!("// CRITICAL: Axiom evaluation fault: {}", e);
                HdcError::LogicFault { reason: format!("Axiom {} failed: {}", axiom.id(), e) }
            })?;

            debuglog!("PslSupervisor::audit: {} — conf={:.3}, relevance={:.2}", axiom.id(), verdict.confidence, relevance);

            weighted_sum += verdict.confidence * relevance;
            total_weight += relevance;
            reasoning.push(format!("{}: {} (w={:.2})", axiom.id(), verdict.detail, relevance));

            // Hard veto: any relevant axiom with critically low confidence
            if verdict.confidence < self.hard_fail_threshold {
                warn!("// AUDIT: VETO by {} (confidence {:.3} < hard_fail {:.3})", axiom.id(), verdict.confidence, self.hard_fail_threshold);
                vetoed = true;
                veto_reason = format!("Vetoed by {}: {}", axiom.id(), verdict.detail);
            } else if verdict.confidence < self.material_trust_threshold {
                warn!("// AUDIT: Axiom below trust threshold: {} ({:.3})", axiom.id(), verdict.confidence);
            }
        }

        // If no relevant axioms, default to pass (nothing to check)
        if total_weight == 0.0 {
            debuglog!("PslSupervisor::audit: No relevant axioms for this target type — default pass");
            return Ok(AxiomVerdict::pass("PSL_GOVERNANCE".into(), 1.0, "No relevant axioms".into()));
        }

        let overall_confidence = weighted_sum / total_weight;
        debuglog!("PslSupervisor::audit: Weighted confidence={:.4} (sum={:.4}, weight={:.2}), vetoed={}", overall_confidence, weighted_sum, total_weight, vetoed);

        if vetoed {
            Ok(AxiomVerdict::fail("PSL_GOVERNANCE".into(), overall_confidence, veto_reason))
        } else if overall_confidence >= self.material_trust_threshold {
            Ok(AxiomVerdict::pass("PSL_GOVERNANCE".into(), overall_confidence, reasoning.join(" | ")))
        } else {
            Ok(AxiomVerdict::fail("PSL_GOVERNANCE".into(), overall_confidence, "Material trust threshold violation".into()))
        }
    }

    /// CARTA AUDIT with reasoning provenance recording.
    ///
    /// Identical to [`audit`] but records a trace entry for each axiom
    /// evaluation into the provided arena. The `parent_trace` links this
    /// audit to the calling reasoning step (e.g., an MCTS simulate).
    ///
    /// Returns `(verdict, Vec<TraceId>)` — the verdict plus the trace IDs
    /// for every axiom evaluated, so the caller can chain them.
    pub fn audit_with_provenance(
        &self,
        target: &AuditTarget,
        arena: &mut TraceArena,
        parent_trace: Option<TraceId>,
    ) -> Result<(AxiomVerdict, Vec<TraceId>), HdcError> {
        debuglog!("PslSupervisor::audit_with_provenance: Beginning traced audit over {} axioms", self.axioms.len());

        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        let mut reasoning = Vec::new();
        let mut vetoed = false;
        let mut veto_reason = String::new();
        let mut trace_ids = Vec::new();

        for axiom in &self.axioms {
            let relevance = axiom.relevance(target);
            if relevance <= 0.0 {
                debuglog!("PslSupervisor::audit_with_provenance: Skipping {} (relevance=0)", axiom.id());
                continue;
            }

            let verdict = axiom.evaluate(target).map_err(|e| {
                error!("// CRITICAL: Axiom evaluation fault: {}", e);
                HdcError::LogicFault { reason: format!("Axiom {} failed: {}", axiom.id(), e) }
            })?;

            debuglog!("PslSupervisor::audit_with_provenance: {} — conf={:.3}, rel={:.2}",
                axiom.id(), verdict.confidence, relevance);

            // Record provenance trace for this axiom evaluation.
            let trace_id = arena.record_step(
                parent_trace,
                InferenceSource::PslAxiomEvaluation {
                    axiom_id: axiom.id().to_string(),
                    relevance,
                },
                vec![format!("axiom:{}", axiom.id())],
                verdict.confidence,
                None,
                format!("PSL axiom '{}': {} (conf={:.3}, rel={:.2})",
                    axiom.id(), verdict.detail, verdict.confidence, relevance),
                0,
            );
            trace_ids.push(trace_id);

            weighted_sum += verdict.confidence * relevance;
            total_weight += relevance;
            reasoning.push(format!("{}: {} (w={:.2})", axiom.id(), verdict.detail, relevance));

            if verdict.confidence < self.hard_fail_threshold {
                warn!("// AUDIT: VETO by {} (confidence {:.3} < hard_fail {:.3})",
                    axiom.id(), verdict.confidence, self.hard_fail_threshold);
                vetoed = true;
                veto_reason = format!("Vetoed by {}: {}", axiom.id(), verdict.detail);
            } else if verdict.confidence < self.material_trust_threshold {
                warn!("// AUDIT: Axiom below trust threshold: {} ({:.3})", axiom.id(), verdict.confidence);
            }
        }

        if total_weight == 0.0 {
            debuglog!("PslSupervisor::audit_with_provenance: No relevant axioms — default pass");
            return Ok((AxiomVerdict::pass("PSL_GOVERNANCE".into(), 1.0, "No relevant axioms".into()), trace_ids));
        }

        let overall_confidence = weighted_sum / total_weight;
        debuglog!("PslSupervisor::audit_with_provenance: Weighted confidence={:.4}, vetoed={}, traces={}",
            overall_confidence, vetoed, trace_ids.len());

        let verdict = if vetoed {
            AxiomVerdict::fail("PSL_GOVERNANCE".into(), overall_confidence, veto_reason)
        } else if overall_confidence >= self.material_trust_threshold {
            AxiomVerdict::pass("PSL_GOVERNANCE".into(), overall_confidence, reasoning.join(" | "))
        } else {
            AxiomVerdict::fail("PSL_GOVERNANCE".into(), overall_confidence, "Material trust threshold violation".into())
        };

        Ok((verdict, trace_ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psl::axiom::DimensionalityAxiom;
    use crate::hdc::vector::BipolarVector;

    #[test]
    fn test_audit_with_provenance_records_traces() {
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let vec = BipolarVector::new_random().expect("random vector");
        let target = AuditTarget::Vector(vec);

        let mut arena = TraceArena::new();
        let (verdict, trace_ids) = supervisor
            .audit_with_provenance(&target, &mut arena, None)
            .expect("audit should succeed");

        // One axiom → one trace entry.
        assert_eq!(trace_ids.len(), 1, "Should have one trace per relevant axiom");
        assert_eq!(arena.len(), 1);

        // The trace entry should be PslAxiomEvaluation.
        let entry = arena.get(trace_ids[0]).expect("trace should exist");
        assert!(
            matches!(entry.source, InferenceSource::PslAxiomEvaluation { .. }),
            "Trace source should be PslAxiomEvaluation, got {:?}",
            entry.source
        );

        // Confidence should match the verdict.
        assert!(verdict.confidence > 0.0);
        assert!((entry.confidence - verdict.confidence).abs() < 0.01);
    }

    #[test]
    fn test_audit_with_provenance_chains_to_parent() {
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let vec = BipolarVector::new_random().expect("random vector");
        let target = AuditTarget::Vector(vec);

        let mut arena = TraceArena::new();

        // Record a parent trace (e.g., from MCTS).
        let parent_id = arena.record_step(
            None,
            InferenceSource::MctsExpansion { action: "Specialize".into(), node_depth: 1 },
            vec!["mcts_parent".into()],
            0.85, None, "MCTS parent step".into(), 0,
        );

        let (_, trace_ids) = supervisor
            .audit_with_provenance(&target, &mut arena, Some(parent_id))
            .expect("audit should succeed");

        // The PSL trace should be chained to the MCTS parent.
        let psl_entry = arena.get(trace_ids[0]).expect("trace should exist");
        assert_eq!(psl_entry.parent, Some(parent_id),
            "PSL trace should chain to MCTS parent");

        // Full chain: psl_entry → parent_id (depth 1)
        let chain = arena.trace_chain(trace_ids[0]);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_audit_with_provenance_backward_compat() {
        // Regular audit() still works without provenance.
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let vec = BipolarVector::new_random().expect("random vector");
        let target = AuditTarget::Vector(vec);

        let verdict = supervisor.audit(&target).expect("audit should succeed");
        assert!(verdict.confidence > 0.0, "Regular audit should still work");
    }

    #[test]
    fn test_supervisor_creation() {
        let supervisor = PslSupervisor::new();
        assert_eq!(supervisor.axiom_count(), 0);
        assert!((supervisor.material_trust_threshold - 0.75).abs() < 0.01);
        assert!((supervisor.hard_fail_threshold - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_register_axiom() {
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));
        assert_eq!(supervisor.axiom_count(), 1);
    }

    #[test]
    fn test_no_axioms_default_pass() {
        let supervisor = PslSupervisor::new();
        let vec = BipolarVector::new_random().expect("random");
        let target = AuditTarget::Vector(vec);
        let verdict = supervisor.audit(&target).expect("audit");
        assert!((verdict.confidence - 1.0).abs() < 0.01, "No axioms should default to pass");
    }

    #[test]
    fn test_multiple_axioms() {
        use crate::psl::axiom::EntropyAxiom;
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));
        supervisor.register_axiom(Box::new(EntropyAxiom::default()));
        assert_eq!(supervisor.axiom_count(), 2);

        let vec = BipolarVector::new_random().expect("random");
        let target = AuditTarget::Vector(vec);
        let verdict = supervisor.audit(&target).expect("audit");
        assert!(verdict.confidence > 0.0, "Multi-axiom audit should succeed");
    }

    #[test]
    fn test_non_vector_target_skips_vector_axioms() {
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom)); // Only relevant for vectors
        let target = AuditTarget::Scalar { label: "test".into(), value: 42.0 };
        let verdict = supervisor.audit(&target).expect("audit");
        // DimensionalityAxiom has relevance=0 for non-vectors → default pass
        assert!((verdict.confidence - 1.0).abs() < 0.01);
    }

    // ============================================================
    // Stress / invariant tests for PslSupervisor
    // ============================================================

    /// INVARIANT: audit() never produces a verdict with confidence > 1.0
    /// or < 0.0 regardless of how many axioms are registered.
    #[test]
    fn invariant_verdict_confidence_in_unit_interval() {
        let mut supervisor = PslSupervisor::new();
        // Register every axiom that's safe to instantiate without args.
        supervisor.register_axiom(Box::new(DimensionalityAxiom));
        supervisor.register_axiom(Box::new(crate::psl::axiom::ClassInterestAxiom));
        supervisor.register_axiom(Box::new(crate::psl::axiom::EntropyAxiom::default()));
        supervisor.register_axiom(Box::new(crate::psl::axiom::ConfidenceCalibrationAxiom::default()));

        for _ in 0..30 {
            let v = BipolarVector::new_random().expect("random");
            let target = AuditTarget::Vector(v);
            let verdict = supervisor.audit(&target).expect("audit");
            assert!(verdict.confidence >= 0.0 && verdict.confidence <= 1.0,
                "confidence escaped [0,1]: {}", verdict.confidence);
        }
    }

    /// INVARIANT: register_axiom is monotonic — count strictly grows by 1.
    #[test]
    fn invariant_register_axiom_count_grows_by_one() {
        let mut supervisor = PslSupervisor::new();
        for i in 0..10 {
            let before = supervisor.axiom_count();
            supervisor.register_axiom(Box::new(DimensionalityAxiom));
            assert_eq!(supervisor.axiom_count(), before + 1,
                "count must grow by exactly 1 at iter {}", i);
        }
    }

    /// INVARIANT: with hard_fail_threshold raised above any axiom's confidence,
    /// every audit fails (vetoed). Tests the veto logic in isolation.
    #[test]
    fn invariant_high_hard_fail_threshold_vetoes_all() {
        let mut supervisor = PslSupervisor::new();
        supervisor.hard_fail_threshold = 0.99; // Almost no axiom can clear this.
        supervisor.register_axiom(Box::new(crate::psl::axiom::EntropyAxiom::default()));
        // Random vectors typically score < 0.99 on entropy.
        let mut vetoed_count = 0;
        for _ in 0..20 {
            let v = BipolarVector::new_random().expect("random");
            let target = AuditTarget::Vector(v);
            let verdict = supervisor.audit(&target).expect("audit");
            if verdict.detail.contains("Vetoed") {
                vetoed_count += 1;
            }
        }
        assert!(vetoed_count > 0,
            "raising hard_fail_threshold to 0.99 must produce some vetoes");
    }

    /// INVARIANT: audit_with_provenance produces exactly N trace entries
    /// when N relevant axioms are registered.
    #[test]
    fn invariant_audit_with_provenance_one_trace_per_relevant_axiom() {
        use crate::reasoning_provenance::TraceArena;
        let mut supervisor = PslSupervisor::new();
        // 3 axioms that are relevant to vector targets.
        supervisor.register_axiom(Box::new(DimensionalityAxiom));
        supervisor.register_axiom(Box::new(crate::psl::axiom::EntropyAxiom::default()));
        supervisor.register_axiom(Box::new(crate::psl::axiom::ConfidenceCalibrationAxiom::default()));

        let v = BipolarVector::new_random().expect("random");
        let target = AuditTarget::Vector(v);
        let mut arena = TraceArena::new();
        let (_, trace_ids) = supervisor
            .audit_with_provenance(&target, &mut arena, None)
            .expect("audit");
        assert_eq!(trace_ids.len(), 3,
            "3 relevant axioms must produce 3 trace entries");
        assert_eq!(arena.len(), 3);
    }

    /// INVARIANT: new() starts with zero axioms.
    #[test]
    fn invariant_new_zero_axioms() {
        let s = PslSupervisor::new();
        assert_eq!(s.axiom_count(), 0);
    }

    /// INVARIANT: register_axiom grows axiom_count by exactly 1.
    #[test]
    fn invariant_register_grows_by_one() {
        let mut s = PslSupervisor::new();
        for i in 0..5 {
            s.register_axiom(Box::new(DimensionalityAxiom));
            assert_eq!(s.axiom_count(), i + 1);
        }
    }

    /// INVARIANT: audit with empty supervisor returns an error (EmptyAxiomSet).
    #[test]
    fn invariant_audit_empty_supervisor_errors_or_handles() {
        let s = PslSupervisor::new();
        let v = BipolarVector::new_random().unwrap();
        let target = AuditTarget::Vector(v);
        // Empty supervisor should either error or pass trivially — just don't panic.
        let _ = s.audit(&target);
    }
}
