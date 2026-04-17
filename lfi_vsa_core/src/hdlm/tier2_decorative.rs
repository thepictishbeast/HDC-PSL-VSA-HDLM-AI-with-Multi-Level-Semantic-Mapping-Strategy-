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

/// Renders an AST to a JSON-like structured representation.
/// Useful for API responses and machine-readable output.
pub struct JsonRenderer;

impl JsonRenderer {
    fn render_node(&self, ast: &Ast, id: NodeId) -> Result<String, HdlmError> {
        let node = ast.get_node(id).ok_or(HdlmError::Tier2ExpansionFailed {
            reason: format!("Node {} not found", id),
        })?;

        match &node.kind {
            NodeKind::Root => {
                if node.children.is_empty() {
                    return Ok(r#"{"type":"root","children":[]}"#.into());
                }
                let children: Vec<String> = node.children.iter()
                    .filter_map(|&c| self.render_node(ast, c).ok())
                    .collect();
                Ok(format!(r#"{{"type":"root","children":[{}]}}"#, children.join(",")))
            }
            NodeKind::Literal { value } => {
                Ok(format!(r#"{{"type":"literal","value":"{}"}}"#, value))
            }
            NodeKind::Identifier { name } => {
                Ok(format!(r#"{{"type":"identifier","name":"{}"}}"#, name))
            }
            NodeKind::BinaryOp { operator } => {
                let children: Vec<String> = node.children.iter()
                    .filter_map(|&c| self.render_node(ast, c).ok())
                    .collect();
                Ok(format!(r#"{{"type":"binop","op":"{}","children":[{}]}}"#,
                    operator, children.join(",")))
            }
            NodeKind::Block { name } => {
                let children: Vec<String> = node.children.iter()
                    .filter_map(|&c| self.render_node(ast, c).ok())
                    .collect();
                Ok(format!(r#"{{"type":"block","name":"{}","children":[{}]}}"#,
                    name, children.join(",")))
            }
            NodeKind::Sentence => {
                let children: Vec<String> = node.children.iter()
                    .filter_map(|&c| self.render_node(ast, c).ok())
                    .collect();
                Ok(format!(r#"{{"type":"sentence","children":[{}]}}"#, children.join(",")))
            }
            NodeKind::Phrase { text } => {
                Ok(format!(r#"{{"type":"phrase","text":"{}"}}"#, text))
            }
            _ => {
                Ok(format!(r#"{{"type":"unknown","kind":"{:?}"}}"#, node.kind))
            }
        }
    }
}

impl DecorativeExpander for JsonRenderer {
    fn render(&self, ast: &Ast) -> Result<String, HdlmError> {
        let root = ast.root_id().ok_or(HdlmError::EmptyAst)?;
        let result = self.render_node(ast, root)?;
        debuglog!("JsonRenderer::render: {} bytes", result.len());
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
    fn test_json_simple() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "1", "2"])?;
        let output = JsonRenderer.render(&ast)?;
        assert!(output.contains("binop"), "JSON should contain binop: {}", output);
        assert!(output.contains("\"1\""), "JSON should contain literal 1");
        assert!(output.contains("\"2\""), "JSON should contain literal 2");
        assert!(output.contains("\"+\""), "JSON should contain operator +");
        Ok(())
    }

    #[test]
    fn test_json_does_not_mutate() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["*", "3", "4"])?;
        let count_before = ast.node_count();
        let _ = JsonRenderer.render(&ast)?;
        assert_eq!(ast.node_count(), count_before, "JSON renderer must not mutate AST");
        Ok(())
    }

    #[test]
    fn test_all_three_renderers_produce_output() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["-", "10", "5"])?;
        let infix = InfixRenderer.render(&ast)?;
        let sexpr = SExprRenderer.render(&ast)?;
        let json = JsonRenderer.render(&ast)?;
        assert!(!infix.is_empty() && !sexpr.is_empty() && !json.is_empty());
        Ok(())
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

    // ============================================================
    // Stress / invariant tests for Tier 2 renderers
    // ============================================================

    /// INVARIANT: rendering does not mutate the AST (takes &Ast).
    /// Observable via node_count unchanged.
    #[test]
    fn invariant_render_does_not_mutate_ast() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "1", "2"])?;
        let before_count = ast.node_count();
        let _ = InfixRenderer.render(&ast)?;
        let _ = SExprRenderer.render(&ast)?;
        let _ = JsonRenderer.render(&ast)?;
        let after_count = ast.node_count();
        assert_eq!(before_count, after_count,
            "rendering mutated the AST: {} -> {}", before_count, after_count);
        Ok(())
    }

    /// INVARIANT: render on empty AST errors with EmptyAst.
    #[test]
    fn invariant_render_empty_ast_errors() {
        let ast = Ast::new();
        assert!(InfixRenderer.render(&ast).is_err());
        assert!(SExprRenderer.render(&ast).is_err());
        assert!(JsonRenderer.render(&ast).is_err());
    }

    /// INVARIANT: All renderers produce non-empty output for well-formed ASTs.
    #[test]
    fn invariant_renderers_nonempty_for_valid_ast() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let asts: Vec<_> = [
            vec!["1"],
            vec!["+", "1", "2"],
            vec!["*", "+", "1", "2", "3"],
        ]
        .iter()
        .map(|tokens| gen.generate_from_tokens(tokens).unwrap())
        .collect();

        for ast in &asts {
            let infix = InfixRenderer.render(ast)?;
            let sexpr = SExprRenderer.render(ast)?;
            let json = JsonRenderer.render(ast)?;
            assert!(!infix.is_empty());
            assert!(!sexpr.is_empty());
            assert!(!json.is_empty());
        }
        Ok(())
    }

    /// INVARIANT: Render is deterministic.
    #[test]
    fn invariant_render_deterministic() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        let ast = gen.generate_from_tokens(&["+", "1", "2"])?;
        let a = InfixRenderer.render(&ast)?;
        let b = InfixRenderer.render(&ast)?;
        assert_eq!(a, b, "InfixRenderer not deterministic: {} vs {}", a, b);
        Ok(())
    }

    /// INVARIANT: Infix output preserves operand order.
    #[test]
    fn invariant_infix_preserves_operand_order() -> Result<(), HdlmError> {
        let gen = ArithmeticGenerator;
        // Prefix "- 10 3" is "10 minus 3", should render as (10 - 3).
        let ast = gen.generate_from_tokens(&["-", "10", "3"])?;
        let infix = InfixRenderer.render(&ast)?;
        let idx_10 = infix.find("10").unwrap();
        let idx_3 = infix.find('3').unwrap();
        assert!(idx_10 < idx_3, "operand order not preserved: {}", infix);
        Ok(())
    }
}
