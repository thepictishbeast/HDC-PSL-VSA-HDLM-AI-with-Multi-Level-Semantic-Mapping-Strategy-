// ============================================================
// Intelligence / OSINT Module — The Sensory Peripheral
// Section 1.IV: "Implement Intelligence/OSINT modules for analysis."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::psl::supervisor::PslSupervisor;
use crate::psl::axiom::AuditTarget;
use crate::identity::IdentityProver;

/// A piece of intelligence gathered from OSINT.
#[derive(Debug, Clone)]
pub struct OsintSignal {
    pub source: String,
    pub payload: String,
    pub metadata: Vec<(String, String)>,
}

/// The Intelligence Analyzer engine.
pub struct OsintAnalyzer {
    pub supervisor: PslSupervisor,
}

impl OsintAnalyzer {
    pub fn new() -> Self {
        debuglog!("OsintAnalyzer::new: Initializing OSINT audit supervisor");
        let supervisor = PslSupervisor::new();
        // Axioms will be inherited or registered here
        Self { supervisor }
    }

    /// Audits an external signal against the threat matrix.
    pub fn analyze_signal(&self, signal: &OsintSignal) -> Result<f64, String> {
        debuglog!("OsintAnalyzer: Analyzing signal from {}", signal.source);
        
        // 1. Vectorization of the signal
        let signal_hash = IdentityProver::hash(&signal.payload);
        let signal_vector = BipolarVector::from_seed(signal_hash);

        // 2. PSL Audit
        let target = AuditTarget::Vector(signal_vector);
        let assessment = self.supervisor.audit(&target).map_err(|e| format!("Audit failed: {:?}", e))?;

        if !assessment.level.permits_execution() {
            debuglog!("OsintAnalyzer: SIGNAL REJECTED (Level={:?})", assessment.level);
            return Err("Signal failed forensic trust audit".to_string());
        }

        debuglog!("OsintAnalyzer: Signal verified. Trust Confidence = {:.4}", assessment.confidence);
        Ok(assessment.confidence)
    }

    /// Perform a simulated CARTA risk assessment on a set of signals.
    pub fn assess_risk(&self, signals: &[OsintSignal]) -> f64 {
        let mut total_risk = 0.0;
        for s in signals {
            match self.analyze_signal(s) {
                Ok(conf) => total_risk += 1.0 - conf,
                Err(_) => total_risk += 1.0, // High risk if audit fails
            }
        }
        total_risk / (signals.len() as f64).max(1.0)
    }
}
