// ============================================================
// Web Audit Layer — Skeptical Ingestion
// ============================================================

use crate::psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
use crate::psl::error::PslError;

pub struct ConnectivityAxiom { pub required_tunnel: String }
impl Axiom for ConnectivityAxiom {
    fn id(&self) -> &str { "Axiom:Secure_Ingress" }
    fn description(&self) -> &str { "Ensures web ingress occurs over secure tunnel" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { fields, .. } => {
                let mut tunnel_active = false;
                for (k, v) in fields {
                    if k == "tunnel" && v == &self.required_tunnel {
                        tunnel_active = true;
                    }
                }

                if tunnel_active {
                    Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Secure tunnel verified".into()))
                } else {
                    Ok(AxiomVerdict::fail(self.id().to_string(), 0.1, "Insecure ingress path".into()))
                }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Internal target".into())),
        }
    }
}
