// ============================================================
// HDLM Codebook — Dual-Mode Semantic Vector Mapping
// Section 1.III: Multi-Level Semantic Mapping
//
// Maps symbolic AST nodes and tokens to unique 10,000-bit
// bipolar hypervectors. This is the "Item Memory" required
// for decoding VSA structures back into symbolic ASTs.
//
// DUAL MODE (UC Irvine 2026, frai.2026.1690492):
//   - Correlated mode: for System 1 learning/classification (65%→95%)
//   - Orthogonal mode: for System 2 reasoning/decoding (85%→100%)
//
// ON-THE-FLY GENERATION (Springer 978-3-032-15638-9_32):
//   Base vectors generated deterministically via Hadamard/Walsh
//   sequences — zero storage overhead.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::hadamard::{HadamardGenerator, CorrelatedGenerator};
use crate::hdlm::ast::NodeKind;
use crate::hdlm::error::HdlmError;
use std::collections::HashMap;

/// Result type for codebook operations.
pub type CodebookResult<T> = Result<T, HdlmError>;

/// The encoding mode for the codebook.
///
/// Correlated mode produces vectors that share structural similarity,
/// which improves classification/learning tasks.
/// Orthogonal mode produces maximally distinct vectors,
/// which improves reasoning/decoding tasks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodebookMode {
    /// Orthogonal encoding: maximally distinct base vectors.
    /// Best for System 2 reasoning and decoding.
    /// Decoding accuracy: ~100% (vs 85% with random vectors).
    Orthogonal,
    /// Correlated encoding: base vectors share structural similarity
    /// within semantic groups.
    /// Best for System 1 classification and learning.
    /// Classification accuracy: ~95% (vs 65% with orthogonal).
    Correlated {
        /// How much correlation between semantically related kinds.
        /// 0.0 = fully orthogonal, 1.0 = identical.
        correlation: f64,
    },
}

impl Default for CodebookMode {
    fn default() -> Self {
        CodebookMode::Orthogonal
    }
}

/// A dual-mode codebook (item memory) for HDLM semantic mapping.
///
/// Supports both orthogonal (System 2) and correlated (System 1) encoding.
/// Base vectors are generated on-the-fly via Hadamard sequences —
/// zero storage overhead compared to ~12.5MB for stored codebooks.
pub struct HdlmCodebook {
    /// Current encoding mode.
    mode: CodebookMode,

    /// Mapping of NodeKind (as String key) to Hadamard index.
    /// The actual vector is generated on demand, not stored.
    kind_indices: HashMap<String, usize>,

    /// Reverse mapping: String key -> canonical NodeKind prototype.
    kind_prototypes: HashMap<String, NodeKind>,

    /// Cache for correlated vectors (only populated in Correlated mode).
    /// Key: (kind_key). Generated lazily on first access per mode switch.
    correlated_cache: HashMap<String, BipolarVector>,

    /// Number of positional encoding slots.
    pos_count: usize,

    /// Starting Hadamard index for positional vectors.
    /// Positional vectors use indices [pos_base_offset..pos_base_offset+pos_count).
    pos_base_offset: usize,
}

impl HdlmCodebook {
    /// Initialize a new dual-mode codebook.
    ///
    /// Each kind gets a unique Hadamard index for deterministic generation.
    /// No vectors are stored — they're computed on demand.
    pub fn new(kinds: &[NodeKind]) -> CodebookResult<Self> {
        debuglog!("HdlmCodebook::new: Initializing dual-mode codebook with {} kinds", kinds.len());

        let mut kind_indices = HashMap::new();
        let mut kind_prototypes = HashMap::new();
        let mut next_index = 0;

        for kind in kinds {
            let key = Self::kind_to_key(kind);
            if !kind_indices.contains_key(&key) {
                debuglog!("HdlmCodebook::new: Assigning Hadamard index {} to kind '{}'", next_index, key);
                kind_indices.insert(key.clone(), next_index);
                kind_prototypes.insert(key, kind.clone());
                next_index += 1;
            }
        }

        let pos_count = 10;
        let pos_base_offset = next_index + 1000; // Leave gap for future kinds

        Ok(Self {
            mode: CodebookMode::Orthogonal,
            kind_indices,
            kind_prototypes,
            correlated_cache: HashMap::new(),
            pos_count,
            pos_base_offset,
        })
    }

