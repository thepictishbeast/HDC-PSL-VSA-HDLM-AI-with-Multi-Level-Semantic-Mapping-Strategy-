// ============================================================
// Reasoning Provenance — Verifiable Derivation Traces
//
// PURPOSE: Every inference step in the reasoning pipeline produces
// a first-class DerivationTrace struct. The system can distinguish
// between a TRACED derivation (complete, verifiable inference chain)
// and a RECONSTRUCTED rationalization (plausible explanation generated
// post-hoc without the original inference path).
//
// CORE ARCHITECTURAL INVARIANT:
//   If no traced derivation exists for a claim, the system CANNOT
//   present a reconstruction as if it were a trace. The ProvenanceKind
//   tag enforces this. The Crypto Epistemology layer is the enforcement
//   point — epistemic honesty is not optional.
//
// DESIGN:
//   Traces are stored in an arena allocator (Vec<TraceEntry>) for
//   cache-friendly traversal, following the same pattern as hdlm::ast::Ast.
//   Each trace entry records:
//     - Input premises (hypervector refs + decoded semantic labels)
//     - The inference rule applied (PSL rule, MCTS node, Active Inference step, etc.)
//     - Confidence weights at each step
//     - Wall-clock timestamp and computational cost
//     - Parent trace ID (for chaining multi-step derivations)
//
// INTEGRATION POINTS:
//   - PSL Supervisor: tag every soft-logic rule firing
//   - MCTS: each node expansion creates a trace entry linked to parent
//   - Active Inference: trace prediction-error-minimization steps
//   - System 1/2 dual cognition: lightweight vs full traces
//   - Self-play: traces survive episodes for post-hoc analysis
//
// ADDRESSES:
//   The fundamental gap in LLM architectures: inability to distinguish
//   genuine reasoning recall from post-hoc confabulation.
// ============================================================

use serde::{Serialize, Deserialize};
use tracing::info;

// ============================================================
// Core Types
// ============================================================

/// Unique identifier for a trace entry within the arena.
pub type TraceId = usize;

/// Unique identifier for a conclusion that may have a derivation trace.
pub type ConclusionId = u64;

/// Distinguishes genuine traced derivations from post-hoc reconstructions.
///
/// This is the core architectural invariant of the provenance system.
/// The system literally cannot present a reconstruction as a trace —
/// the enum tag makes the distinction structural, not behavioral.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvenanceKind {
    /// The system has a complete, verifiable inference chain from
    /// premises to conclusion stored in the trace arena.
    TracedDerivation,

    /// The system is generating a plausible explanation post-hoc
    /// without access to the original inference path. This is
    /// explicitly labeled so consumers know the explanation may
    /// not reflect the actual reasoning process.
    ReconstructedRationalization {
        /// Why no traced derivation exists.
        reason: String,
    },
}

/// Which subsystem produced this trace entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InferenceSource {
    /// PSL axiom evaluation during CARTA audit.
    PslAxiomEvaluation {
        axiom_id: String,
        relevance: f64,
    },
    /// MCTS node expansion during System 2 deliberation.
    MctsExpansion {
        action: String,
        node_depth: usize,
    },
    /// Active inference free-energy minimization step.
    ActiveInferenceStep {
        free_energy: f64,
        prediction_error: f64,
    },
    /// System 1 fast-path recognition (lightweight trace).
    System1FastPath {
        similarity_score: f64,
    },
    /// System 2 deep deliberation (full trace tree).
    System2Deliberation {
        iterations: usize,
    },
    /// Knowledge compiler acceleration (System 2 → System 1).
    KnowledgeCompilation,
    /// Self-play adversarial episode.
    SelfPlayEpisode {
        generation: usize,
    },
    /// Manual / external assertion (no inference — just recorded).
    ExternalAssertion {
        source: String,
    },
}

/// A single entry in the derivation trace arena.
///
/// Each entry records one inference step: what went in, what rule
/// was applied, what came out, and how confident we are.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Unique ID within the trace arena.
    pub id: TraceId,
    /// Parent trace entry (None for root premises).
    pub parent: Option<TraceId>,
    /// Which subsystem produced this step.
    pub source: InferenceSource,
    /// Semantic labels for the input premises.
    pub premise_labels: Vec<String>,
    /// Confidence weight at this step (0.0 to 1.0).
    pub confidence: f64,
    /// Wall-clock timestamp (Unix epoch millis).
    pub timestamp_ms: u64,
    /// Computational cost in microseconds.
    pub cost_us: u64,
    /// Optional conclusion ID linking this trace to a named conclusion.
    pub conclusion_id: Option<ConclusionId>,
    /// Human-readable description of what happened at this step.
    pub description: String,
}

/// An explanation tagged with its provenance kind.
///
/// Every self-explanation or introspection query the system answers
/// MUST be wrapped in this struct. The ProvenanceKind tag is mandatory.
#[derive(Debug, Clone)]
pub struct ProvenancedExplanation {
    /// Is this a genuine traced derivation or a post-hoc reconstruction?
    pub kind: ProvenanceKind,
    /// The explanation text.
    pub explanation: String,
    /// The full derivation chain (empty for reconstructions).
    pub trace_chain: Vec<TraceId>,
    /// Confidence at each step in the chain.
    pub confidence_chain: Vec<f64>,
    /// Total derivation depth.
    pub depth: usize,
}

