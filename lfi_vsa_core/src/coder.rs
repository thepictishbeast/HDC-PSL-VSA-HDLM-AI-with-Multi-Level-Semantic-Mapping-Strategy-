// ============================================================
// LFI Coder — Universal Polyglot Code Synthesis Engine
//
// The LfiCoder translates universal programming constructs
// into language-specific ASTs and renders them as source code.
//
// Pipeline:
//   1. Accept UniversalConstruct sequence + target language
//   2. Build forensic AST (Tier 1)
//   3. Run PSL audit on the AST
//   4. Self-improve via iterative optimization
//   5. Render to source code string (Tier 2)
//
// The coder learns from its own output: successful code
// patterns are reinforced in the SelfImproveEngine.
// ============================================================

use crate::languages::constructs::UniversalConstruct;
use crate::languages::registry::{LanguageId, LanguageRegistry};
use crate::languages::self_improve::SelfImproveEngine;
use crate::hdlm::ast::{Ast, NodeKind};
use crate::psl::supervisor::PslSupervisor;
use crate::debuglog;

/// Resource constraints for code generation.
#[derive(Debug, Clone)]
pub struct ResourceConstraints {
    /// Maximum allowed memory in MB (0 = unlimited).
    pub max_memory_mb: usize,
    /// Maximum allowed CPU cores (0 = unlimited).
    pub max_cores: usize,
    /// Whether to prefer stack over heap allocation.
    pub prefer_stack: bool,
    /// Whether to optimize for binary size.
    pub minimize_binary: bool,
    /// Target execution environment.
    pub environment: ExecutionEnvironment,
}

impl Default for ResourceConstraints {
    fn default() -> Self {
        Self {
            max_memory_mb: 0,
            max_cores: 0,
            prefer_stack: false,
            minimize_binary: false,
            environment: ExecutionEnvironment::Server,
        }
    }
}

/// Target execution environment affects code generation strategy.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionEnvironment {
    /// Full server/desktop — no resource restrictions.
    Server,
    /// Mobile device — memory and battery constrained.
    Mobile,
    /// Embedded/RTOS — extreme memory and compute limits.
    Embedded,
    /// Browser/WASM — sandboxed, no system calls.
    Browser,
    /// FPGA/ASIC — hardware synthesis constraints.
    Hardware,
}

