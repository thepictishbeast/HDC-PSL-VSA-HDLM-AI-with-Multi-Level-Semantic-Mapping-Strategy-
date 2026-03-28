// ============================================================
// PSL (Policy-Specification Language) — The Auditor
// Section 1.II: "The Forensic Supervisor engine verifies all
// VSA outputs, external GPU returns, and file ingestions."
// ============================================================

pub mod axiom;
pub mod supervisor;
pub mod trust;
pub mod error;
pub mod probes;
pub mod coercion;

pub use axiom::{Axiom, AuditTarget, AxiomVerdict, WebSearchSkepticismAxiom, ForbiddenSpaceAxiom};
pub use supervisor::PslSupervisor;
pub use trust::{TrustLevel, TrustAssessment};
pub use error::PslError;
pub use probes::{OverflowProbe, EncryptionProbe};
pub use coercion::CoercionAxiom;