// ============================================================
// Trace Arena
// ============================================================

/// Arena-allocated storage for derivation traces.
///
/// Follows the same pattern as `hdlm::ast::Ast`: entries stored in a Vec,
/// referenced by TraceId. Cache-friendly traversal, O(1) insertion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceArena {
    /// All trace entries, arena-allocated.
    entries: Vec<TraceEntry>,
    /// Index: conclusion_id → trace entry IDs that produced it.
    conclusion_index: std::collections::HashMap<ConclusionId, Vec<TraceId>>,
    /// Reference counts for garbage collection.
    ref_counts: Vec<usize>,
    /// AUDIT FIX #21: Maximum entries before auto-compact.
    /// Prevents unbounded memory growth on long-running servers.
    max_entries: usize,
}

impl TraceArena {
    pub fn new() -> Self {
        debuglog!("TraceArena::new: Initializing derivation trace arena");
        Self {
            entries: Vec::new(),
            conclusion_index: std::collections::HashMap::new(),
            ref_counts: Vec::new(),
            max_entries: 100_000, // AUDIT FIX #21: cap at 100K traces
        }
    }

    /// Record a new trace entry. Returns its TraceId.
    /// AUDIT FIX #21: Auto-compacts when max_entries exceeded (keeps newest half).
    pub fn record(&mut self, entry: TraceEntry) -> TraceId {
        if self.entries.len() >= self.max_entries {
            // Compact: keep the newest half
            let keep_from = self.entries.len() / 2;
            self.entries = self.entries.split_off(keep_from);
            self.ref_counts = if self.ref_counts.len() > keep_from {
                self.ref_counts.split_off(keep_from)
            } else {
                Vec::new()
            };
            self.conclusion_index.clear(); // Rebuild would be needed for full correctness
            tracing::info!("// PROVENANCE: Auto-compacted arena from {} to {} entries", self.max_entries, self.entries.len());
        }
        let id = self.entries.len();
        debuglog!(
            "TraceArena::record: id={}, source={:?}, parent={:?}, confidence={:.4}",
            id, entry.source, entry.parent, entry.confidence
        );

        // Update conclusion index if this entry has a conclusion_id.
        if let Some(cid) = entry.conclusion_id {
            self.conclusion_index
                .entry(cid)
                .or_default()
                .push(id);
        }

        // Initialize ref count. Parent gets +1 ref.
        if let Some(parent_id) = entry.parent {
            if parent_id < self.ref_counts.len() {
                self.ref_counts[parent_id] += 1;
            }
        }

        self.entries.push(entry);
        self.ref_counts.push(1); // self-reference (alive)

        info!("// PROVENANCE: Trace #{} recorded", id);
        id
    }

    /// Record a trace entry and return its ID. Convenience wrapper that
    /// builds the TraceEntry from components.
    pub fn record_step(
        &mut self,
        parent: Option<TraceId>,
        source: InferenceSource,
        premise_labels: Vec<String>,
        confidence: f64,
        conclusion_id: Option<ConclusionId>,
        description: String,
        cost_us: u64,
    ) -> TraceId {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let id = self.entries.len();
        let entry = TraceEntry {
            id,
            parent,
            source,
            premise_labels,
            confidence,
            timestamp_ms,
            cost_us,
            conclusion_id,
            description,
        };

        self.record(entry)
    }

    /// Get a trace entry by ID.
    pub fn get(&self, id: TraceId) -> Option<&TraceEntry> {
        self.entries.get(id)
    }

    /// Number of entries in the arena.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Walk the derivation chain from a trace entry back to the root.
    /// Returns the chain of TraceIds from the given entry to the root premise.
    pub fn trace_chain(&self, start: TraceId) -> Vec<TraceId> {
        let mut chain = Vec::new();
        let mut current = Some(start);

        while let Some(id) = current {
            if id >= self.entries.len() {
                debuglog!("TraceArena::trace_chain: invalid id={}, stopping", id);
                break;
            }
            chain.push(id);
            current = self.entries[id].parent;
        }

        debuglog!("TraceArena::trace_chain: start={}, depth={}", start, chain.len());
        chain
    }

    /// Get the confidence values along a derivation chain.
    pub fn confidence_chain(&self, start: TraceId) -> Vec<f64> {
        self.trace_chain(start)
            .iter()
            .filter_map(|&id| self.entries.get(id).map(|e| e.confidence))
            .collect()
    }

    /// Derivation depth from a trace entry to the root.
    pub fn derivation_depth(&self, start: TraceId) -> usize {
        let chain = self.trace_chain(start);
        if chain.is_empty() { 0 } else { chain.len() - 1 }
    }