/// Generated source code output.
#[derive(Debug, Clone)]
pub struct CodeOutput {
    /// The generated source code string.
    pub source: String,
    /// The language it was generated for.
    pub language: LanguageId,
    /// The underlying AST.
    pub ast: Ast,
    /// Quality metrics from self-improvement.
    pub quality_score: f64,
    /// Warnings or notes.
    pub notes: Vec<String>,
}

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
            // Check paradigm compatibility
            let paradigms = construct.paradigms();
            let supported = paradigms.iter().any(|p| meta.paradigms.contains(p));
            if !supported {
                debuglog!("LfiCoder::synthesize: WARN - {:?} not natively supported by {:?}",
                         construct, language);
            }

            // Map UniversalConstruct to NodeKind (Tier 1)
            let kind = self.construct_to_node_kind(construct);
            let node = ast.add_node(kind);
            ast.add_child(root, node).map_err(|e| format!("AST link failure: {:?}", e))?;
        }

        // 3. Optimize the generated structure
        debuglog!("LfiCoder::synthesize: Running recursive optimization loop...");
        let optimized_ast = self.improver.optimize(&ast)?;

        debuglog!("LfiCoder::synthesize: SUCCESS, synthesized {} semantic nodes",
                 optimized_ast.node_count());
        Ok(optimized_ast)
    }

    /// Synthesize and render to source code string.
    pub fn synthesize_code(
        &self,
        language: LanguageId,
        constructs: &[UniversalConstruct],
        constraints: &ResourceConstraints,
    ) -> Result<CodeOutput, String> {
        debuglog!("LfiCoder::synthesize_code: lang={:?}, constraints={:?}",
                 language, constraints.environment);

        let ast = self.synthesize(language.clone(), constructs)?;
        let metrics = self.improver.evaluate_ast(&ast);

        // Check resource constraints
        let mut notes = Vec::new();
        if constraints.max_memory_mb > 0 && metrics.memory_cost > constraints.max_memory_mb as f64 {
            notes.push(format!(
                "WARNING: Estimated memory cost {:.1} exceeds limit {} MB",
                metrics.memory_cost, constraints.max_memory_mb
            ));
        }

        // Render AST to source code
        let source = self.render_ast(&ast, &language, constraints);

        Ok(CodeOutput {
            source,
            language,
            ast,
            quality_score: metrics.overall_score(),
            notes,
        })
    }

    /// Map a UniversalConstruct to its AST NodeKind representation.
    fn construct_to_node_kind(&self, construct: &UniversalConstruct) -> NodeKind {
        debuglog!("LfiCoder::construct_to_node_kind: {:?}", construct);
        match construct {
            UniversalConstruct::Conditional => {
                NodeKind::Conditional
            }
            UniversalConstruct::ForLoop | UniversalConstruct::WhileLoop => {
                NodeKind::Loop
            }
            UniversalConstruct::VariableBinding => {
                NodeKind::Assignment
            }
            UniversalConstruct::FunctionCall => {
                NodeKind::Call { function: "dispatch".to_string() }
            }
            UniversalConstruct::FunctionDefinition => {
                NodeKind::Block { name: "fn".to_string() }
            }
            UniversalConstruct::ErrorHandling => {
                NodeKind::Block { name: "error_handling".to_string() }
            }
            UniversalConstruct::PatternMatch => {
                NodeKind::Conditional
            }
            UniversalConstruct::Block => {
                NodeKind::Block { name: "block".to_string() }
            }
            UniversalConstruct::Lambda => {
                NodeKind::Block { name: "closure".to_string() }
            }
            UniversalConstruct::StructDefinition | UniversalConstruct::ClassDefinition => {
                NodeKind::Block { name: "type_def".to_string() }
            }
            UniversalConstruct::ThreadSpawn | UniversalConstruct::AsyncAwait => {
                NodeKind::Call { function: "spawn_async".to_string() }
            }
            UniversalConstruct::Channel => {
                NodeKind::Call { function: "channel".to_string() }
            }
            UniversalConstruct::FlowControl => {
                NodeKind::Return
            }
            _ => {
                // Default: represent as a call to a named operation
                NodeKind::Call { function: format!("{:?}", construct) }
            }
        }
    }

    /// Render an AST to source code for the given language.
    fn render_ast(&self, ast: &Ast, language: &LanguageId, constraints: &ResourceConstraints) -> String {
        debuglog!("LfiCoder::render_ast: lang={:?}", language);

        let traversal = match ast.dfs() {
            Ok(t) => t,
            Err(_) => return "// Empty AST\n".to_string(),
        };

        let mut lines = Vec::new();
        lines.push(self.render_header(language));

        for &node_id in traversal.iter().skip(1) {
            if let Some(node) = ast.get_node(node_id) {
                let line = self.render_node(&node.kind, language, constraints);
                if !line.is_empty() {
                    lines.push(line);
                }
            }
        }

        lines.push(self.render_footer(language));
        lines.join("\n")
    }

    /// Render a file header for the given language.
    fn render_header(&self, language: &LanguageId) -> String {
        match language {
            LanguageId::Rust => "// Auto-generated by LFI Coder v5.6\nuse std::io;\n".to_string(),
            LanguageId::Go => "// Auto-generated by LFI Coder v5.6\npackage main\n\nimport \"fmt\"\n".to_string(),
            LanguageId::Python => "# Auto-generated by LFI Coder v5.6\n".to_string(),
            LanguageId::TypeScript | LanguageId::JavaScript => "// Auto-generated by LFI Coder v5.6\n".to_string(),
            LanguageId::Kotlin => "// Auto-generated by LFI Coder v5.6\npackage com.lfi.generated\n".to_string(),
            LanguageId::Swift => "// Auto-generated by LFI Coder v5.6\nimport Foundation\n".to_string(),
            LanguageId::Java => "// Auto-generated by LFI Coder v5.6\npublic class Generated {\n".to_string(),
            LanguageId::Csharp => "// Auto-generated by LFI Coder v5.6\nusing System;\n\nnamespace LFI {\n".to_string(),
            LanguageId::Sql => "-- Auto-generated by LFI Coder v5.6\n".to_string(),
            LanguageId::Assembly => "; Auto-generated by LFI Coder v5.6\nsection .text\nglobal _start\n".to_string(),
            _ => "// Auto-generated by LFI Coder v5.6\n".to_string(),
        }
    }

    /// Render a file footer for the given language.
    fn render_footer(&self, language: &LanguageId) -> String {
        match language {
            LanguageId::Java => "}".to_string(),
            LanguageId::Csharp => "}".to_string(),
            _ => String::new(),
        }
    }

    /// Render a single AST node to source code.
    fn render_node(&self, kind: &NodeKind, language: &LanguageId, constraints: &ResourceConstraints) -> String {
        match kind {
            NodeKind::Assignment => self.render_assignment(language, constraints),
            NodeKind::Conditional => self.render_conditional(language),
            NodeKind::Loop => self.render_loop(language),
            NodeKind::Return => self.render_return(language),
            NodeKind::Call { function } => self.render_call(function, language),
            NodeKind::Block { name } => self.render_block(name, language),
            NodeKind::Literal { value } => value.clone(),
            _ => String::new(),
        }
    }

    fn render_assignment(&self, lang: &LanguageId, constraints: &ResourceConstraints) -> String {
        let stack_note = if constraints.prefer_stack { " // stack-allocated" } else { "" };
        match lang {
            LanguageId::Rust => format!("    let value = compute_result();{}", stack_note),
            LanguageId::Go => format!("    value := computeResult(){}", stack_note),
            LanguageId::Python => "    value = compute_result()".to_string(),
            LanguageId::Kotlin => "    val value = computeResult()".to_string(),
            LanguageId::Swift => "    let value = computeResult()".to_string(),
            LanguageId::Java => "    var value = computeResult();".to_string(),
            LanguageId::Csharp => "    var value = ComputeResult();".to_string(),
            LanguageId::TypeScript => "    const value = computeResult();".to_string(),
            LanguageId::Sql => "    SET @value = (SELECT result FROM compute);".to_string(),
            LanguageId::Assembly => "    mov eax, [result]".to_string(),
            _ => "    value = compute_result()".to_string(),
        }
    }

    fn render_conditional(&self, lang: &LanguageId) -> String {
        match lang {
            LanguageId::Rust => "    if condition {\n        // branch\n    }".to_string(),
            LanguageId::Go => "    if condition {\n        // branch\n    }".to_string(),
            LanguageId::Python => "    if condition:\n        pass".to_string(),
            LanguageId::Kotlin => "    if (condition) {\n        // branch\n    }".to_string(),
            LanguageId::Swift => "    if condition {\n        // branch\n    }".to_string(),
            LanguageId::Sql => "    CASE WHEN condition THEN result END".to_string(),
            LanguageId::Assembly => "    cmp eax, ebx\n    je .branch".to_string(),
            _ => "    if (condition) { /* branch */ }".to_string(),
        }
    }

    fn render_loop(&self, lang: &LanguageId) -> String {
        match lang {
            LanguageId::Rust => "    for item in collection.iter() {\n        // body\n    }".to_string(),
            LanguageId::Go => "    for _, item := range collection {\n        // body\n    }".to_string(),
            LanguageId::Python => "    for item in collection:\n        pass".to_string(),
            LanguageId::Kotlin => "    for (item in collection) {\n        // body\n    }".to_string(),
            LanguageId::Assembly => "    .loop:\n    ; body\n    dec ecx\n    jnz .loop".to_string(),
            _ => "    for (const item of collection) { /* body */ }".to_string(),
        }
    }

    fn render_return(&self, lang: &LanguageId) -> String {
        match lang {
            LanguageId::Rust => "    Ok(result)".to_string(),
            LanguageId::Go => "    return result, nil".to_string(),
            LanguageId::Python => "    return result".to_string(),
            LanguageId::Assembly => "    ret".to_string(),
            _ => "    return result;".to_string(),
        }
    }

    fn render_call(&self, function: &str, lang: &LanguageId) -> String {
        match lang {
            LanguageId::Rust => format!("    {}()?;", function),
            LanguageId::Go => format!("    if err := {}(); err != nil {{ return err }}", function),
            LanguageId::Python => format!("    {}()", function),
            LanguageId::Assembly => format!("    call {}", function),
            _ => format!("    {}();", function),
        }
    }

    fn render_block(&self, name: &str, lang: &LanguageId) -> String {
        match lang {
            LanguageId::Rust => format!("fn {}() -> Result<(), Box<dyn std::error::Error>> {{", name),
            LanguageId::Go => format!("func {}() error {{", name),
            LanguageId::Python => format!("def {}():", name),
            LanguageId::Kotlin => format!("fun {}() {{", name),
            LanguageId::Swift => format!("func {}() throws {{", name),
            LanguageId::Java => format!("    public void {}() {{", name),
            _ => format!("function {}() {{", name),
        }
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
        assert!(recs.contains(&LanguageId::Rust));
        assert!(recs.contains(&LanguageId::Assembly));
    }

    #[test]
    fn test_synthesize_unsupported_language_fails() {
        let coder = LfiCoder::new();
        let result = coder.synthesize(LanguageId::VisualBasic, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_rust_code() -> Result<(), String> {
        let coder = LfiCoder::new();
        let constructs = vec![
            UniversalConstruct::FunctionDefinition,
            UniversalConstruct::VariableBinding,
            UniversalConstruct::Conditional,
            UniversalConstruct::ForLoop,
            UniversalConstruct::FlowControl,
        ];
        let output = coder.synthesize_code(
            LanguageId::Rust,
            &constructs,
            &ResourceConstraints::default(),
        )?;
        assert!(output.source.contains("let value"), "Rust should use 'let'");
        assert!(output.source.contains("if condition"), "Should have conditional");
        assert!(output.source.contains("for item"), "Should have loop");
        assert!(output.quality_score > 0.0);
        Ok(())
    }

    #[test]
    fn test_render_go_code() -> Result<(), String> {
        let coder = LfiCoder::new();
        let constructs = vec![
            UniversalConstruct::VariableBinding,
            UniversalConstruct::Channel,
        ];
        let output = coder.synthesize_code(
            LanguageId::Go,
            &constructs,
            &ResourceConstraints::default(),
        )?;
        assert!(output.source.contains("package main"), "Go should have package");
        assert!(output.source.contains(":="), "Go should use ':='");
        Ok(())
    }

    #[test]
    fn test_render_python_code() -> Result<(), String> {
        let coder = LfiCoder::new();
        let constructs = vec![
            UniversalConstruct::FunctionDefinition,
            UniversalConstruct::VariableBinding,
        ];
        let output = coder.synthesize_code(
            LanguageId::Python,
            &constructs,
            &ResourceConstraints::default(),
        )?;
        assert!(output.source.contains("def "), "Python should use 'def'");
        assert!(output.source.contains("compute_result()"), "Python should have call");
        Ok(())
    }

    #[test]
    fn test_resource_constraints_warning() -> Result<(), String> {
        let coder = LfiCoder::new();
        let mut big_constructs = Vec::new();
        for _ in 0..100 {
            big_constructs.push(UniversalConstruct::VariableBinding);
        }
        let constraints = ResourceConstraints {
            max_memory_mb: 1,
            environment: ExecutionEnvironment::Embedded,
            ..Default::default()
        };
        let output = coder.synthesize_code(LanguageId::Rust, &big_constructs, &constraints)?;
        // With 100 nodes, memory cost should exceed 1MB limit
        assert!(output.notes.iter().any(|n| n.contains("WARNING")),
               "Should warn about resource constraints");
        Ok(())
    }
}
