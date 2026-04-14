// NODE 018: NeuPSL World-State Predicates
// STATUS: ALPHA - Strategic Governance Active
// PROTOCOL: Asymmetric Information Arbitrage

use crate::psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
use crate::psl::error::PslError;

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

/// Privacy compliance predicate: enforces data minimization and
/// purpose limitation principles. Detects when more data is being
/// processed than necessary for the stated purpose.
pub struct PrivacyCompliancePredicate {
    /// Maximum fields allowed in a single operation.
    pub max_fields: usize,
    /// Sensitive field names that require elevated justification.
    pub sensitive_fields: Vec<String>,
}

impl Default for PrivacyCompliancePredicate {
    fn default() -> Self {
        Self {
            max_fields: 50,
            sensitive_fields: vec![
                "ssn".into(), "password".into(), "credit_card".into(),
                "dob".into(), "address".into(), "phone".into(),
                "email".into(), "biometric".into(), "medical".into(),
                "income".into(), "political".into(), "religion".into(),
            ],
        }
    }
}

impl Axiom for PrivacyCompliancePredicate {
    fn id(&self) -> &str { "Predicate:Privacy_Compliance" }
    fn description(&self) -> &str { "Enforces data minimization and sensitive field protection" }

    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { fields, .. } => {
                // Data minimization: reject excessive field counts
                if fields.len() > self.max_fields {
                    return Ok(AxiomVerdict::fail(
                        self.id().into(), 0.2,
                        format!("Data minimization violation: {} fields exceeds limit {}", fields.len(), self.max_fields),
                    ));
                }

                // Sensitive field detection
                let mut sensitive_count = 0;
                for (key, _) in fields {
                    let lower_key = key.to_lowercase();
                    if self.sensitive_fields.iter().any(|s| lower_key.contains(s)) {
                        sensitive_count += 1;
                    }
                }

                if sensitive_count > 3 {
                    Ok(AxiomVerdict::fail(
                        self.id().into(), 0.15,
                        format!("Excessive sensitive data: {} sensitive fields in single operation", sensitive_count),
                    ))
                } else if sensitive_count > 0 {
                    Ok(AxiomVerdict::pass(
                        self.id().into(),
                        1.0 - (sensitive_count as f64 * 0.1),
                        format!("Privacy caution: {} sensitive field(s) present", sensitive_count),
                    ))
                } else {
                    Ok(AxiomVerdict::pass(self.id().into(), 1.0, "No sensitive data detected".into()))
                }
            }
            _ => Ok(AxiomVerdict::pass(self.id().into(), 1.0, "Non-payload target".into())),
        }
    }

    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Payload { .. }) { 1.0 } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_gain_high_growth() -> Result<(), PslError> {
        let pred = MaterialGainPredicate { target_growth: 0.5 };
        let target = AuditTarget::Scalar { label: "projected_growth".into(), value: 0.8 };
        let verdict = pred.evaluate(&target)?;
        assert!(verdict.confidence >= 0.8, "High growth should pass");
        Ok(())
    }

    #[test]
    fn test_material_gain_low_growth() -> Result<(), PslError> {
        let pred = MaterialGainPredicate { target_growth: 0.5 };
        let target = AuditTarget::Scalar { label: "projected_growth".into(), value: 0.2 };
        let verdict = pred.evaluate(&target)?;
        assert!(verdict.confidence < 0.5, "Low growth should fail");
        Ok(())
    }

    #[test]
    fn test_material_gain_non_growth_target() -> Result<(), PslError> {
        let pred = MaterialGainPredicate { target_growth: 0.5 };
        let target = AuditTarget::Scalar { label: "temperature".into(), value: 42.0 };
        let verdict = pred.evaluate(&target)?;
        assert!((verdict.confidence - 0.5).abs() < 0.01, "Non-growth target should default to 0.5");
        Ok(())
    }

    #[test]
    fn test_critical_node_above_threshold() -> Result<(), PslError> {
        let pred = CriticalNodePredicate { centrality_threshold: 0.7 };
        let target = AuditTarget::Payload {
            source: "network_analysis".into(),
            fields: vec![("centrality".into(), "0.9".into())],
        };
        let verdict = pred.evaluate(&target)?;
        assert!(verdict.confidence > 0.7);
        assert!(verdict.detail.contains("CRITICAL"));
        Ok(())
    }

    #[test]
    fn test_critical_node_below_threshold() -> Result<(), PslError> {
        let pred = CriticalNodePredicate { centrality_threshold: 0.7 };
        let target = AuditTarget::Payload {
            source: "network_analysis".into(),
            fields: vec![("centrality".into(), "0.3".into())],
        };
        let verdict = pred.evaluate(&target)?;
        assert!(verdict.detail.contains("Peripheral"));
        Ok(())
    }

    #[test]
    fn test_predicate_ids_unique() {
        let mg = MaterialGainPredicate { target_growth: 0.5 };
        let cn = CriticalNodePredicate { centrality_threshold: 0.7 };
        let pc = PrivacyCompliancePredicate::default();
        assert_ne!(mg.id(), cn.id());
        assert_ne!(mg.id(), pc.id());
        assert_ne!(cn.id(), pc.id());
    }

    #[test]
    fn test_privacy_clean_data() -> Result<(), PslError> {
        let pred = PrivacyCompliancePredicate::default();
        let target = AuditTarget::Payload {
            source: "app".into(),
            fields: vec![("username".into(), "john".into()), ("preference".into(), "dark_mode".into())],
        };
        let verdict = pred.evaluate(&target)?;
        assert!(verdict.confidence > 0.9, "Clean data should pass: {:.4}", verdict.confidence);
        Ok(())
    }

    #[test]
    fn test_privacy_sensitive_fields_detected() -> Result<(), PslError> {
        let pred = PrivacyCompliancePredicate::default();
        let target = AuditTarget::Payload {
            source: "form".into(),
            fields: vec![
                ("user_email".into(), "test@example.com".into()),
                ("user_phone".into(), "555-1234".into()),
            ],
        };
        let verdict = pred.evaluate(&target)?;
        // Should pass but with reduced confidence (2 sensitive fields)
        assert!(verdict.confidence < 1.0 && verdict.confidence > 0.5,
            "Sensitive fields should reduce confidence: {:.4}", verdict.confidence);
        Ok(())
    }

    #[test]
    fn test_privacy_excessive_sensitive_blocked() -> Result<(), PslError> {
        let pred = PrivacyCompliancePredicate::default();
        let target = AuditTarget::Payload {
            source: "form".into(),
            fields: vec![
                ("ssn".into(), "123-45-6789".into()),
                ("credit_card".into(), "4111-1111-1111-1111".into()),
                ("medical".into(), "diagnosis: X".into()),
                ("income".into(), "50000".into()),
            ],
        };
        let verdict = pred.evaluate(&target)?;
        assert!(!verdict.level.permits_execution(),
            "4 sensitive fields should be blocked: {:?}", verdict);
        Ok(())
    }

    #[test]
    fn test_privacy_data_minimization() -> Result<(), PslError> {
        let pred = PrivacyCompliancePredicate { max_fields: 3, ..Default::default() };
        let target = AuditTarget::Payload {
            source: "bulk".into(),
            fields: vec![
                ("a".into(), "1".into()), ("b".into(), "2".into()),
                ("c".into(), "3".into()), ("d".into(), "4".into()),
            ],
        };
        let verdict = pred.evaluate(&target)?;
        assert!(!verdict.level.permits_execution(),
            "4 fields > max 3 should fail data minimization");
        Ok(())
    }
}
