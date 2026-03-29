// ============================================================
// Tier 2 — Decorative AST Expansion
// Section 1.III: "Expands the AST into aesthetic code or human
// prose without altering the foundational logic."
//
// INVARIANT: Tier 2 operations are read-only on the AST structure.
// They project/render the AST into output formats but MUST NOT
// mutate the logical tree. This is verified by taking &Ast.
// ============================================================

use crate::hdlm::ast::{Ast, NodeKind, NodeId};
use crate::hdlm::error::HdlmError;

/// Trait for Tier 2 decorative expanders.
/// Takes an immutable AST reference and produces a rendered output.
pub trait DecorativeExpander {
    /// Render the AST to a string representation.
    /// The output format depends on the implementation (code, prose, etc.).
    fn render(&self, ast: &Ast) -> Result<String, HdlmError>;
}

/// Renders an arithmetic AST to infix notation with parentheses.
/// Example: (1 + 2) * (5 - 3)
pub struct InfixRenderer;

impl InfixRenderer {
    fn render_node(&self, ast: &Ast, id: NodeId) -> Result<String, HdlmError> {
        let node = ast.get_node(id).ok_or(HdlmError::Tier2ExpansionFailed {
            reason: format!("Node {} not found in AST", id),
        })?;

        debuglog!("InfixRenderer::render_node: id={}, kind={:?}", id, node.kind);

        match &node.kind {
            NodeKind::Root => {
                if node.children.is_empty() {
                    return Err(HdlmError::Tier2ExpansionFailed {
                        reason: "Root has no children".to_string(),
                    });
                }
                // Render the first (and typically only) child of root
                self.render_node(ast, node.children[0])
            }
            NodeKind::Literal { value } => Ok(value.clone()),
            NodeKind::BinaryOp { operator } => {
                if node.children.len() != 2 {
                    return Err(HdlmError::Tier2ExpansionFailed {
                        reason: format!(
                            "BinaryOp '{}' expects 2 children, got {}",
                            operator,
                            node.children.len()
                        ),
                    });
                }
                let left = self.render_node(ast, node.children[0])?;
                let right = self.render_node(ast, node.children[1])?;
                Ok(format!("({} {} {})", left, operator, right))
            }
            NodeKind::Identifier { name } => Ok(name.clone()),
            other => Err(HdlmError::Tier2ExpansionFailed {
                reason: format!("InfixRenderer does not handle {:?}", other),
            }),
        }
    }
}

impl DecorativeExpander for InfixRenderer {
    fn render(&self, ast: &Ast) -> Result<String, HdlmError> {
        let root = ast.root_id().ok_or(HdlmError::EmptyAst)?;
        let result = self.render_node(ast, root)?;
        debuglog!("InfixRenderer::render: '{}'", result);
        Ok(result)
    }
}

/// Renders an arithmetic AST to Lisp-style S-expressions.
/// Example: (* (+ 1 2) (- 5 3))
pub struct SExprRenderer;

impl SExprRenderer {
    fn render_node(&self, ast: &Ast, id: NodeId) -> Result<String, HdlmError> {
        let node = ast.get_node(id).ok_or(HdlmError::Tier2ExpansionFailed {
            reason: format!("Node {} not found", id),
        })?;

        debuglog!("SExprRenderer::render_node: id={}, kind={:?}", id, node.kind);

        match &node.kind {
            NodeKind::Root => {
                if node.children.is_empty() {
                    return Err(HdlmError::Tier2ExpansionFailed {
                        reason: "Root has no children".to_string(),
                    });
                }
                self.render_node(ast, node.children[0])
            }
            NodeKind::Literal { value } => Ok(value.clone()),
            NodeKind::Identifier { name } => Ok(name.clone()),
            NodeKind::BinaryOp { operator } => {
                if node.children.len() != 2 {
                    return Err(HdlmError::Tier2ExpansionFailed {
                        reason: format!("BinaryOp expects 2 children, got {}", node.children.len()),
                    });
                }
                let left = self.render_node(ast, node.children[0])?;
                let right = self.render_node(ast, node.children[1])?;
                Ok(format!("({} {} {})", operator, left, right))
            }
            other => Err(HdlmError::Tier2ExpansionFailed {
                reason: format!("SExprRenderer does not handle {:?}", other),
            }),
        }
    }
}

impl DecorativeExpander for SExprRenderer {
    fn render(&self, ast: &Ast) -> Result<String, HdlmError> {
        let root = ast.root_id().ok_or(HdlmError::EmptyAst)?;
        let result = self.render_node(ast, root)?;
        debuglog!("SExprRenderer::render: '{}'", result);
        Ok(result)
    }
}

// ============================================================
// Tier 2 Tests — Decorative Expansion Proofs
// Critical invariant: Tier 2 does not mutate the AST.
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdlm::tier1_forensic::{ArithmeticGenerator, ForensicGenerator};

    #[test]
    fn test_infix_single_literal() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["42"])?;
        let output = InfixRenderer.render(&ast)?;
        assert_eq!(output, "42");
        Ok(())
    }

    #[test]
    fn test_infix_simple_addition() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "3", "4"])?;
        let output = InfixRenderer.render(&ast)?;
        assert_eq!(output, "(3 + 4)");
        Ok(())
    }

    #[test]
    fn test_infix_nested_expression() -> Result<(), HdlmError> {
        // * + 1 2 - 5 3 => (1 + 2) * (5 - 3)
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["*", "+", "1", "2", "-", "5", "3"])?;
        let output = InfixRenderer.render(&ast)?;
        assert_eq!(output, "((1 + 2) * (5 - 3))");
        Ok(())
    }

    #[test]
    fn test_sexpr_simple_addition() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "3", "4"])?;
        let output = SExprRenderer.render(&ast)?;
        assert_eq!(output, "(+ 3 4)");
        Ok(())
    }

    #[test]
    fn test_sexpr_nested_expression() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["*", "+", "1", "2", "-", "5", "3"])?;
        let output = SExprRenderer.render(&ast)?;
        assert_eq!(output, "(* (+ 1 2) (- 5 3))");
        Ok(())
    }

    #[test]
    fn test_tier2_does_not_mutate_ast() -> Result<(), HdlmError> {
        // Critical invariant: rendering is read-only.
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "1", "2"])?;
        let count_before = ast.node_count();
        let _ = InfixRenderer.render(&ast)?;
        let _ = SExprRenderer.render(&ast)?;
        assert_eq!(ast.node_count(), count_before, "Tier 2 must not mutate the AST");
        Ok(())
    }

    #[test]
    fn test_render_empty_ast_fails() {
        let ast = Ast::new();
        assert!(InfixRenderer.render(&ast).is_err());
        assert!(SExprRenderer.render(&ast).is_err());
    }

    #[test]
    fn test_both_renderers_agree_on_structure() -> Result<(), HdlmError> {
        // Both renderers should process the same AST without errors.
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["/", "*", "2", "3", "+", "4", "5"])?;
        let infix = InfixRenderer.render(&ast)?;
        let sexpr = SExprRenderer.render(&ast)?;
        assert!(!infix.is_empty());
        assert!(!sexpr.is_empty());
        // Both should contain the same operators and operands
        for token in &["2", "3", "4", "5", "*", "+", "/"] {
            assert!(infix.contains(token), "Infix missing '{}'", token);
            assert!(sexpr.contains(token), "S-expr missing '{}'", token);
        }
        Ok(())
    }
}
