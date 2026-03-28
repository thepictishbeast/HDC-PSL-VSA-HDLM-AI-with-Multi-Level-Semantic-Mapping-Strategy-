#![forbid(unsafe_code)]

// ============================================================
// LFI VSA Core — Sovereign Crate Root
// Section 5: Absolute Memory Safety enforced via forbid(unsafe_code).
// ============================================================

pub mod telemetry;
pub mod hdc;
pub mod psl;
pub mod hdlm;
pub mod hid;
pub mod agent;
pub mod hmas;
pub mod api;
pub mod transducers;
pub mod languages;
pub mod coder;
pub mod cognition;
pub mod laws;
pub mod identity;
pub mod intelligence;

// --------------------------------------------------------
// Re-export core public types for ergonomic access.
// --------------------------------------------------------

// I. HDC & Liquid Core
pub use hdc::error::HdcError;
pub use hdc::vector::{BipolarVector, HD_DIMENSIONS};
pub use hdc::compute::{ComputeBackend, LocalBackend};
pub use hdc::liquid::{LiquidSensorium, LiquidNeuron};

// II. PSL Supervisor (The Auditor)
pub use psl::supervisor::PslSupervisor;
pub use psl::trust::{TrustLevel, TrustAssessment};
pub use psl::axiom::{Axiom, AuditTarget, AxiomVerdict};

// III. HDLM (Discretization & Mapping)
pub use hdlm::ast::{Ast, AstNode, NodeKind};
pub use hdlm::codebook::HdlmCodebook;

// IV. Intelligence & OSINT
pub use intelligence::{OsintAnalyzer, OsintSignal};

// V. Laws & Identity (The Sovereign Constraints)
pub use laws::{PrimaryLaw, SovereignConstraint};
pub use identity::{IdentityProver, SovereignProof};

// V. Unified Sensorium & Interaction
pub use hid::{HidDevice, HidCommand};
pub use agent::LfiAgent;
pub use hmas::{MicroSupervisor, AgentRole, AgentTemplate};