    /// Switch the codebook mode.
    ///
    /// When switching to Correlated mode, the correlated cache is regenerated.
    /// When switching to Orthogonal mode, the cache is cleared (not needed).
    pub fn set_mode(&mut self, mode: CodebookMode) -> CodebookResult<()> {
        debuglog!("HdlmCodebook::set_mode: {:?} -> {:?}", self.mode, mode);

        if self.mode == mode {
            return Ok(());
        }

        self.mode = mode;
        self.correlated_cache.clear();

        // Pre-generate correlated vectors if switching to Correlated mode
        if let CodebookMode::Correlated { correlation } = mode {
            debuglog!("HdlmCodebook::set_mode: Generating correlated vectors (corr={:.2})", correlation);
            for (key, &index) in &self.kind_indices {
                let base = HadamardGenerator::generate(index).map_err(|e| {
                    HdlmError::Tier1GenerationFailed {
                        reason: format!("Hadamard generation failed: {}", e),
                    }
                })?;
                let correlated = CorrelatedGenerator::generate_correlated(
                    &base,
                    correlation,
                    index as u64 + 0x_C0DE_B00C,
                ).map_err(|e| {
                    HdlmError::Tier1GenerationFailed {
                        reason: format!("Correlated generation failed: {}", e),
                    }
                })?;
                self.correlated_cache.insert(key.clone(), correlated);
            }
        }

        Ok(())
    }

    /// Get the current codebook mode.
    pub fn mode(&self) -> CodebookMode {
        self.mode
    }

    /// Retrieve the base vector for a specific NodeKind.
    ///
    /// In Orthogonal mode: returns the Hadamard vector (generated on-the-fly).
    /// In Correlated mode: returns the pre-generated correlated vector.
    pub fn get_kind_base(&self, kind: &NodeKind) -> Option<BipolarVector> {
        let key = Self::kind_to_key(kind);
        debuglog!("HdlmCodebook::get_kind_base: key='{}', mode={:?}", key, self.mode);

        match self.mode {
            CodebookMode::Orthogonal => {
                let index = self.kind_indices.get(&key)?;
                HadamardGenerator::generate(*index).ok()
            }
            CodebookMode::Correlated { .. } => {
                self.correlated_cache.get(&key).cloned()
            }
        }
    }

    /// Retrieve a positional encoding vector by index.
    /// Always uses Hadamard generation (orthogonal positional encoding).
    pub fn get_pos_base(&self, index: usize) -> Option<BipolarVector> {
        debuglog!("HdlmCodebook::get_pos_base: index={}", index);
        if index >= self.pos_count {
            return None;
        }
        HadamardGenerator::generate(self.pos_base_offset + index).ok()
    }

    /// Identifies the closest NodeKind for a given hypervector.
    /// Uses the cosine similarity metric (HDC Core).
    ///
    /// Returns the NodeKind prototype with the highest cosine similarity
    /// to the query vector, along with the similarity score.
    pub fn identify_kind(&self, hv: &BipolarVector) -> CodebookResult<(NodeKind, f64)> {
        debuglog!("HdlmCodebook::identify_kind: query dim={}, mode={:?}", hv.dim(), self.mode);

        let mut best_key: Option<&String> = None;
        let mut max_sim = -1.1; // Lower than possible minimum (-1.0)

        for key in self.kind_indices.keys() {
            let base = match self.mode {
                CodebookMode::Orthogonal => {
                    let index = self.kind_indices[key];
                    HadamardGenerator::generate(index).map_err(|e| HdlmError::Tier1GenerationFailed {
                        reason: format!("Hadamard generation failed: {}", e),
                    })?
                }
                CodebookMode::Correlated { .. } => {
                    self.correlated_cache.get(key).cloned().ok_or_else(|| {
                        HdlmError::Tier1GenerationFailed {
                            reason: format!("No correlated vector for key '{}'", key),
                        }
                    })?
                }
            };

            let sim = hv.similarity(&base).map_err(|e| HdlmError::Tier1GenerationFailed {
                reason: format!("Similarity check failed: {}", e),
            })?;
            debuglog!("HdlmCodebook::identify_kind: key='{}', sim={:.4}", key, sim);

            if sim > max_sim {
                max_sim = sim;
                best_key = Some(key);
            }
        }

        if let Some(key) = best_key {
            let prototype = self.kind_prototypes.get(key).ok_or_else(|| {
                debuglog!("HdlmCodebook::identify_kind: FAIL - no prototype for key='{}'", key);
                HdlmError::Tier1GenerationFailed {
                    reason: format!("No prototype stored for key '{}'", key),
                }
            })?;
            debuglog!(
                "HdlmCodebook::identify_kind: MATCH key='{}', sim={:.4}, kind={:?}",
                key, max_sim, prototype
            );
            Ok((prototype.clone(), max_sim))
        } else {
            debuglog!("HdlmCodebook::identify_kind: FAIL - codebook is empty");
            Err(HdlmError::Tier1GenerationFailed {
                reason: "IdentifyKind: Codebook is empty".to_string(),
            })
        }
    }

