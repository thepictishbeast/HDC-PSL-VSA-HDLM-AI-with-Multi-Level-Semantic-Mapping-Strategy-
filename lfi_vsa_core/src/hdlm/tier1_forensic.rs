// ============================================================
// Tier 1 — Forensic AST Generator
// Section 1.III: "Generates mathematically perfect ASTs."
//
// This module defines the trait for Tier 1 generation: taking
// a symbolic or vector-space input and producing a verified AST.
// The AST is the immutable logical backbone; Tier 2 decorates.
// ============================================================

use crate::hdlm::ast::{Ast, NodeKind, NodeId};
use crate::hdlm::codebook::HdlmCodebook;
use crate::hdlm::error::HdlmError;
use crate::hdc::vector::BipolarVector;

/// Trait for Tier 1 forensic generators.
/// Implementations produce ASTs from various input modalities.
pub trait ForensicGenerator {
    /// Generate an AST from a sequence of symbolic tokens.
    fn generate_from_tokens(&self, tokens: &[&str]) -> Result<Ast, HdlmError>;

    /// Generate an AST from a hypervector representation.
    /// This bridges the HDC and HDLM subsystems.
    fn generate_from_vector(&self, hv: &BipolarVector) -> Result<Ast, HdlmError>;
}

/// Minimal forensic generator for arithmetic expressions.
/// Demonstrates the Tier 1 pipeline: tokens -> verified AST.
///
/// Supported grammar: number, (expr op expr)
/// where op is one of: +, -, *, /
pub struct ArithmeticGenerator;

/// Codebook-backed forensic generator.
/// Uses an HdlmCodebook to decode hypervectors back into AST nodes.
/// This bridges the HDC and HDLM subsystems bidirectionally.
pub struct CodebookGenerator {
    codebook: HdlmCodebook,
}

impl ArithmeticGenerator {
    /// Parse a flat token sequence into an expression AST.
    /// Tokens are expected in prefix notation: [op, left, right] or [literal].
    pub fn parse_prefix(ast: &mut Ast, tokens: &[&str], pos: &mut usize) -> Result<NodeId, HdlmError> {
        if *pos >= tokens.len() {
            return Err(HdlmError::Tier1GenerationFailed {
                reason: "Unexpected end of token stream".to_string(),
            });
        }

        let token = tokens[*pos];
        *pos += 1;

        debuglog!("ArithmeticGenerator::parse_prefix: token='{}', pos={}", token, *pos);

        match token {
            "+" | "-" | "*" | "/" => {
                let op_node = ast.add_node(NodeKind::BinaryOp { operator: token.to_string() });
                let left = Self::parse_prefix(ast, tokens, pos)?;
                let right = Self::parse_prefix(ast, tokens, pos)?;
                ast.add_child(op_node, left)?;
                ast.add_child(op_node, right)?;
                Ok(op_node)
            }
            _ => {
                // Attempt to parse as literal
                let _: f64 = token.parse().map_err(|_| HdlmError::Tier1GenerationFailed {
                    reason: format!("Invalid literal: '{}'", token),
                })?;
                let lit_node = ast.add_node(NodeKind::Literal { value: token.to_string() });
                Ok(lit_node)
            }
        }
    }
}

impl ForensicGenerator for ArithmeticGenerator {
    fn generate_from_tokens(&self, tokens: &[&str]) -> Result<Ast, HdlmError> {
        if tokens.is_empty() {
            return Err(HdlmError::Tier1GenerationFailed {
                reason: "Empty token stream".to_string(),
            });
        }

        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let mut pos = 0;
        let expr = Self::parse_prefix(&mut ast, tokens, &mut pos)?;
        ast.add_child(root, expr)?;

        if pos != tokens.len() {
            return Err(HdlmError::Tier1GenerationFailed {
                reason: format!("Unconsumed tokens: consumed {}, total {}", pos, tokens.len()),
            });
        }

        debuglog!(
            "ArithmeticGenerator::generate_from_tokens: {} nodes, {} tokens consumed",
            ast.node_count(), pos
        );
        Ok(ast)
    }

    fn generate_from_vector(&self, _hv: &BipolarVector) -> Result<Ast, HdlmError> {
        // Vector-to-AST decoding requires a trained codebook (item memory).
        // Structural placeholder — Beta defines the semantic mapping axioms.
        debuglog!("ArithmeticGenerator::generate_from_vector: NOT IMPLEMENTED (awaiting codebook)");
        Err(HdlmError::Tier1GenerationFailed {
            reason: "Vector-to-AST decoding requires trained HDC codebook (pending Phase 3)".to_string(),
        })
    }
}

impl CodebookGenerator {
    /// Create a new codebook-backed generator.
    /// The codebook must be pre-populated with NodeKind bases.
    pub fn new(codebook: HdlmCodebook) -> Self {
        debuglog!("CodebookGenerator::new: codebook with {} kinds", codebook.kind_count());
        Self { codebook }
    }

    /// Access the underlying codebook for encoding operations.
    pub fn codebook(&self) -> &HdlmCodebook {
        &self.codebook
    }
}

impl ForensicGenerator for CodebookGenerator {
    fn generate_from_tokens(&self, tokens: &[&str]) -> Result<Ast, HdlmError> {
        // Delegate to ArithmeticGenerator for token parsing —
        // the codebook is used for vector-based generation.
        debuglog!("CodebookGenerator::generate_from_tokens: delegating to ArithmeticGenerator");
        let arith = ArithmeticGenerator;
        arith.generate_from_tokens(tokens)
    }

