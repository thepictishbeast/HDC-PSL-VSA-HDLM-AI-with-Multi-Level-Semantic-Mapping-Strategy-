// ============================================================
// LFI VSA Core — Sovereign Crate Root
// Section 5: Absolute Memory Safety enforced via forbid(unsafe_code).
// ============================================================

#![forbid(unsafe_code)]

#[macro_export]
macro_rules! debuglog {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

/// UTF-8 safe string truncation. Truncates at a char boundary, never panics.
/// SUPERSOCIETY: Every string slice in the codebase must use this instead of
/// byte-level `&s[..n]` which panics on multi-byte UTF-8 characters.
pub fn truncate_str(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
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
pub mod reasoning_provenance;
pub mod diag;
pub mod data_ingestion;
pub mod data_quality;
pub mod persistence;

// Re-export core public types
pub use hdc::vector::BipolarVector;
pub use hdc::compute::{ComputeBackend, LocalBackend};
pub use hdc::liquid::{LiquidSensorium, LiquidNeuron};
pub use psl::supervisor::PslSupervisor;
pub use psl::trust::TrustLevel;
pub use psl::axiom::{Axiom, AuditTarget, AxiomVerdict};
pub use hdlm::ast::{Ast, AstNode, NodeKind};
pub use hdlm::codebook::{HdlmCodebook, CodebookMode};
pub use intelligence::{OsintAnalyzer, OsintSignal};
pub use hdc::hadamard::{HadamardGenerator, CorrelatedGenerator};
pub use cognition::metacognitive::{MetaCognitiveProfiler, CognitiveDomain};
pub use cognition::knowledge_compiler::{KnowledgeCompiler, AccelerationMetrics};
pub use psl::feedback::{PslFeedbackLoop, AvoidanceCheck};
pub use laws::{PrimaryLaw, SovereignConstraint};
pub use identity::{IdentityProver, SovereignProof};
pub use hid::{HidDevice, HidCommand};
pub use agent::LfiAgent;
pub use hmas::{MicroSupervisor, AgentRole, AgentTemplate};
pub use reasoning_provenance::{
    ProvenanceEngine, ProvenanceKind, ProvenancedExplanation,
    TraceArena, TraceEntry, TraceId, ConclusionId, InferenceSource,
};
pub mod crypto_commitment;
pub mod mesh;