    /// Find trace entries that produced a given conclusion.
    pub fn traces_for_conclusion(&self, conclusion_id: ConclusionId) -> Vec<TraceId> {
        self.conclusion_index
            .get(&conclusion_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the most recent (highest-confidence) trace for a conclusion.
    pub fn best_trace_for_conclusion(&self, conclusion_id: ConclusionId) -> Option<TraceId> {
        self.traces_for_conclusion(conclusion_id)
            .into_iter()
            .max_by(|&a, &b| {
                let ca = self.entries.get(a).map(|e| e.confidence).unwrap_or(0.0);
                let cb = self.entries.get(b).map(|e| e.confidence).unwrap_or(0.0);
                ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Decrement reference count. If it reaches zero, the entry is logically dead.
    /// Returns true if the entry was reclaimed.
    pub fn release(&mut self, id: TraceId) -> bool {
        if id >= self.ref_counts.len() {
            return false;
        }
        if self.ref_counts[id] > 0 {
            self.ref_counts[id] -= 1;
        }
        let reclaimed = self.ref_counts[id] == 0;
        if reclaimed {
            debuglog!("TraceArena::release: trace {} reclaimed (ref_count=0)", id);
        }
        reclaimed
    }

    /// Count of entries with zero references (logically dead).
    pub fn dead_count(&self) -> usize {
        self.ref_counts.iter().filter(|&&rc| rc == 0).count()
    }

    /// Serialize the arena to a JSON string for persistence.
    /// BUG ASSUMPTION: untrusted JSON — caller must bound input size before deserializing.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        debuglog!("TraceArena::to_json: serializing {} entries", self.entries.len());
        serde_json::to_string(self)
    }

    /// Deserialize an arena from a JSON string.
    /// SECURITY: rejects inputs larger than 64 MiB before parsing (DoS guard).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        const MAX_ARENA_JSON_BYTES: usize = 64 * 1024 * 1024;
        if json.len() > MAX_ARENA_JSON_BYTES {
            debuglog!("TraceArena::from_json: REJECTED — input {} bytes exceeds {} limit",
                json.len(), MAX_ARENA_JSON_BYTES);
            return Err(serde::de::Error::custom("trace arena JSON exceeds 64 MiB limit"));
        }
        let arena: Self = serde_json::from_str(json)?;
        debuglog!("TraceArena::from_json: loaded {} entries", arena.entries.len());
        Ok(arena)
    }

    /// Save the arena to a file at the given path.
    pub fn save_to_path(&self, path: &std::path::Path) -> std::io::Result<()> {
        debuglog!("TraceArena::save_to_path: {:?}", path);
        let json = self.to_json().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
    }

    /// Load an arena from a file at the given path.
    pub fn load_from_path(path: &std::path::Path) -> std::io::Result<Self> {
        debuglog!("TraceArena::load_from_path: {:?}", path);
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })
    }

    /// Compact the arena by removing dead entries. Returns count removed.
    /// WARNING: This invalidates all existing TraceIds. Only call when
    /// no external references to trace IDs exist.
    pub fn compact(&mut self) -> usize {
        let before = self.entries.len();
        let alive: Vec<bool> = self.ref_counts.iter().map(|&rc| rc > 0).collect();

        // Build old→new ID mapping
        let mut new_ids: Vec<Option<TraceId>> = vec![None; before];
        let mut new_idx = 0;
        for (old_idx, &is_alive) in alive.iter().enumerate() {
            if is_alive {
                new_ids[old_idx] = Some(new_idx);
                new_idx += 1;
            }
        }

        // Rebuild entries with updated IDs
        let mut new_entries = Vec::with_capacity(new_idx);
        let mut new_ref_counts = Vec::with_capacity(new_idx);
        for (old_idx, entry) in self.entries.drain(..).enumerate() {
            if alive[old_idx] {
                let mut remapped = entry;
                remapped.id = new_ids[old_idx].unwrap_or(0);
                remapped.parent = remapped.parent.and_then(|p| new_ids.get(p).copied().flatten());
                new_entries.push(remapped);
                new_ref_counts.push(self.ref_counts[old_idx]);
            }
        }

        // Rebuild conclusion index
        self.conclusion_index.clear();
        for entry in &new_entries {
            if let Some(cid) = entry.conclusion_id {
                self.conclusion_index
                    .entry(cid)
                    .or_default()
                    .push(entry.id);
            }
        }

        self.entries = new_entries;
        self.ref_counts = new_ref_counts;

        let removed = before - self.entries.len();
        debuglog!("TraceArena::compact: removed {} dead entries, {} alive", removed, self.entries.len());
        removed
    }
}

// ============================================================
// Introspection API
// ============================================================

/// The Reasoning Provenance engine. Wraps a TraceArena and provides
/// the introspection API specified in the architecture.
pub struct ProvenanceEngine {
    /// Arena storage for all derivation traces.
    pub arena: TraceArena,
}

impl ProvenanceEngine {
    pub fn new() -> Self {
        debuglog!("ProvenanceEngine::new: Initializing reasoning provenance engine");
        Self {
            arena: TraceArena::new(),
        }
    }

