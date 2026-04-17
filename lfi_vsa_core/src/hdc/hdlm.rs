// ============================================================
// HDLM: Hyperdimensional Language Model — Semantic Mapping
// Section 1.III: Multi-Level Semantic Mapping
// Tier 1 (Forensic): AST logic vectors.
// Tier 2 (Decorative): Aesthetic expansion and prose.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Result type for HDLM operations.
pub type HdlmResult<T> = Result<T, HdcError>;

/// Tier 1: Forensic AST Node Types.
/// These represent the mathematically perfect logic of the code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ForensicNode {
    Module,
    Function,
    Statement,
    Expression,
    Literal,
    Identifier,
    Unknown,
}

/// A Semantic Mapping between a Forensic AST and the VSA space.
/// Enforces strict separation between logic (Tier 1) and decoration (Tier 2).
pub struct SemanticMap {
    /// Mapping of node types to their unique, orthogonal base vectors.
    node_bases: HashMap<ForensicNode, BipolarVector>,
    
    /// Positional encoding vectors for structural hierarchy.
    pos_bases: Vec<BipolarVector>,
}

impl SemanticMap {
    /// Initialize a new SemanticMap with fresh orthogonal bases.
    pub fn new() -> HdlmResult<Self> {
        debuglog!("SemanticMap::new: Initializing forensic bases");
        
        let mut node_bases = HashMap::new();
        node_bases.insert(ForensicNode::Module, BipolarVector::new_random()?);
        node_bases.insert(ForensicNode::Function, BipolarVector::new_random()?);
        node_bases.insert(ForensicNode::Statement, BipolarVector::new_random()?);
        node_bases.insert(ForensicNode::Expression, BipolarVector::new_random()?);
        node_bases.insert(ForensicNode::Literal, BipolarVector::new_random()?);
        node_bases.insert(ForensicNode::Identifier, BipolarVector::new_random()?);
        node_bases.insert(ForensicNode::Unknown, BipolarVector::new_random()?);
        
        // Positional bases for up to 10 children per node.
        let mut pos_bases = Vec::with_capacity(10);
        for i in 0..10 {
            debuglog!("SemanticMap::new: Generating positional base {}", i);
            pos_bases.push(BipolarVector::new_random()?);
        }
        
        Ok(Self { node_bases, pos_bases })
    }

    /// Returns the positional encoding base vector at the given child index.
    /// Used for encoding structural hierarchy in the AST.
    pub fn get_pos_base(&self, index: usize) -> Option<&BipolarVector> {
        debuglog!("SemanticMap::get_pos_base: index={}", index);
        self.pos_bases.get(index)
    }

    /// Projects a Forensic Node and its Decorative metadata into a single Hypervector.
    ///
    /// `V = XOR(ForensicBase, DecorativeVector)`
    pub fn project_node(
        &self, 
        node: ForensicNode, 
        decoration: &BipolarVector
    ) -> HdlmResult<BipolarVector> {
        debuglog!("project_node: entry, node={:?}", node);
        
        let base = self.node_bases.get(&node).ok_or_else(|| {
            debuglog!("project_node: FAIL - MissingBase for {:?}", node);
            HdcError::InitializationFailed {
                reason: format!("No base vector for node type {:?}", node),
            }
        })?;
        
        // Bind the logic to the decoration.
        // Resulting vector is quasi-orthogonal to both, but invertible if decoration is known.
        let projected = base.bind(decoration)?;
        
        debuglog!("project_node: SUCCESS, node={:?}, similarity_to_base={:.4}", 
            node, projected.similarity(base)?);
            
        Ok(projected)
    }

    /// Verifies if a projected vector contains a specific Forensic Node.
    ///
    /// `Verification = CosineSimilarity(XOR(Projected, Decoration), ForensicBase) \approx 1.0`
    pub fn verify_forensic_integrity(
        &self,
        projected: &BipolarVector,
        node: ForensicNode,
        decoration: &BipolarVector
    ) -> HdlmResult<bool> {
        debuglog!("verify_forensic_integrity: entry, node={:?}", node);
        
        let base = self.node_bases.get(&node).ok_or_else(|| {
            HdcError::InitializationFailed {
                reason: format!("No base vector for node type {:?}", node),
            }
        })?;
        
        // Unbind the decoration to recover the forensic logic.
        let recovered = projected.bind(decoration)?;
        let sim = recovered.similarity(base)?;
        
        debuglog!("verify_forensic_integrity: recovered_similarity={:.4}", sim);
        
        // Tolerance for floating point and VSA noise (should be exactly 1.0 in this model).
        Ok(sim > 0.99)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_mapping_separation() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        let logic = ForensicNode::Function;
        let decoration = BipolarVector::new_random()?;
        let projected = map.project_node(logic.clone(), &decoration)?;
        assert!(map.verify_forensic_integrity(&projected, logic, &decoration)?);
        let fake_decoration = BipolarVector::new_random()?;
        assert!(!map.verify_forensic_integrity(&projected, ForensicNode::Function, &fake_decoration)?);
        Ok(())
    }

