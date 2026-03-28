// ============================================================
// PSL Coercion Detection — Probabilistic Duress Audit
// Section 1.I: "Continuously audit environmental and biometric
// telemetry for signs of coercion."
// ============================================================

use crate::debuglog;
use crate::psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
use crate::psl::error::PslError;

/// Telemetry markers for coercion detection.
pub struct CoercionMarkers {
    pub stress_jitter: f64,    // 0.0 to 1.0 (biometric jitter)
    pub pulse_bpm: f64,       // BPM
    pub geolocation_risk: f64, // 0.0 to 1.0 (proximity to threats)
}

/// The Coercion Detection Axiom.
pub struct CoercionAxiom {
    pub sensitivity: f64,
}

impl Axiom for CoercionAxiom {
    fn id(&self) -> &str { "Axiom:Coercion_Detection" }
    fn description(&self) -> &str { "Analyzes biometric and environmental telemetry for duress signals" }
    fn verify(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { source, fields } if source == "telemetry_sensors" => {
                let mut jitter = 0.0;
                let mut geo_risk = 0.0;
                
                for (k, v) in fields {
                    if k == "stress_jitter" { jitter = v.parse().unwrap_or(0.0); }
                    if k == "geo_risk" { geo_risk = v.parse().unwrap_or(0.0); }
                }

                // Probabilistic Coercion P(C)
                let p_c = (jitter * 0.7) + (geo_risk * 0.3);
                
                if p_c > self.sensitivity {
                    debuglog!("PSL: COERCION DETECTED (P(C)={:.4})", p_c);
                    Ok(AxiomVerdict::fail(self.id().to_string(), p_c, "Duress signal detected".into()))
                } else {
                    Ok(AxiomVerdict::pass(self.id().to_string(), 1.0 - p_c, "Safe".into()))
                }
            },
            _ => Err(PslError::InvalidAuditTarget { reason: "Telemetry payload required".into() })
        }
    }
}