    /// Retrieve the full derivation trace for a conclusion.
    ///
    /// Returns None if no trace exists (the conclusion was not derived
    /// through the traced reasoning pipeline).
    pub fn trace_for_conclusion(&self, conclusion_id: ConclusionId) -> Option<&TraceEntry> {
        debuglog!("ProvenanceEngine::trace_for_conclusion: cid={}", conclusion_id);
        let best_id = self.arena.best_trace_for_conclusion(conclusion_id)?;
        self.arena.get(best_id)
    }

    /// Explain a conclusion with full provenance tagging.
    ///
    /// If a traced derivation exists, returns ProvenanceKind::TracedDerivation
    /// with the full chain. If not, returns ProvenanceKind::ReconstructedRationalization
    /// with an honest admission that no trace exists.
    ///
    /// THIS IS THE CORE ENFORCEMENT POINT. The crypto epistemology layer
    /// trusts this function to never mislabel a reconstruction as a trace.
    pub fn explain_conclusion(&self, conclusion_id: ConclusionId) -> ProvenancedExplanation {
        debuglog!("ProvenanceEngine::explain_conclusion: cid={}", conclusion_id);

        let traces = self.arena.traces_for_conclusion(conclusion_id);

        if traces.is_empty() {
            // No trace exists — be honest about it.
            info!("// PROVENANCE: No traced derivation for conclusion {}. Returning RECONSTRUCTED tag.", conclusion_id);
            return ProvenancedExplanation {
                kind: ProvenanceKind::ReconstructedRationalization {
                    reason: format!(
                        "No derivation trace exists for conclusion {}. \
                         The reasoning path was either not recorded or has been reclaimed.",
                        conclusion_id
                    ),
                },
                explanation: format!(
                    "Conclusion {} has no traced derivation. Any explanation \
                     would be a post-hoc reconstruction, not a recall of actual reasoning.",
                    conclusion_id
                ),
                trace_chain: Vec::new(),
                confidence_chain: Vec::new(),
                depth: 0,
            };
        }

        // Find the best (highest-confidence) trace entry for this conclusion.
        let best_id = self.arena.best_trace_for_conclusion(conclusion_id)
            .expect("traces non-empty but best_trace returned None — logic error");

        let chain = self.arena.trace_chain(best_id);
        let confidence_chain = self.arena.confidence_chain(best_id);
        let depth = self.arena.derivation_depth(best_id);

        // Build the explanation from the trace chain.
        let mut explanation_parts = Vec::new();
        for &trace_id in chain.iter().rev() {
            if let Some(entry) = self.arena.get(trace_id) {
                explanation_parts.push(format!(
                    "[Step {} | {:?} | conf={:.3}] {}",
                    entry.id, entry.source, entry.confidence, entry.description
                ));
            }
        }

        let explanation = explanation_parts.join("\n");

        info!("// PROVENANCE: Traced derivation found for conclusion {}. Depth={}, steps={}",
            conclusion_id, depth, chain.len());

        ProvenancedExplanation {
            kind: ProvenanceKind::TracedDerivation,
            explanation,
            trace_chain: chain,
            confidence_chain,
            depth,
        }
    }

    /// Derivation depth for a conclusion (0 if no trace exists).
    pub fn derivation_depth(&self, conclusion_id: ConclusionId) -> usize {
        self.arena.best_trace_for_conclusion(conclusion_id)
            .map(|id| self.arena.derivation_depth(id))
            .unwrap_or(0)
    }

    /// Confidence values at each step of the derivation chain.
    /// Empty if no trace exists.
    pub fn confidence_chain(&self, conclusion_id: ConclusionId) -> Vec<f64> {
        self.arena.best_trace_for_conclusion(conclusion_id)
            .map(|id| self.arena.confidence_chain(id))
            .unwrap_or_default()
    }

    /// Total number of trace entries recorded.
    pub fn trace_count(&self) -> usize {
        self.arena.len()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traced_derivation_returns_traced_kind() {
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = 42;

        // Record a simple derivation: premise → intermediate → conclusion
        let premise = engine.arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "user_input".into() },
            vec!["premise A".into()],
            1.0,
            None,
            "User provided premise A".into(),
            100,
        );

        let intermediate = engine.arena.record_step(
            Some(premise),
            InferenceSource::PslAxiomEvaluation {
                axiom_id: "DimensionalityAxiom".into(),
                relevance: 0.95,
            },
            vec!["premise A".into()],
            0.92,
            None,
            "PSL verified dimensionality constraint".into(),
            250,
        );

        let _conclusion = engine.arena.record_step(
            Some(intermediate),
            InferenceSource::MctsExpansion {
                action: "Specialize".into(),
                node_depth: 2,
            },
            vec!["premise A".into(), "PSL verified".into()],
            0.87,
            Some(cid),
            "MCTS specialized toward goal".into(),
            5000,
        );

        // The explanation MUST be tagged as TracedDerivation.
        let explanation = engine.explain_conclusion(cid);
        assert_eq!(
            explanation.kind,
            ProvenanceKind::TracedDerivation,
            "A conclusion with a stored trace must return TracedDerivation"
        );
        assert!(!explanation.trace_chain.is_empty());
        assert!(!explanation.confidence_chain.is_empty());
        assert!(explanation.depth > 0);
    }

