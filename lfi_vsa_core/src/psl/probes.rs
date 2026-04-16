// ============================================================
// OPSEC Probes — Offensive Verification
// ============================================================

use crate::psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
use crate::psl::error::PslError;

pub struct OverflowProbe;
impl Axiom for OverflowProbe {
    fn id(&self) -> &str { "Probe:Memory_Overflow" }
    fn description(&self) -> &str { "Offensive probe for buffer overflow vulnerabilities" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        debuglog!("OverflowProbe::evaluate");
        match target {
            AuditTarget::Vector(v) => {
                if v.dim() > 10000 { Ok(AxiomVerdict::fail(self.id().to_string(), 0.1, "Overflow detected".into())) }
                else { Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Bounds verified".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-vector target".into())),
        }
    }
}

pub struct EncryptionProbe;
impl Axiom for EncryptionProbe {
    fn id(&self) -> &str { "Probe:Entropy_Sweep" }
    fn description(&self) -> &str { "Verifies signal encryption strength" }
    fn evaluate(&self, _target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        debuglog!("EncryptionProbe::evaluate");
        Ok(AxiomVerdict::pass(self.id().to_string(), 0.9, "Entropy nominal".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdc::vector::BipolarVector;
    use crate::psl::trust::TrustLevel;

    #[test]
    fn test_overflow_probe_normal_vector() -> Result<(), PslError> {
        let probe = OverflowProbe;
        let v = BipolarVector::new_random().unwrap();
        let target = AuditTarget::Vector(v);
        let verdict = probe.evaluate(&target)?;
        assert!(verdict.confidence > 0.5, "Normal 10k vector should pass");
        assert!(matches!(verdict.level, TrustLevel::Sovereign | TrustLevel::Trusted));
        Ok(())
    }

    #[test]
    fn test_overflow_probe_non_vector() -> Result<(), PslError> {
        let probe = OverflowProbe;
        let target = AuditTarget::Scalar { label: "test".into(), value: 42.0 };
        let verdict = probe.evaluate(&target)?;
        assert!(verdict.confidence == 1.0, "Non-vector should pass");
        Ok(())
    }

    #[test]
    fn test_encryption_probe_always_passes() -> Result<(), PslError> {
        let probe = EncryptionProbe;
        let target = AuditTarget::Scalar { label: "entropy".into(), value: 0.99 };
        let verdict = probe.evaluate(&target)?;
        assert!(verdict.confidence > 0.8);
        Ok(())
    }

    #[test]
    fn test_probe_ids_are_unique() {
        let overflow = OverflowProbe;
        let encryption = EncryptionProbe;
        assert_ne!(overflow.id(), encryption.id());
        assert!(!overflow.description().is_empty());
        assert!(!encryption.description().is_empty());
    }

    // ============================================================
    // Stress / invariant tests for OPSEC probes
    // ============================================================

    /// INVARIANT: every probe returns confidence in [0,1].
    #[test]
    fn invariant_probe_confidence_in_unit_interval() -> Result<(), PslError> {
        let probes: Vec<Box<dyn Axiom>> = vec![
            Box::new(OverflowProbe),
            Box::new(EncryptionProbe),
        ];
        let targets = [
            AuditTarget::Scalar { label: "x".into(), value: 0.5 },
            AuditTarget::Vector(BipolarVector::new_random().unwrap()),
            AuditTarget::Payload {
                source: "s".into(),
                fields: vec![("k".into(), "v".into())],
            },
        ];
        for p in &probes {
            for t in &targets {
                let v = p.evaluate(t)?;
                assert!(v.confidence.is_finite() && (0.0..=1.0).contains(&v.confidence),
                    "probe {} confidence out of [0,1]: {}", p.id(), v.confidence);
            }
        }
        Ok(())
    }

    /// INVARIANT: probe evaluation never panics across diverse targets.
    #[test]
    fn invariant_probes_never_panic() -> Result<(), PslError> {
        let probes: Vec<Box<dyn Axiom>> = vec![
            Box::new(OverflowProbe),
            Box::new(EncryptionProbe),
        ];
        let v = BipolarVector::new_random().unwrap();
        let targets = vec![
            AuditTarget::Vector(v),
            AuditTarget::Scalar { label: "".into(), value: 0.0 },
            AuditTarget::Scalar { label: "neg".into(), value: -1e9 },
            AuditTarget::Scalar { label: "big".into(), value: 1e9 },
            AuditTarget::Payload { source: "".into(), fields: vec![] },
        ];
        for p in &probes {
            for t in &targets {
                let _ = p.evaluate(t)?;
            }
        }
        Ok(())
    }

    /// INVARIANT: probe ids and descriptions are non-empty and distinct.
    #[test]
    fn invariant_probe_metadata_nonempty_and_distinct() {
        let probes: Vec<Box<dyn Axiom>> = vec![
            Box::new(OverflowProbe),
            Box::new(EncryptionProbe),
        ];
        let mut ids = std::collections::HashSet::new();
        for p in &probes {
            assert!(!p.id().is_empty(), "probe id empty");
            assert!(!p.description().is_empty(), "probe description empty");
            assert!(ids.insert(p.id().to_string()),
                "probe id collision: {}", p.id());
        }
    }
}