    fn generate_from_vector(&self, hv: &BipolarVector) -> Result<Ast, HdlmError> {
        debuglog!("CodebookGenerator::generate_from_vector: entry, dim={}", hv.dim());

        // Step 1: Identify the closest NodeKind for the input vector.
        let (kind, similarity) = self.codebook.identify_kind(hv)?;
        debuglog!(
            "CodebookGenerator::generate_from_vector: identified kind={:?}, sim={:.4}",
            kind, similarity
        );

        // Step 2: Construct a minimal AST with the identified kind.
        // The input HV becomes the fingerprint on the decoded node.
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);
        let decoded = ast.add_node_with_hv(kind, hv.clone());
        ast.add_child(root, decoded)?;

        debuglog!(
            "CodebookGenerator::generate_from_vector: SUCCESS, nodes={}, root_child_kind={:?}",
            ast.node_count(),
            ast.get_node(decoded).map(|n| &n.kind)
        );
        Ok(ast)
    }
}

// ============================================================
// Tier 1 Tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_literal() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["42"])?;
        assert_eq!(ast.node_count(), 2); // Root + Literal
        assert_eq!(ast.leaf_count(), 1);
        Ok(())
    }

    #[test]
    fn test_simple_addition() -> Result<(), HdlmError> {
        // Prefix: + 3 4
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "3", "4"])?;
        // Root -> BinaryOp(+) -> [Literal(3), Literal(4)]
        assert_eq!(ast.node_count(), 4); // Root, +, 3, 4
        assert_eq!(ast.leaf_count(), 2);
        Ok(())
    }

    #[test]
    fn test_nested_expression() -> Result<(), HdlmError> {
        // Prefix: * + 1 2 - 5 3  =>  (1 + 2) * (5 - 3)
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["*", "+", "1", "2", "-", "5", "3"])?;
        // Root, *, +, 1, 2, -, 5, 3 = 8 nodes
        assert_eq!(ast.node_count(), 8);
        assert_eq!(ast.leaf_count(), 4); // 1, 2, 5, 3
        Ok(())
    }

    #[test]
    fn test_dfs_of_generated_ast() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "1", "2"])?;
        let order = ast.dfs()?;
        assert_eq!(order.len(), 4);
        // Root(0), BinaryOp(1), Literal(2), Literal(3)
        assert_eq!(order[0], 0); // Root first
        Ok(())
    }

    #[test]
    fn test_empty_tokens_fails() {
        let gen = ArithmeticGenerator;
        assert!(gen.generate_from_tokens(&[]).is_err());
    }

    #[test]
    fn test_invalid_literal_fails() {
        let gen = ArithmeticGenerator;
        let result = gen.generate_from_tokens(&["not_a_number"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_unconsumed_tokens_fails() {
        let gen = ArithmeticGenerator;
        // "42 99" — 42 is consumed, 99 is extra
        let result = gen.generate_from_tokens(&["42", "99"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_truncated_expression_fails() {
        let gen = ArithmeticGenerator;
        // "+" expects two operands, only gets one
        let result = gen.generate_from_tokens(&["+", "1"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_four_operators() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        for op in &["+", "-", "*", "/"] {
            let ast = gen.generate_from_tokens(&[op, "10", "5"])?;
            assert_eq!(ast.node_count(), 4);
        }
        Ok(())
    }

    // --------------------------------------------------------
    // CodebookGenerator Tests — Vector-to-AST Decoding
    // --------------------------------------------------------

    #[test]
    fn test_codebook_generator_from_tokens() -> Result<(), HdlmError> {
        use crate::hdlm::codebook::HdlmCodebook;
        let kinds = vec![NodeKind::Root, NodeKind::Literal { value: "0".to_string() }];
        let codebook = HdlmCodebook::new(&kinds)?;
        let gen = CodebookGenerator::new(codebook);
        // Token generation should delegate to ArithmeticGenerator
        let ast = gen.generate_from_tokens(&["+", "1", "2"])?;
        assert_eq!(ast.node_count(), 4);
        Ok(())
    }

    #[test]
    fn test_codebook_generator_from_vector_identifies_root() -> Result<(), HdlmError> {
        use crate::hdlm::codebook::HdlmCodebook;
        let kinds = vec![
            NodeKind::Root,
            NodeKind::Assignment,
            NodeKind::Return,
        ];
        let codebook = HdlmCodebook::new(&kinds)?;

        // Get Root's base vector and use it as input
        let root_base = codebook.get_kind_base(&NodeKind::Root)
            .ok_or(HdlmError::Tier1GenerationFailed {
                reason: "Missing Root base".to_string(),
            })?.clone();

        let gen = CodebookGenerator::new(codebook);
        let ast = gen.generate_from_vector(&root_base)?;

        // Should produce Root -> Root (identified kind)
        assert_eq!(ast.node_count(), 2);

        // The child should have an HV fingerprint
        let child_id = 1; // Second node added
        let child = ast.get_node(child_id);
        assert!(child.is_some());
        assert!(child.map(|n| n.hv_fingerprint.is_some()) == Some(true));
        Ok(())
    }

    #[test]
    fn test_codebook_generator_from_vector_identifies_assignment() -> Result<(), HdlmError> {
        use crate::hdlm::codebook::HdlmCodebook;
        let kinds = vec![
            NodeKind::Root,
            NodeKind::Assignment,
            NodeKind::Return,
        ];
        let codebook = HdlmCodebook::new(&kinds)?;

        let assign_base = codebook.get_kind_base(&NodeKind::Assignment)
            .ok_or(HdlmError::Tier1GenerationFailed {
                reason: "Missing Assignment base".to_string(),
            })?.clone();

        let gen = CodebookGenerator::new(codebook);
        let ast = gen.generate_from_vector(&assign_base)?;

        // The decoded child node should be Assignment
        let child = ast.get_node(1);
        assert!(child.is_some());
        assert_eq!(child.map(|n| &n.kind), Some(&NodeKind::Assignment));
        Ok(())
    }
}
