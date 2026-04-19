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

/// SECURITY: Sanitize a string for safe inclusion in log output.
/// Strips control characters (newlines, ANSI escapes, null bytes) that could
/// corrupt log formatting or enable log injection attacks, then truncates.
pub fn sanitize_for_log(s: &str, max_chars: usize) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .take(max_chars)
        .collect()
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
pub mod sealed;
pub mod ingest;
pub mod formal;
pub mod stats_cache;

// Re-export core public types
pub use sealed::{Sealed, Sensitive, SecretBroker};
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========== truncate_str ==========

    #[test]
    fn truncate_str_within_limit() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn truncate_str_at_limit() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncate_str_above_limit() {
        assert_eq!(truncate_str("hello world", 5), "hello");
    }

    #[test]
    fn truncate_str_empty() {
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn truncate_str_zero_limit() {
        assert_eq!(truncate_str("hello", 0), "");
    }

    #[test]
    fn truncate_str_unicode_safe() {
        // "αβγδ" is 4 chars but 8 bytes — truncating at char boundary
        let s = "αβγδ";
        let t = truncate_str(s, 2);
        assert_eq!(t, "αβ");
        assert_eq!(t.len(), 4); // 2 UTF-8 chars × 2 bytes each
    }

    #[test]
    fn truncate_str_emoji_safe() {
        let s = "🦀🔥💯";
        assert_eq!(truncate_str(s, 1), "🦀");
        assert_eq!(truncate_str(s, 2), "🦀🔥");
    }

    // ========== sanitize_for_log ==========

    #[test]
    fn sanitize_strips_newlines() {
        assert_eq!(sanitize_for_log("hello\nworld", 50), "helloworld");
    }

    #[test]
    fn sanitize_strips_null_bytes() {
        assert_eq!(sanitize_for_log("hello\0world", 50), "helloworld");
    }

    #[test]
    fn sanitize_strips_ansi_escape() {
        assert_eq!(sanitize_for_log("hello\x1b[31mred\x1b[0m", 50), "hello[31mred[0m");
    }

    #[test]
    fn sanitize_preserves_spaces() {
        assert_eq!(sanitize_for_log("hello world", 50), "hello world");
    }

    #[test]
    fn sanitize_truncates() {
        assert_eq!(sanitize_for_log("hello world", 5), "hello");
    }

    #[test]
    fn sanitize_empty() {
        assert_eq!(sanitize_for_log("", 100), "");
    }

    #[test]
    fn sanitize_tabs_stripped() {
        assert_eq!(sanitize_for_log("col1\tcol2", 50), "col1col2");
    }
}
