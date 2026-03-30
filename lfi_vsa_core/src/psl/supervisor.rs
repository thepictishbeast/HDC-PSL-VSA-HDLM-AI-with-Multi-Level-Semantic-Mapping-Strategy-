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
}