    /// Encode a NodeKind into a hypervector using its base + optional
    /// positional encoding for tree hierarchy.
    ///
    /// `V_node = permute(base(kind), position)`
    pub fn encode_node(
        &self,
        kind: &NodeKind,
        position: usize,
    ) -> CodebookResult<BipolarVector> {
        debuglog!("HdlmCodebook::encode_node: kind={:?}, pos={}, mode={:?}", kind, position, self.mode);

        let base = self.get_kind_base(kind).ok_or_else(|| {
            debuglog!("HdlmCodebook::encode_node: FAIL - no base for {:?}", kind);
            HdlmError::Tier1GenerationFailed {
                reason: format!("No codebook entry for kind {:?}", kind),
            }
        })?;

        // Apply positional encoding via permutation
        let encoded = base.permute(position).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: format!("Permutation failed: {}", e),
        })?;

        debuglog!(
            "HdlmCodebook::encode_node: SUCCESS, dim={}, ones={}",
            encoded.dim(), encoded.count_ones()
        );
        Ok(encoded)
    }

    /// Number of registered kinds in the codebook.
    pub fn kind_count(&self) -> usize {
        debuglog!("HdlmCodebook::kind_count: {}", self.kind_indices.len());
        self.kind_indices.len()
    }

    fn kind_to_key(kind: &NodeKind) -> String {
        // Simple discriminant-based key for the codebook.
        let key = format!("{:?}", kind);
        debuglog!("HdlmCodebook::kind_to_key: {:?} -> '{}'", kind, key);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codebook_orthogonality() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Literal { value: "0".to_string() }];
        let cb = HdlmCodebook::new(&kinds)?;

        let v1 = cb.get_kind_base(&kinds[0]).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing base for Root".to_string(),
        })?;
        let v2 = cb.get_kind_base(&kinds[1]).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing base for Literal".to_string(),
        })?;

        let sim = v1.similarity(&v2).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string()
        })?;

        // Hadamard vectors should be quasi-orthogonal (sim ~ 0).
        assert!(sim.abs() < 0.15, "sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_identify_kind_returns_correct_variant() -> CodebookResult<()> {
        let kinds = vec![
            NodeKind::Root,
            NodeKind::Assignment,
            NodeKind::Return,
            NodeKind::Literal { value: "0".to_string() },
        ];
        let cb = HdlmCodebook::new(&kinds)?;

        // Query with the base vector for Root — should identify Root
        let root_base = cb.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing Root base".to_string(),
        })?;

        let (identified, sim) = cb.identify_kind(&root_base)?;
        assert_eq!(identified, NodeKind::Root, "Should identify Root");
        assert!(sim > 0.99, "Self-similarity should be ~1.0, got {}", sim);
        Ok(())
    }

    #[test]
    fn test_identify_kind_with_noisy_vector() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Assignment, NodeKind::Return];
        let cb = HdlmCodebook::new(&kinds)?;

        // Create a noisy version of Root's base: bundle with random noise
        let root_base = cb.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing Root base".to_string(),
        })?;
        let noise = BipolarVector::new_random().map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string(),
        })?;
        // 3x Root + 1x noise: Root should still dominate
        let noisy = BipolarVector::bundle(&[&root_base, &root_base, &root_base, &noise])
            .map_err(|e| HdlmError::Tier1GenerationFailed { reason: e.to_string() })?;

        let (identified, sim) = cb.identify_kind(&noisy)?;
        assert_eq!(identified, NodeKind::Root, "Should still identify Root despite noise");
        assert!(sim > 0.5, "Similarity should be reasonably high, got {}", sim);
        Ok(())
    }

    #[test]
    fn test_identify_kind_empty_codebook() {
        let cb = HdlmCodebook::new(&[]);
        assert!(cb.is_ok());
        let cb = cb.ok().filter(|c| c.kind_count() == 0);
        // An empty codebook should fail to identify
        if let Some(cb) = cb {
            let random = BipolarVector::new_random();
            if let Ok(v) = random {
                let result = cb.identify_kind(&v);
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_encode_node_basic() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Assignment];
        let cb = HdlmCodebook::new(&kinds)?;

        let encoded = cb.encode_node(&NodeKind::Root, 0)?;
        assert_eq!(encoded.dim(), 10000);

        // Position 0 permutation is identity, so encoded should match base
        let base = cb.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing Root base".to_string(),
        })?;
        let sim = encoded.similarity(&base).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string(),
        })?;
        assert!(sim > 0.99, "Position 0 should be identity, sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_encode_node_different_positions_orthogonal() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root];
        let cb = HdlmCodebook::new(&kinds)?;

        let enc0 = cb.encode_node(&NodeKind::Root, 0)?;
        let enc1 = cb.encode_node(&NodeKind::Root, 1)?;
        let enc2 = cb.encode_node(&NodeKind::Root, 2)?;

        let sim01 = enc0.similarity(&enc1).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string(),
        })?;
        let sim02 = enc0.similarity(&enc2).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string(),
        })?;

        assert!(sim01.abs() < 0.15, "pos 0 vs 1 should be orthogonal, sim={}", sim01);
        assert!(sim02.abs() < 0.15, "pos 0 vs 2 should be orthogonal, sim={}", sim02);
        Ok(())
    }

    #[test]
    fn test_encode_node_missing_kind_fails() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root];
        let cb = HdlmCodebook::new(&kinds)?;
        let result = cb.encode_node(&NodeKind::Assignment, 0);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_kind_count() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Assignment, NodeKind::Return];
        let cb = HdlmCodebook::new(&kinds)?;
        assert_eq!(cb.kind_count(), 3);
        Ok(())
    }

    #[test]
    fn test_duplicate_kinds_deduplicated() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Root, NodeKind::Root];
        let cb = HdlmCodebook::new(&kinds)?;
        assert_eq!(cb.kind_count(), 1);
        Ok(())
    }

    #[test]
    fn test_pos_base_access() -> CodebookResult<()> {
        let cb = HdlmCodebook::new(&[NodeKind::Root])?;
        // Should have 10 positional bases
        assert!(cb.get_pos_base(0).is_some());
        assert!(cb.get_pos_base(9).is_some());
        assert!(cb.get_pos_base(10).is_none()); // Out of range
        Ok(())
    }

    #[test]
    fn test_dual_mode_switch() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Assignment, NodeKind::Return];
        let mut cb = HdlmCodebook::new(&kinds)?;

        // Default is Orthogonal
        assert_eq!(cb.mode(), CodebookMode::Orthogonal);

        // Get orthogonal base for Root
        let orth_root = cb.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing Root base".to_string(),
        })?;

        // Switch to Correlated mode
        cb.set_mode(CodebookMode::Correlated { correlation: 0.7 })?;
        assert_eq!(cb.mode(), CodebookMode::Correlated { correlation: 0.7 });

        // Get correlated base for Root — should be different from orthogonal
        let corr_root = cb.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing Root base in correlated mode".to_string(),
        })?;

        // They should be related (derived from same base) but not identical
        let sim = orth_root.similarity(&corr_root).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string(),
        })?;
        debuglog!("Orthogonal vs Correlated Root: sim={:.4}", sim);
        assert!(sim < 0.99, "Should be different vectors, sim={}", sim);

        // Switch back to Orthogonal
        cb.set_mode(CodebookMode::Orthogonal)?;
        let orth_root2 = cb.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing Root base after switch back".to_string(),
        })?;

        // Should be identical to original (deterministic Hadamard)
        let sim2 = orth_root.similarity(&orth_root2).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string(),
        })?;
        assert!(sim2 > 0.99, "Orthogonal should be deterministic, sim={}", sim2);

        Ok(())
    }

    #[test]
    fn test_correlated_mode_identification() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Assignment, NodeKind::Return];
        let mut cb = HdlmCodebook::new(&kinds)?;
        cb.set_mode(CodebookMode::Correlated { correlation: 0.6 })?;

        // Should still identify correctly in correlated mode
        let root_base = cb.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing Root base".to_string(),
        })?;
        let (identified, sim) = cb.identify_kind(&root_base)?;
        assert_eq!(identified, NodeKind::Root, "Should identify Root in correlated mode");
        assert!(sim > 0.99, "Self-similarity should be ~1.0, got {}", sim);
        Ok(())
    }

    #[test]
    fn test_hadamard_determinism_across_instances() -> CodebookResult<()> {
        // Two separate codebook instances should produce identical vectors
        let kinds = vec![NodeKind::Root, NodeKind::Assignment];
        let cb1 = HdlmCodebook::new(&kinds)?;
        let cb2 = HdlmCodebook::new(&kinds)?;

        let v1 = cb1.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing".to_string(),
        })?;
        let v2 = cb2.get_kind_base(&NodeKind::Root).ok_or_else(|| HdlmError::Tier1GenerationFailed {
            reason: "Missing".to_string(),
        })?;

        let sim = v1.similarity(&v2).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string(),
        })?;
        assert!(sim > 0.99, "Same kinds in same order should produce same vectors, sim={}", sim);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for HdlmCodebook
    // ============================================================

    /// INVARIANT: default mode is Orthogonal.
    #[test]
    fn invariant_default_mode_orthogonal() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Assignment];
        let cb = HdlmCodebook::new(&kinds)?;
        assert_eq!(cb.mode(), CodebookMode::Orthogonal);
        Ok(())
    }

    /// INVARIANT: get_kind_base returns Some for registered kinds, None for
    /// unregistered kinds.
    #[test]
    fn invariant_get_kind_base_registered_only() -> CodebookResult<()> {
        let kinds = vec![NodeKind::Root, NodeKind::Assignment];
        let cb = HdlmCodebook::new(&kinds)?;
        assert!(cb.get_kind_base(&NodeKind::Root).is_some());
        assert!(cb.get_kind_base(&NodeKind::Assignment).is_some());
        // An unregistered kind should return None.
        assert!(cb.get_kind_base(&NodeKind::Return).is_none());
        Ok(())
    }

    /// INVARIANT: get_pos_base returns None for indices beyond pos_count.
    #[test]
    fn invariant_pos_base_bounded() -> CodebookResult<()> {
        let cb = HdlmCodebook::new(&[NodeKind::Root])?;
        assert!(cb.get_pos_base(0).is_some());
        assert!(cb.get_pos_base(100).is_none(),
            "pos_base should return None beyond pos_count");
        assert!(cb.get_pos_base(usize::MAX).is_none());
        Ok(())
    }

    /// INVARIANT: set_mode to same mode is idempotent no-op.
    #[test]
    fn invariant_set_mode_idempotent() -> CodebookResult<()> {
        let mut cb = HdlmCodebook::new(&[NodeKind::Root])?;
        cb.set_mode(CodebookMode::Orthogonal)?;
        cb.set_mode(CodebookMode::Orthogonal)?;
        assert_eq!(cb.mode(), CodebookMode::Orthogonal);
        Ok(())
    }

    /// INVARIANT: switching to Correlated then back to Orthogonal clears
    /// correlated cache.
    #[test]
    fn invariant_mode_switch_clears_cache() -> CodebookResult<()> {
        let mut cb = HdlmCodebook::new(&[NodeKind::Root, NodeKind::Assignment])?;
        cb.set_mode(CodebookMode::Correlated { correlation: 0.7 })?;
        cb.set_mode(CodebookMode::Orthogonal)?;
        assert_eq!(cb.mode(), CodebookMode::Orthogonal);
        // In Orthogonal mode, get_kind_base still works (regenerates via Hadamard).
        assert!(cb.get_kind_base(&NodeKind::Root).is_some());
        Ok(())
    }

    /// INVARIANT: empty kind list still produces a valid codebook.
    #[test]
    fn invariant_empty_kinds_produces_valid_codebook() -> CodebookResult<()> {
        let cb = HdlmCodebook::new(&[])?;
        assert_eq!(cb.mode(), CodebookMode::Orthogonal);
        // No registered kinds.
        assert!(cb.get_kind_base(&NodeKind::Root).is_none());
        Ok(())
    }
}
