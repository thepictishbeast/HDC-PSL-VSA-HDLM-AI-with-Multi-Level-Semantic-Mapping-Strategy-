// ============================================================
// HDLM Codebook — Semantic Vector Mapping
// Section 1.III: Multi-Level Semantic Mapping
//
// Maps symbolic AST nodes and tokens to unique 10,000-bit
// bipolar hypervectors. This is the "Item Memory" required
// for decoding VSA structures back into symbolic ASTs.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdlm::ast::NodeKind;
use crate::hdlm::error::HdlmError;
use std::collections::HashMap;

/// Result type for codebook operations.
pub type CodebookResult<T> = Result<T, HdlmError>;

/// A codebook (item memory) for HDLM semantic mapping.
/// Stores orthogonal bases for forensic node kinds and tokens.
/// Supports bidirectional mapping: NodeKind -> HV and HV -> NodeKind.
pub struct HdlmCodebook {
    /// Mapping of NodeKind (as String key) to unique base vectors.
    kind_bases: HashMap<String, BipolarVector>,

    /// Reverse mapping: String key -> canonical NodeKind prototype.
    /// Used by identify_kind to return the correct variant.
    kind_prototypes: HashMap<String, NodeKind>,

    /// Positional encoding vectors (e.g., for child indices).
    pos_bases: Vec<BipolarVector>,
}

impl HdlmCodebook {
    /// Initialize a new codebook with fresh orthogonal bases for a set of kinds.
    /// Each kind gets a unique random base vector and a stored prototype
    /// for reverse lookup via identify_kind.
    pub fn new(kinds: &[NodeKind]) -> CodebookResult<Self> {
        debuglog!("HdlmCodebook::new: Initializing with {} kinds", kinds.len());

        let mut kind_bases = HashMap::new();
        let mut kind_prototypes = HashMap::new();
        for kind in kinds {
            let key = Self::kind_to_key(kind);
            if !kind_bases.contains_key(&key) {
                debuglog!("HdlmCodebook::new: Generating base for kind '{}'", key);
                kind_bases.insert(key.clone(), BipolarVector::new_random().map_err(|e| HdlmError::Tier1GenerationFailed {
                    reason: format!("VSA initialization failed: {}", e),
                })?);
                kind_prototypes.insert(key, kind.clone());
            }
        }

        // Generate 10 positional bases for structural hierarchy.
        let mut pos_bases = Vec::with_capacity(10);
        for idx in 0..10 {
            debuglog!("HdlmCodebook::new: Generating positional base {}", idx);
            pos_bases.push(BipolarVector::new_random().map_err(|e| HdlmError::Tier1GenerationFailed {
                reason: format!("VSA initialization failed: {}", e),
            })?);
        }

        Ok(Self { kind_bases, kind_prototypes, pos_bases })
    }

    /// Retrieve the base vector for a specific NodeKind.
    pub fn get_kind_base(&self, kind: &NodeKind) -> Option<&BipolarVector> {
        let key = Self::kind_to_key(kind);
        debuglog!("HdlmCodebook::get_kind_base: key='{}'", key);
        self.kind_bases.get(&key)
    }

    /// Retrieve a positional encoding vector by index.
    pub fn get_pos_base(&self, index: usize) -> Option<&BipolarVector> {
        debuglog!("HdlmCodebook::get_pos_base: index={}", index);
        self.pos_bases.get(index)
    }

    /// Identifies the closest NodeKind for a given hypervector.
    /// Uses the cosine similarity metric (HDC Core).
    ///
    /// Returns the NodeKind prototype with the highest cosine similarity
    /// to the query vector, along with the similarity score.
    pub fn identify_kind(&self, hv: &BipolarVector) -> CodebookResult<(NodeKind, f64)> {
        debuglog!("HdlmCodebook::identify_kind: query dim={}", hv.dim());

        let mut best_key: Option<&String> = None;
        let mut max_sim = -1.1; // Lower than possible minimum (-1.0)

        for (key, base) in &self.kind_bases {
            let sim = hv.similarity(base).map_err(|e| HdlmError::Tier1GenerationFailed {
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
        debuglog!("HdlmCodebook::encode_node: kind={:?}, pos={}", kind, position);

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
        debuglog!("HdlmCodebook::kind_count: {}", self.kind_bases.len());
        self.kind_bases.len()
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

        let sim = v1.similarity(v2).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: e.to_string()
        })?;

        // Random 10k-bit vectors should be quasi-orthogonal (sim ~ 0).
        assert!(sim.abs() < 0.1, "sim={}", sim);
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
        })?.clone();

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
        })?.clone();
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
        let sim = encoded.similarity(base).map_err(|e| HdlmError::Tier1GenerationFailed {
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

        assert!(sim01.abs() < 0.1, "pos 0 vs 1 should be orthogonal, sim={}", sim01);
        assert!(sim02.abs() < 0.1, "pos 0 vs 2 should be orthogonal, sim={}", sim02);
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
}
