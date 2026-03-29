// NODE 018: NeuPSL World-State Predicates
// STATUS: ALPHA - Strategic Governance Active
// PROTOCOL: Asymmetric Information Arbitrage

use crate::psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
use crate::psl::error::PslError;
use tracing::{info, debug};

/// Predicate: Strategic Material Gain (Sovereign Reward)
/// IF Action(A) results in AssetIncrease(X) AND Risk(R) is Low THEN Reward is High.
pub struct MaterialGainPredicate { pub target_growth: f64 }

impl Axiom for MaterialGainPredicate {
    fn id(&self) -> &str { "Predicate:Material_Gain" }
    fn description(&self) -> &str { "Calculates the Sovereign Reward based on physical asset accumulation." }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Scalar { label, value } if label == "projected_growth" => {
                if *value >= self.target_growth {
                    Ok(AxiomVerdict::pass(self.id().to_string(), *value, "High Sovereign Reward projected".into()))
                } else {
                    Ok(AxiomVerdict::fail(self.id().to_string(), *value, "Insufficient material return".into()))
                }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 0.5, "Non-growth target".into())),
        }
    }
}

/// Predicate: Critical Node Exploitation
/// IF Node(N) is Central(C) AND Resistance(R) is Low THEN Node is Exploit_Target.
pub struct CriticalNodePredicate { pub centrality_threshold: f64 }

impl Axiom for CriticalNodePredicate {
    fn id(&self) -> &str { "Predicate:Critical_Node" }
    fn description(&self) -> &str { "Identifies high-leverage nodes in financial or social substrates." }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { source: _, fields } => {
                let mut centrality = 0.0;
                for (k, v) in fields {
                    if k == "centrality" { centrality = v.parse().unwrap_or(0.0); }
                }
                
                if centrality >= self.centrality_threshold {
                    Ok(AxiomVerdict::pass(self.id().to_string(), centrality, "CRITICAL NODE IDENTIFIED".into()))
                } else {
                    Ok(AxiomVerdict::fail(self.id().to_string(), centrality, "Peripheral node - ignore".into()))
                }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 0.5, "Static analysis".into())),
        }
    }
}
