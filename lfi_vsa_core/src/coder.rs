// ============================================================
// LFI Coder — Universal Polyglot Synthesis
// Section 1.V: "The LFI agent needs a universal polyglot code
// intelligence that surpasses any single LLM."
//
// The LfiCoder takes universal programming constructs and
// synthesizes them into language-specific ASTs and code.
// It leverages the LanguageRegistry for platform knowledge
// and the SelfImproveEngine for recursive optimization.
// ============================================================

use crate::languages::constructs::UniversalConstruct;
// Unused PlatformTarget removed
use crate::languages::registry::{LanguageId, LanguageRegistry};
use crate::languages::self_improve::SelfImproveEngine;
use crate::hdlm::ast::{Ast, NodeKind};
use crate::psl::supervisor::PslSupervisor;
use crate::debuglog;
// Unused HdcError removed

/// The Coder engine for cross-platform, polyglot development.
pub struct LfiCoder {
    registry: LanguageRegistry,
    improver: SelfImproveEngine,
}

impl LfiCoder {
    /// Initialize the coder with standard registries and supervisors.
    pub fn new() -> Self {
        debuglog!("LfiCoder::new: Initializing polyglot synthesis engine");
        let registry = LanguageRegistry::new();
        let supervisor = PslSupervisor::new();
        let improver = SelfImproveEngine::new(supervisor);
        Self { registry, improver }
    }

    /// Synthesize a code block from universal constructs for a specific target.
    ///
    /// Example: Translating a "Conditional" construct into Kotlin (Android)
    /// vs. Verilog (FPGA).
    pub fn synthesize(
        &self,
        language: LanguageId,
        constructs: &[UniversalConstruct],
    ) -> Result<Ast, String> {
        debuglog!("LfiCoder::synthesize: lang={:?}, count={}", language, constructs.len());

        // 1. Verify language capability in the registry
        let meta = self.registry.get_language(&language)
            .ok_or_else(|| format!("Language {:?} not supported by registry", language))?;

        // 2. Build the forensic AST (Tier 1)
        let mut ast = Ast::new();
        let root = ast.add_node(NodeKind::Root);

        for construct in constructs {
            // Check if language supports this specific construct's paradigm
            let paradigms = construct.paradigms();
            let mut supported = false;
            for p in &paradigms {
                if meta.paradigms.contains(p) {
                    supported = true;
                    break;
                }
            }

            if !supported {
                debuglog!("LfiCoder::synthesize: WARN - {:?} not natively supported by {:?}", construct, language);
            }

            // Map UniversalConstruct to NodeKind (Tier 1)
            let kind = match construct {
                UniversalConstruct::Conditional => NodeKind::BinaryOp { operator: "IF_THEN_ELSE".to_string() },
                UniversalConstruct::VariableBinding => NodeKind::Assignment,
                UniversalConstruct::FunctionCall => NodeKind::Call { function: "INTERNAL_DISPATCH".to_string() },
                _ => NodeKind::Root, // Default placeholder
            };

            let node = ast.add_node(kind);
            ast.add_child(root, node).map_err(|e| format!("AST link failure: {:?}", e))?;
        }

        // 3. Optimize the generated structure
        debuglog!("LfiCoder::synthesize: Running recursive optimization loop...");
        let optimized_ast = self.improver.optimize(&ast)?;

        debuglog!("LfiCoder::synthesize: SUCCESS, synthesized {} semantic nodes", optimized_ast.node_count());
        Ok(optimized_ast)
    }

    /// Identifies the best language/platform for a given set of requirements.
    pub fn recommend_platform(&self, paradigms: &[crate::languages::constructs::Paradigm]) -> Vec<LanguageId> {
        debuglog!("LfiCoder::recommend_platform: requirements={:?}", paradigms);
        let mut recommendations = Vec::new();
        for p in paradigms {
            let matches = self.registry.find_by_paradigm(p.clone());
            for m in matches {
                if !recommendations.contains(&m.id) {
                    recommendations.push(m.id.clone());
                }
            }
        }
        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::constructs::Paradigm;

    #[test]
    fn test_coder_synthesize_rust() -> Result<(), String> {
        let coder = LfiCoder::new();
        let constructs = vec![
            UniversalConstruct::VariableBinding,
            UniversalConstruct::Conditional,
        ];
        let ast = coder.synthesize(LanguageId::Rust, &constructs)?;
        assert!(ast.node_count() >= 3); // Root + 2 constructs
        Ok(())
    }

    #[test]
    fn test_recommend_systems_platform() {
        let coder = LfiCoder::new();
        let recs = coder.recommend_platform(&[Paradigm::Systems]);
        // Rust and Assembly are registered as systems languages
        assert!(recs.contains(&LanguageId::Rust));
        assert!(recs.contains(&LanguageId::Assembly));
    }

    #[test]
    fn test_synthesize_unsupported_language_fails() {
        let coder = LfiCoder::new();
        // Scala is in the enum but not populated in the registry.new() yet
        let result = coder.synthesize(LanguageId::Scala, &[]);
        assert!(result.is_err());
    }
}
