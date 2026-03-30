// ============================================================
// LFI VSA Core — Sovereign Crate Root
// Section 5: Absolute Memory Safety enforced via forbid(unsafe_code).
// ============================================================

#![forbid(unsafe_code)]

#[macro_export]
macro_rules! debuglog {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        println!($($arg)*);
    };
}

pub mod api;
pub mod coder;
pub mod cognition;
pub mod hdc;
pub mod hdlm;
pub mod intelligence;
pub mod languages;
pub mod psl;
pub mod transducers;
pub mod agent;
pub mod hid;
pub mod hmas;
pub mod identity;
pub mod laws;
pub mod telemetry;
pub mod memory_bus;
pub mod inference_engine;
pub mod data_ingestor;
pub mod qos;
pub mod crypto_epistemology;

// Re-export core public types
pub use hdc::vector::BipolarVector;
pub use hdc::compute::{ComputeBackend, LocalBackend};
pub use hdc::liquid::{LiquidSensorium, LiquidNeuron};
pub use psl::supervisor::PslSupervisor;
pub use psl::trust::TrustLevel;
pub use psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
pub use hdlm::ast::{Ast, AstNode, NodeKind};
pub use hdlm::codebook::HdlmCodebook;
pub use intelligence::{OsintAnalyzer, OsintSignal};
pub use laws::{PrimaryLaw, SovereignConstraint};
pub use identity::{IdentityProver, SovereignProof};
pub use hid::{HidDevice, HidCommand};
pub use agent::LfiAgent;
pub use hmas::{MicroSupervisor, AgentRole, AgentTemplate};
