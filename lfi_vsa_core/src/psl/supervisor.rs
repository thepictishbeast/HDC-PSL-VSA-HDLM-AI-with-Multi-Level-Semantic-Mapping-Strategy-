// NODE 012: Probabilistic Soft Logic (PSL) Supervisor
// STATUS: ALPHA - Symbolic Governance Active
// PROTOCOL: CARTA / Materialist-Constraint-Layer

use tracing::{info, warn, error};
use crate::psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
use crate::hdc::error::HdcError;

pub struct PslSupervisor {
    pub axioms: Vec<Box<dyn Axiom>>,
    pub material_trust_threshold: f64,
}

impl PslSupervisor {
    pub fn new() -> Self {
        info!("// AUDIT: PSL Supervisor initialized. Materialist logic gating active.");
        Self {
            axioms: Vec::new(),
            material_trust_threshold: 0.75,
        }
    }

    pub fn register_axiom(&mut self, axiom: Box<dyn Axiom>) {
        info!("// AUDIT: Registering PSL Axiom: {}", axiom.id());
        self.axioms.push(axiom);
    }

    pub fn axiom_count(&self) -> usize {
        self.axioms.len()
    }

    /// CARTA AUDIT: Recalculates trust based on material reality vs. neural weights.
    pub fn audit(&self, target: &AuditTarget) -> Result<AxiomVerdict, HdcError> {
        let mut overall_confidence = 1.0;
        let mut reasoning = Vec::new();

        for axiom in &self.axioms {
            let verdict = axiom.evaluate(target).map_err(|e| {
                error!("// CRITICAL: Axiom evaluation fault: {}", e);
                HdcError::LogicFault { reason: format!("Axiom {} failed: {}", axiom.id(), e) }
            })?;

            overall_confidence *= verdict.confidence;
            reasoning.push(format!("{}: {}", axiom.id(), verdict.detail));
            
            if verdict.confidence < self.material_trust_threshold {
                warn!("// AUDIT: Axiom Violation detected: {}", axiom.id());
            }
        }

        if overall_confidence >= self.material_trust_threshold {
            Ok(AxiomVerdict::pass("PSL_GOVERNANCE".into(), overall_confidence, reasoning.join(" | ")))
        } else {
            Ok(AxiomVerdict::fail("PSL_GOVERNANCE".into(), overall_confidence, "Material trust threshold violation".into()))
        }
    }
}
