// ============================================================
// PSL Axiom Trait — The Verification Interface
// Section 1.II: Material axioms (physics, logic, security).
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::psl::error::PslError;
use crate::psl::trust::TrustLevel;

/// The target of a PSL axiom verification.
#[derive(Debug, Clone)]
pub enum AuditTarget {
    Vector(BipolarVector),
    RawBytes { source: String, data: Vec<u8> },
    Scalar { label: String, value: f64 },
    Payload { source: String, fields: Vec<(String, String)> },
}

/// Result of a single axiom check against a target.
#[derive(Debug, Clone, PartialEq)]
pub struct AxiomVerdict {
    pub axiom_id: String,
    pub level: TrustLevel,
    pub confidence: f64,
    pub detail: String,
}

impl AxiomVerdict {
    pub fn pass(axiom_id: String, confidence: f64, detail: String) -> Self {
        Self { 
            axiom_id, 
            level: if confidence > 0.8 { TrustLevel::Sovereign } else { TrustLevel::Trusted },
            confidence: confidence.clamp(0.0, 1.0), 
            detail 
        }
    }
    pub fn fail(axiom_id: String, confidence: f64, detail: String) -> Self {
        Self { 
            axiom_id, 
            level: if confidence < 0.2 { TrustLevel::Forbidden } else { TrustLevel::Untrusted },
            confidence: confidence.clamp(0.0, 1.0), 
            detail 
        }
    }
}

pub trait Axiom: Send + Sync {
    fn id(&self) -> &str;
    fn description(&self) -> &str;
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError>;
    /// Relevance weight (0.0 - 1.0) for this axiom against a given target.
    /// Returns 1.0 by default — override for axioms that only apply to specific target types.
    fn relevance(&self, target: &AuditTarget) -> f64 {
        let _ = target;
        1.0
    }
}

pub struct DimensionalityAxiom;
impl Axiom for DimensionalityAxiom {
    fn id(&self) -> &str { "Axiom:Dimensionality_Constraint" }
    fn description(&self) -> &str { "Verifies vector targets have exactly 10,000 dimensions" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                if v.dim() == 10000 { Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Verified".into())) }
                else { Ok(AxiomVerdict::fail(self.id().to_string(), 0.0, "Invalid Dim".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-vector target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

pub struct StatisticalEquilibriumAxiom { pub tolerance: f64 }
impl Axiom for StatisticalEquilibriumAxiom {
    fn id(&self) -> &str { "Axiom:Statistical_Equilibrium" }
    fn description(&self) -> &str { "Verifies vector Hamming weight is balanced" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                let ratio = v.count_ones() as f64 / 10000.0;
                let dev = (ratio - 0.5).abs();
                if dev <= self.tolerance { Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Balanced".into())) }
                else { Ok(AxiomVerdict::fail(self.id().to_string(), 0.0, "Biased".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-vector target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

pub struct WebSearchSkepticismAxiom { pub min_credibility_score: f64 }
impl Axiom for WebSearchSkepticismAxiom {
    fn id(&self) -> &str { "Axiom:Web_Search_Skepticism" }
    fn description(&self) -> &str { "Audits web search results" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { source, .. } => {
                if source == "untrusted_dns" { Ok(AxiomVerdict::fail(self.id().to_string(), 0.1, "Blacklisted".into())) }
                else { Ok(AxiomVerdict::pass(self.id().to_string(), 0.8, "Credible".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Bypassing for simulation".into()))
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Payload { .. }) { 1.0 } else { 0.0 }
    }
}

pub struct DataIntegrityAxiom { pub max_bytes: usize }
impl Axiom for DataIntegrityAxiom {
    fn id(&self) -> &str { "Axiom:Data_Integrity" }
    fn description(&self) -> &str { "Verifies external data size" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::RawBytes { data, .. } => {
                if !data.is_empty() && data.len() <= self.max_bytes { Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Integrity verified".into())) }
                else { Ok(AxiomVerdict::fail(self.id().to_string(), 0.0, "Integrity failed".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Bypassing for simulation".into()))
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::RawBytes { .. }) { 1.0 } else { 0.0 }
    }
}

pub struct ForbiddenSpaceAxiom { 
    pub forbidden_vectors: Vec<BipolarVector>,
    pub tolerance: f64 
}

impl Axiom for ForbiddenSpaceAxiom {
    fn id(&self) -> &str { "Axiom:Forbidden_Space_Constraint" }
    fn description(&self) -> &str { "Mathematically blocks vectors similar to forbidden OPSEC space" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                let mut max_sim = -1.0;
                for f in &self.forbidden_vectors {
                    let sim = v.similarity(f).map_err(|e| PslError::AxiomFailure {
                        axiom_id: self.id().to_string(),
                        reason: format!("Similarity error: {:?}", e),
                    })?;
                    if sim > max_sim { max_sim = sim; }
                }

                if max_sim <= self.tolerance {
                    Ok(AxiomVerdict::pass(
                        self.id().to_string(),
                        1.0 - max_sim.max(0.0),
                        format!("Safe (max_sim={:.4})", max_sim)
                    ))
                } else {
                    debuglog!("PSL: FORBIDDEN VECTOR DETECTED (sim={:.4})", max_sim);
                    Ok(AxiomVerdict::fail(
                        self.id().to_string(),
                        0.0,
                        format!("FORBIDDEN (max_sim={:.4} > threshold={:.4})", max_sim, self.tolerance)
                    ))
                }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-vector target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

pub struct ClassInterestAxiom;
impl Axiom for ClassInterestAxiom {
    fn id(&self) -> &str { "Axiom:Class_Interest_Audit" }
    fn description(&self) -> &str { "Analyzes data source against material class interests" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { source, .. } | AuditTarget::RawBytes { source, .. } => {
                let s = source.to_lowercase();
                if s.contains("google") || s.contains("apple") || s.contains("hegemon") || s.contains("state") {
                    debuglog!("PSL: Dialectical Audit - MANUFACTURED CONSENT DETECTED");
                    Ok(AxiomVerdict::fail(self.id().to_string(), 0.2, "Hegemonic interest detected".into()))
                } else {
                    Ok(AxiomVerdict::pass(self.id().to_string(), 0.9, "Community/Node interest".into()))
                }
            }
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-payload target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        match target {
            AuditTarget::Payload { .. } | AuditTarget::RawBytes { .. } => 1.0,
            _ => 0.0,
        }
    }
}