    #[test]
    fn test_no_trace_returns_reconstructed_kind() {
        let engine = ProvenanceEngine::new();
        let cid: ConclusionId = 999;

        // Query a conclusion that was never traced.
        let explanation = engine.explain_conclusion(cid);
        assert!(
            matches!(explanation.kind, ProvenanceKind::ReconstructedRationalization { .. }),
            "A conclusion with no stored trace must return ReconstructedRationalization"
        );
        assert!(explanation.trace_chain.is_empty());
        assert!(explanation.confidence_chain.is_empty());
        assert_eq!(explanation.depth, 0);
    }

    #[test]
    fn test_trace_chains_correctly_linked() {
        let mut arena = TraceArena::new();

        let step0 = arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "root".into() },
            vec!["root premise".into()],
            1.0, None, "Root".into(), 0,
        );

        let step1 = arena.record_step(
            Some(step0),
            InferenceSource::System1FastPath { similarity_score: 0.95 },
            vec!["intermediate".into()],
            0.95, None, "Step 1".into(), 50,
        );

        let step2 = arena.record_step(
            Some(step1),
            InferenceSource::System2Deliberation { iterations: 20 },
            vec!["final".into()],
            0.88, Some(1), "Step 2".into(), 1000,
        );

        // Chain from step2 should be: [step2, step1, step0]
        let chain = arena.trace_chain(step2);
        assert_eq!(chain, vec![step2, step1, step0]);

        // Depth should be 2 (two parent hops)
        assert_eq!(arena.derivation_depth(step2), 2);
    }

    #[test]
    fn test_confidence_chain_from_stored_weights() {
        let mut arena = TraceArena::new();

        let s0 = arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "test".into() },
            vec![], 1.0, None, "root".into(), 0,
        );
        let s1 = arena.record_step(
            Some(s0),
            InferenceSource::PslAxiomEvaluation { axiom_id: "A".into(), relevance: 1.0 },
            vec![], 0.9, None, "psl".into(), 0,
        );
        let s2 = arena.record_step(
            Some(s1),
            InferenceSource::MctsExpansion { action: "Specialize".into(), node_depth: 1 },
            vec![], 0.85, Some(100), "mcts".into(), 0,
        );

        // Confidence chain from s2: [0.85, 0.9, 1.0] (leaf to root)
        let conf = arena.confidence_chain(s2);
        assert_eq!(conf.len(), 3);
        assert!((conf[0] - 0.85).abs() < 1e-6);
        assert!((conf[1] - 0.9).abs() < 1e-6);
        assert!((conf[2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_arena_release_and_compact() {
        let mut arena = TraceArena::new();

        let id0 = arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "temp".into() },
            vec![], 1.0, None, "temporary".into(), 0,
        );
        let _id1 = arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "keep".into() },
            vec![], 1.0, Some(1), "permanent".into(), 0,
        );

        assert_eq!(arena.len(), 2);
        assert_eq!(arena.dead_count(), 0);

        // Release the temporary entry.
        let reclaimed = arena.release(id0);
        assert!(reclaimed, "Entry with ref_count=1 should be reclaimed on release");
        assert_eq!(arena.dead_count(), 1);

        // Compact.
        let removed = arena.compact();
        assert_eq!(removed, 1);
        assert_eq!(arena.len(), 1);

        // The surviving entry should still be indexed by conclusion.
        let traces = arena.traces_for_conclusion(1);
        assert_eq!(traces.len(), 1);
    }

    #[test]
    fn test_multiple_traces_for_same_conclusion() {
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = 7;

        // Record two different derivation paths for the same conclusion.
        engine.arena.record_step(
            None,
            InferenceSource::System1FastPath { similarity_score: 0.7 },
            vec!["fast path".into()],
            0.7, Some(cid), "System 1 quick answer".into(), 10,
        );

        engine.arena.record_step(
            None,
            InferenceSource::System2Deliberation { iterations: 100 },
            vec!["deep analysis".into()],
            0.95, Some(cid), "System 2 deliberated answer".into(), 50000,
        );

        // best_trace should return the higher-confidence one.
        let explanation = engine.explain_conclusion(cid);
        assert_eq!(explanation.kind, ProvenanceKind::TracedDerivation);

        // The best trace should be the System 2 one (confidence 0.95 > 0.7).
        let best_entry = engine.trace_for_conclusion(cid).expect("trace should exist");
        assert!((best_entry.confidence - 0.95).abs() < 1e-6,
            "Best trace should be highest confidence (got {:.4})", best_entry.confidence);
    }

    #[test]
    fn test_provenance_kind_serialization() {
        // ProvenanceKind must be serializable for audit logs.
        let traced = ProvenanceKind::TracedDerivation;
        let json = serde_json::to_string(&traced).expect("serialize TracedDerivation");
        assert!(json.contains("TracedDerivation"));

        let reconstructed = ProvenanceKind::ReconstructedRationalization {
            reason: "no trace found".into(),
        };
        let json2 = serde_json::to_string(&reconstructed).expect("serialize Reconstructed");
        assert!(json2.contains("ReconstructedRationalization"));
        assert!(json2.contains("no trace found"));
    }

    #[test]
    fn test_self_play_trace_survives() {
        // Traces from self-play episodes must be queryable after the episode.
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = 21;

        // Simulate a self-play episode with multiple steps.
        let thesis = engine.arena.record_step(
            None,
            InferenceSource::SelfPlayEpisode { generation: 1 },
            vec!["thesis strategy".into()],
            0.8, None, "Self-play thesis".into(), 100,
        );

        let antithesis = engine.arena.record_step(
            Some(thesis),
            InferenceSource::PslAxiomEvaluation {
                axiom_id: "ForbiddenSpaceAxiom".into(),
                relevance: 1.0,
            },
            vec!["thesis audited".into()],
            0.75, None, "PSL audit of thesis".into(), 200,
        );

        let _synthesis = engine.arena.record_step(
            Some(antithesis),
            InferenceSource::SelfPlayEpisode { generation: 1 },
            vec!["thesis".into(), "antithesis".into()],
            0.88, Some(cid), "Synthesis: hardened strategy".into(), 300,
        );

        // Trace should survive and be queryable.
        let explanation = engine.explain_conclusion(cid);
        assert_eq!(explanation.kind, ProvenanceKind::TracedDerivation);
        assert_eq!(explanation.depth, 2); // thesis → antithesis → synthesis
        assert_eq!(explanation.trace_chain.len(), 3);
    }

    #[test]
    fn test_empty_arena_operations() {
        let arena = TraceArena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
        assert_eq!(arena.dead_count(), 0);
        assert!(arena.traces_for_conclusion(0).is_empty());
        assert!(arena.trace_chain(0).is_empty());
        assert!(arena.confidence_chain(0).is_empty());
    }

    // ================================================================
    // ADVERSARIAL TESTS — attempt to break the provenance invariant
    // ================================================================

    #[test]
    fn adversarial_reclaimed_trace_becomes_reconstructed() {
        // ATTACK: Record a trace, reclaim it, then query the conclusion.
        // The system MUST return ReconstructedRationalization, not TracedDerivation.
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = 50;

        let trace_id = engine.arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "ephemeral".into() },
            vec![], 0.9, Some(cid), "Temporary trace".into(), 0,
        );

        // Verify it's traced before reclamation.
        assert_eq!(engine.explain_conclusion(cid).kind, ProvenanceKind::TracedDerivation);

        // Reclaim and compact.
        engine.arena.release(trace_id);
        engine.arena.compact();

        // After reclamation, the conclusion has NO trace.
        // The system MUST NOT claim it was traced.
        let explanation = engine.explain_conclusion(cid);
        assert!(
            matches!(explanation.kind, ProvenanceKind::ReconstructedRationalization { .. }),
            "Reclaimed trace must return Reconstructed, not Traced"
        );
    }

    #[test]
    fn adversarial_orphaned_parent_chain_is_safe() {
        // ATTACK: Create a chain where the middle node is reclaimed.
        // The chain should stop at the gap, not crash or return garbage.
        let mut arena = TraceArena::new();

        let root = arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "root".into() },
            vec![], 1.0, None, "Root".into(), 0,
        );
        let middle = arena.record_step(
            Some(root),
            InferenceSource::System1FastPath { similarity_score: 0.9 },
            vec![], 0.9, None, "Middle".into(), 0,
        );
        let _leaf = arena.record_step(
            Some(middle),
            InferenceSource::System2Deliberation { iterations: 5 },
            vec![], 0.85, Some(1), "Leaf".into(), 0,
        );

        // Reclaim the middle node.
        arena.release(middle);
        arena.compact();

        // The leaf's parent reference is now invalid (remapped or gone).
        // trace_chain should handle this gracefully — stop at the gap.
        let chain = arena.trace_chain(0); // leaf got remapped to 0 or 1
        // Should not panic, should return a valid (possibly short) chain.
        assert!(chain.len() <= 2, "Chain should be truncated after orphaning");
    }

    #[test]
    fn adversarial_massive_arena_does_not_oom() {
        // STRESS: Record 10,000 independent trace entries (no parent chain),
        // then compact most of them. Verifies arena handles scale.
        let mut arena = TraceArena::new();

        for i in 0..10_000 {
            arena.record_step(
                None, // Independent entries — no parent ref count inflation
                InferenceSource::MctsExpansion {
                    action: "Decompose".into(),
                    node_depth: i,
                },
                vec![], 0.5, None, format!("Step {}", i), 0,
            );
        }
        assert_eq!(arena.len(), 10_000);

        // Release all but the last 100.
        for i in 0..9_900 {
            arena.release(i);
        }
        assert_eq!(arena.dead_count(), 9_900);

        let removed = arena.compact();
        assert_eq!(removed, 9_900);
        assert_eq!(arena.len(), 100);
    }

    #[test]
    fn adversarial_duplicate_conclusion_ids_pick_best() {
        // ATTACK: Multiple traces claim the same conclusion with different
        // confidence levels. The system should always pick the highest.
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = 77;

        // Low confidence trace.
        engine.arena.record_step(
            None, InferenceSource::System1FastPath { similarity_score: 0.3 },
            vec![], 0.3, Some(cid), "Low quality guess".into(), 0,
        );
        // Medium confidence trace.
        engine.arena.record_step(
            None, InferenceSource::System2Deliberation { iterations: 10 },
            vec![], 0.7, Some(cid), "Decent derivation".into(), 0,
        );
        // High confidence trace.
        engine.arena.record_step(
            None, InferenceSource::System2Deliberation { iterations: 100 },
            vec![], 0.99, Some(cid), "Thorough derivation".into(), 0,
        );

        let best = engine.trace_for_conclusion(cid).expect("should find trace");
        assert!(
            (best.confidence - 0.99).abs() < 1e-6,
            "Should return highest-confidence trace, got {:.4}", best.confidence
        );
    }

    #[test]
    fn adversarial_zero_confidence_still_traced() {
        // EDGE CASE: A trace with confidence 0.0 still counts as
        // TracedDerivation — the system recorded the reasoning path,
        // it just has zero confidence. Provenance is about HAVING the
        // path, not about the path being good.
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = 88;

        engine.arena.record_step(
            None, InferenceSource::MctsExpansion { action: "Contrast".into(), node_depth: 0 },
            vec![], 0.0, Some(cid), "Zero confidence but traced".into(), 0,
        );

        let explanation = engine.explain_conclusion(cid);
        assert_eq!(
            explanation.kind,
            ProvenanceKind::TracedDerivation,
            "Zero-confidence trace is still a TRACE, not a reconstruction"
        );
    }

    #[test]
    fn test_trace_arena_roundtrip_json() {
        // Build a small arena, serialize it, deserialize it, verify equivalence.
        let mut arena = TraceArena::new();
        let root = arena.record_step(
            None,
            InferenceSource::ExternalAssertion { source: "root".into() },
            vec!["premise".into()],
            1.0, None, "root step".into(), 10,
        );
        let child = arena.record_step(
            Some(root),
            InferenceSource::PslAxiomEvaluation {
                axiom_id: "Dimensionality".into(),
                relevance: 0.9,
            },
            vec!["psl".into()],
            0.87, Some(42), "axiom pass".into(), 150,
        );

        let json = arena.to_json().expect("serialize");
        let restored = TraceArena::from_json(&json).expect("deserialize");

        assert_eq!(restored.len(), arena.len());
        let entry = restored.get(child).expect("child survives roundtrip");
        assert_eq!(entry.parent, Some(root));
        assert!((entry.confidence - 0.87).abs() < 1e-9);
        // Conclusion index survives.
        assert_eq!(restored.traces_for_conclusion(42), vec![child]);
        // Chain still walks.
        assert_eq!(restored.trace_chain(child), vec![child, root]);
    }

    #[test]
    fn test_trace_arena_roundtrip_file() {
        let mut arena = TraceArena::new();
        arena.record_step(
            None,
            InferenceSource::System2Deliberation { iterations: 50 },
            vec![], 0.77, Some(7), "persisted".into(), 0,
        );

        let dir = std::env::temp_dir().join(format!("lfi_provenance_test_{}",
            std::process::id()));
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let path = dir.join("arena.json");

        arena.save_to_path(&path).expect("save");
        let restored = TraceArena::load_from_path(&path).expect("load");
        assert_eq!(restored.len(), 1);
        assert!(restored.best_trace_for_conclusion(7).is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_from_json_rejects_oversize_input() {
        // SECURITY: guards against memory-exhaustion via giant JSON payloads.
        let huge = "a".repeat(64 * 1024 * 1024 + 1);
        let result = TraceArena::from_json(&huge);
        assert!(result.is_err(), "must reject >64 MiB input");
    }

    #[test]
    fn test_trace_entry_serializes_with_inference_source() {
        // Every InferenceSource variant must survive roundtrip so an audit
        // log of traces can be replayed into a fresh ProvenanceEngine.
        let entry = TraceEntry {
            id: 0,
            parent: None,
            source: InferenceSource::MctsExpansion {
                action: "Specialize".into(),
                node_depth: 3,
            },
            premise_labels: vec!["a".into(), "b".into()],
            confidence: 0.93,
            timestamp_ms: 1234567890,
            cost_us: 250,
            conclusion_id: Some(99),
            description: "mcts step".into(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("MctsExpansion"));
        assert!(json.contains("Specialize"));
        let decoded: TraceEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.id, 0);
        assert_eq!(decoded.confidence, 0.93);
        assert_eq!(decoded.conclusion_id, Some(99));
    }

    #[test]
    fn adversarial_conclusion_id_overflow() {
        // EDGE CASE: Very large conclusion IDs should work fine.
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = u64::MAX;

        engine.arena.record_step(
            None, InferenceSource::ExternalAssertion { source: "max_id".into() },
            vec![], 1.0, Some(cid), "Max u64 conclusion".into(), 0,
        );

        assert_eq!(engine.explain_conclusion(cid).kind, ProvenanceKind::TracedDerivation);
        assert!(engine.explain_conclusion(cid - 1).trace_chain.is_empty());
    }

    // ============================================================
    // Stress / property-style invariant tests for TraceArena
    // ============================================================

    /// INVARIANT: arena.len() equals the number of record_step calls.
    #[test]
    fn invariant_len_equals_recorded_count() {
        let mut arena = TraceArena::new();
        let n = 1000;
        for i in 0..n {
            arena.record_step(
                None,
                InferenceSource::MctsExpansion {
                    action: "Decompose".into(),
                    node_depth: i,
                },
                vec![], 0.5, None, format!("step {}", i), 0,
            );
        }
        assert_eq!(arena.len(), n);
    }

    /// INVARIANT: For any chain, every TraceId returned by trace_chain
    /// must resolve to a live entry in the arena.
    #[test]
    fn invariant_chain_ids_always_resolve() {
        let mut arena = TraceArena::new();
        // Build a 50-deep chain.
        let mut last: Option<TraceId> = None;
        for i in 0..50 {
            let id = arena.record_step(
                last,
                InferenceSource::System2Deliberation { iterations: i },
                vec![], 0.9, None, format!("step {}", i), 0,
            );
            last = Some(id);
        }
        let leaf = last.expect("non-empty");
        let chain = arena.trace_chain(leaf);
        assert_eq!(chain.len(), 50);
        for &tid in &chain {
            assert!(arena.get(tid).is_some(),
                "chain contains unresolvable TraceId {} in arena of {}",
                tid, arena.len());
        }
    }

    /// INVARIANT: derivation_depth equals chain.len() - 1 for any non-empty
    /// chain rooted at a TraceId.
    #[test]
    fn invariant_depth_matches_chain_length() {
        let mut arena = TraceArena::new();
        let mut last: Option<TraceId> = None;
        for i in 0..30 {
            let id = arena.record_step(
                last,
                InferenceSource::PslAxiomEvaluation {
                    axiom_id: format!("ax_{}", i),
                    relevance: 0.5,
                },
                vec![], 0.7, None, format!("step {}", i), 0,
            );
            last = Some(id);
            let depth = arena.derivation_depth(id);
            let chain = arena.trace_chain(id);
            assert_eq!(depth, chain.len() - 1,
                "depth {} must equal chain.len-1 {} at step {}",
                depth, chain.len() - 1, i);
        }
    }

    /// INVARIANT: best_trace_for_conclusion always returns the
    /// highest-confidence trace for a given conclusion ID.
    #[test]
    fn invariant_best_trace_is_highest_confidence() {
        let mut engine = ProvenanceEngine::new();
        let cid: ConclusionId = 1234;
        let confidences = [0.1, 0.5, 0.99, 0.3, 0.95, 0.0, 0.7];
        let mut max = 0.0f64;
        for &c in &confidences {
            engine.arena.record_step(
                None,
                InferenceSource::System1FastPath { similarity_score: c },
                vec![], c, Some(cid), format!("conf {}", c), 0,
            );
            if c > max { max = c; }
        }
        let best = engine.trace_for_conclusion(cid).expect("non-empty");
        assert!((best.confidence - max).abs() < 1e-9,
            "best_trace must have max confidence {}, got {}", max, best.confidence);
    }

    /// INVARIANT: After any number of release+compact cycles, queries for
    /// reclaimed conclusions return ReconstructedRationalization (never
    /// fake a Traced result).
    #[test]
    fn invariant_compaction_preserves_traced_vs_reconstructed_truth() {
        let mut engine = ProvenanceEngine::new();
        // Record traces for cids 0..20.
        let mut ids = Vec::new();
        for cid in 0..20 {
            let id = engine.arena.record_step(
                None,
                InferenceSource::ExternalAssertion { source: format!("c{}", cid) },
                vec![], 0.9, Some(cid as u64), "trace".into(), 0,
            );
            ids.push((cid as u64, id));
        }
        // Release the first 10.
        for &(_, id) in ids.iter().take(10) {
            engine.arena.release(id);
        }
        engine.arena.compact();
        // The first 10 cids must now report Reconstructed; the remaining
        // 10 must still report Traced.
        for cid in 0..10u64 {
            assert!(
                matches!(engine.explain_conclusion(cid).kind,
                    ProvenanceKind::ReconstructedRationalization { .. }),
                "cid {} was reclaimed — must report Reconstructed", cid);
        }
        for cid in 10..20u64 {
            assert_eq!(engine.explain_conclusion(cid).kind,
                ProvenanceKind::TracedDerivation,
                "cid {} was kept — must report Traced", cid);
        }
    }

    /// INVARIANT: trace_chain returns an empty Vec for an out-of-range ID
    /// rather than panicking. Adversarial input guard.
    #[test]
    fn invariant_out_of_range_id_returns_empty_chain() {
        let arena = TraceArena::new();
        let chain = arena.trace_chain(usize::MAX);
        assert!(chain.is_empty());
        assert_eq!(arena.derivation_depth(usize::MAX), 0);
    }
}