    #[test]
    fn test_all_node_types_have_bases() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        let nodes = vec![
            ForensicNode::Module, ForensicNode::Function,
            ForensicNode::Statement, ForensicNode::Expression,
            ForensicNode::Literal, ForensicNode::Identifier,
            ForensicNode::Unknown,
        ];
        let decoration = BipolarVector::new_random()?;
        for node in nodes {
            let projected = map.project_node(node, &decoration)?;
            assert_eq!(projected.dim(), 10000);
        }
        Ok(())
    }

    #[test]
    fn test_different_nodes_different_projections() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        let decoration = BipolarVector::new_random()?;
        let p_func = map.project_node(ForensicNode::Function, &decoration)?;
        let p_lit = map.project_node(ForensicNode::Literal, &decoration)?;
        let sim = p_func.similarity(&p_lit)?;
        assert!(sim < 0.9, "Different node types should produce different projections: {:.4}", sim);
        Ok(())
    }

    #[test]
    fn test_positional_bases_exist() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        for i in 0..10 {
            assert!(map.get_pos_base(i).is_some(), "Position {} should have a base", i);
        }
        assert!(map.get_pos_base(10).is_none(), "Position 10 should not exist");
        Ok(())
    }

    #[test]
    fn test_wrong_node_fails_verification() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        let decoration = BipolarVector::new_random()?;
        let projected = map.project_node(ForensicNode::Module, &decoration)?;
        // Verify with wrong node type should fail.
        let wrong = map.verify_forensic_integrity(&projected, ForensicNode::Statement, &decoration)?;
        assert!(!wrong, "Wrong forensic node should fail verification");
        Ok(())
    }

    #[test]
    fn test_forensic_node_equality() {
        assert_eq!(ForensicNode::Function, ForensicNode::Function);
        assert_ne!(ForensicNode::Function, ForensicNode::Literal);
    }

    // ============================================================
    // Stress / invariant tests for SemanticMap
    // ============================================================

    /// INVARIANT: project_node is deterministic — same node + decoration →
    /// same vector across calls.
    #[test]
    fn invariant_project_node_deterministic() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        let dec = BipolarVector::new_random().expect("random");
        let v1 = map.project_node(ForensicNode::Function, &dec)?;
        let v2 = map.project_node(ForensicNode::Function, &dec)?;
        assert_eq!(v1, v2,
            "deterministic projection required");
        Ok(())
    }

    /// INVARIANT: different decorations produce different projections.
    #[test]
    fn invariant_different_decorations_different_projections() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        let d1 = BipolarVector::new_random().expect("random");
        let d2 = BipolarVector::new_random().expect("random");
        let v1 = map.project_node(ForensicNode::Function, &d1)?;
        let v2 = map.project_node(ForensicNode::Function, &d2)?;
        assert_ne!(v1, v2,
            "different decorations must produce different projections");
        Ok(())
    }

    /// INVARIANT: get_pos_base returns Some for index 0.
    #[test]
    fn invariant_get_pos_base_low_indices_some() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        assert!(map.get_pos_base(0).is_some());
        Ok(())
    }

    /// INVARIANT: get_pos_base returns None for out-of-range indices.
    #[test]
    fn invariant_get_pos_base_out_of_range_none() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        assert!(map.get_pos_base(10).is_none());
        assert!(map.get_pos_base(999).is_none());
        assert!(map.get_pos_base(usize::MAX).is_none());
        Ok(())
    }

    /// INVARIANT: ForensicNode enum serde round-trip.
    #[test]
    fn invariant_forensic_node_serde_roundtrip() {
        let nodes = [
            ForensicNode::Module, ForensicNode::Function,
            ForensicNode::Statement, ForensicNode::Expression,
            ForensicNode::Literal, ForensicNode::Identifier,
            ForensicNode::Unknown,
        ];
        for n in nodes {
            let json = serde_json::to_string(&n).unwrap();
            let recovered: ForensicNode = serde_json::from_str(&json).unwrap();
            assert_eq!(n, recovered);
        }
    }

    /// INVARIANT: verify_forensic_integrity succeeds for a correctly-projected
    /// node (returns Ok result without panic).
    #[test]
    fn invariant_verify_forensic_succeeds_on_valid() -> HdlmResult<()> {
        let map = SemanticMap::new()?;
        let decor = BipolarVector::new_random()?;
        let v = map.project_node(ForensicNode::Function, &decor)?;
        // verify_forensic_integrity takes (projected_vec, kind, decoration)
        let result = map.verify_forensic_integrity(&v, ForensicNode::Function, &decor);
        assert!(result.is_ok(), "valid integrity verification should succeed");
        Ok(())
    }
}
