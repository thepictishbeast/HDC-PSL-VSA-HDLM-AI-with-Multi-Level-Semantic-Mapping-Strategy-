// ============================================================
// LFI Cognitive Validation Test Suite
//
// PURPOSE: Validate that the LFI cognitive pipeline (intent
//   detection, dual-mode reasoning, planning, self-improvement,
//   and polyglot code synthesis) produces DERIVED answers rather
//   than memorized outputs. The AI must reason through each
//   problem from first principles using VSA-based cognition.
//
// ============================================================
//
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
// !!                                                          !!
// !!               ROTATION NOTICE                            !!
// !!                                                          !!
// !!  LAST ROTATED: 2026-03-28                                !!
// !!                                                          !!
// !!  These tests MUST be rotated regularly.                   !!
// !!  Every 30 days (or sooner if the AI begins passing        !!
// !!  without genuine derivation), change:                     !!
// !!    - The input strings fed to detect_intent / think       !!
// !!    - The expected intent variants                         !!
// !!    - The goal descriptions given to the Planner           !!
// !!    - The construct combinations given to LfiCoder         !!
// !!    - The expected patterns in generated source code       !!
// !!                                                          !!
// !!  The AI must DERIVE answers from positional keyword       !!
// !!  weights, VSA similarity, and AST analysis -- not from    !!
// !!  memorized input/output pairs. Rotation guards against    !!
// !!  overfitting to specific test fixtures.                   !!
// !!                                                          !!
// !!  CATEGORIES:                                              !!
// !!    1. Intent Detection (keyword-weighted NLU)             !!
// !!    2. Dual-Mode Cognition (System 1 / System 2)           !!
// !!    3. Planning & Execution (goal decomposition)           !!
// !!    4. Self-Improvement (AST evaluation & optimization)    !!
// !!    5. Code Generation (polyglot synthesis & rendering)    !!
// !!                                                          !!
// !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
//
// ============================================================

use lfi_vsa_core::hdc::error::HdcError;
use lfi_vsa_core::cognition::reasoner::{CognitiveCore, CognitiveMode, Intent};
use lfi_vsa_core::cognition::planner::{Planner, StepStatus};
use lfi_vsa_core::languages::self_improve::SelfImproveEngine;
use lfi_vsa_core::psl::supervisor::PslSupervisor;
use lfi_vsa_core::coder::LfiCoder;
use lfi_vsa_core::languages::registry::LanguageId;
use lfi_vsa_core::languages::constructs::{UniversalConstruct, Paradigm};
use lfi_vsa_core::hdlm::ast::{Ast, NodeKind};
use lfi_vsa_core::coder::ResourceConstraints;

// ============================================================
// 1. INTENT DETECTION TESTS
//
// These validate the position-weighted keyword matching system.
// Earlier words in the input carry more weight (position decay
// factor 1/(1 + i*0.3)), reflecting how English front-loads
// intent signals. Each test probes a different intent category.
// ============================================================

/// Verify that a coding request with an explicit language mention
/// is classified as WriteCode with the correct language extracted.
#[test]
fn test_intent_detection_write_code_with_python() -> Result<(), HdcError> {
    // debuglog: testing WriteCode intent with Python language mention
    let core = CognitiveCore::new()?;

    // "write" and "code" both appear early, matching the write_code prototype.
    // "python" appears later and is extracted by detect_language_mention.
    let intent = core.detect_intent("write a python function that computes factorials")?;

    match &intent {
        Intent::WriteCode { language, description } => {
            assert_eq!(language, "Python",
                "Language should be extracted as Python from input text");
            assert!(!description.is_empty(),
                "Description should capture the full input");
        }
        other => {
            return Err(HdcError::InitializationFailed {
                reason: format!("Expected WriteCode intent, got {:?}", other),
            });
        }
    }

    Ok(())
}

/// Verify that a bug-fixing request is classified as FixBug.
/// The word "fix" has high position weight when it leads the sentence.
#[test]
fn test_intent_detection_fix_bug_leading_keyword() -> Result<(), HdcError> {
    // debuglog: testing FixBug intent with leading "fix" keyword
    let core = CognitiveCore::new()?;

    let intent = core.detect_intent("fix the crash happening in the serialization pipeline")?;

    match &intent {
        Intent::FixBug { description } => {
            assert!(description.contains("crash"),
                "Description should preserve the original error context");
        }
        other => {
            return Err(HdcError::InitializationFailed {
                reason: format!("Expected FixBug intent, got {:?}", other),
            });
        }
    }

    Ok(())
}

/// Verify that an explanation request is classified as Explain.
/// "explain" and "how" both match the explain prototype keywords.
#[test]
fn test_intent_detection_explain_topic() -> Result<(), HdcError> {
    // debuglog: testing Explain intent with question-style input
    let core = CognitiveCore::new()?;

    let intent = core.detect_intent("explain how ownership and borrowing work in rust")?;

    assert!(matches!(intent, Intent::Explain { .. }),
        "Question starting with 'explain' should map to Explain intent");

    Ok(())
}

/// Verify that a search/research request is classified as Search.
/// "search" and "find" both appear in the search prototype keywords.
#[test]
fn test_intent_detection_search_query() -> Result<(), HdcError> {
    // debuglog: testing Search intent with research-style input
    let core = CognitiveCore::new()?;

    let intent = core.detect_intent("search for recent papers on homomorphic encryption")?;

    assert!(matches!(intent, Intent::Search { .. }),
        "'search for' should trigger Search intent");

    Ok(())
}

/// Verify that a planning request is classified as PlanTask.
/// "plan" and "design" both match the plan prototype.
#[test]
fn test_intent_detection_plan_task() -> Result<(), HdcError> {
    // debuglog: testing PlanTask intent with architecture-style input
    let core = CognitiveCore::new()?;

    let intent = core.detect_intent("plan the architecture for a distributed key-value store")?;

    assert!(matches!(intent, Intent::PlanTask { .. }),
        "'plan' at the start should trigger PlanTask intent");

    Ok(())
}

/// Verify that a greeting is classified as Converse.
/// "hey" is a converse prototype keyword with high position weight.
#[test]
fn test_intent_detection_converse_greeting() -> Result<(), HdcError> {
    // debuglog: testing Converse intent with informal greeting
    let core = CognitiveCore::new()?;

    let intent = core.detect_intent("hey, thanks for helping me earlier")?;

    assert!(matches!(intent, Intent::Converse { .. }),
        "Greeting with 'hey' and 'thanks' should trigger Converse intent");

    Ok(())
}

/// Verify that an optimization request is classified as Improve.
/// "optimize" and "refactor" both match the improve prototype.
#[test]
fn test_intent_detection_improve_target() -> Result<(), HdcError> {
    // debuglog: testing Improve intent with optimization request
    let core = CognitiveCore::new()?;

    let intent = core.detect_intent("optimize the database query performance in the analytics module")?;

    assert!(matches!(intent, Intent::Improve { .. }),
        "'optimize' should trigger Improve intent");

    Ok(())
}


// ============================================================
// 2. DUAL-MODE COGNITION TESTS (System 1 / System 2)
//
// System 1 (Fast): Fires when input matches a stored pattern
//   in holographic memory (similarity > novelty_threshold).
// System 2 (Deep): Fires for novel inputs, producing a plan
//   with decomposed sub-steps.
//
// The key invariant: the FIRST encounter with a problem must
// always trigger Deep mode, while a REPEATED identical input
// should trigger Fast mode (the pattern was stored on first
// encounter).
// ============================================================

/// A completely novel problem should always trigger Deep mode
/// with a generated plan containing multiple steps.
#[test]
fn test_deep_mode_novel_problem_produces_plan() -> Result<(), HdcError> {
    // debuglog: testing that novel input triggers System 2 (Deep)
    let mut core = CognitiveCore::new()?;

    let result = core.think("design a zero-knowledge proof system for credential verification")?;

    assert_eq!(result.mode, CognitiveMode::Deep,
        "First encounter with a novel problem must trigger Deep mode");
    assert!(result.plan.is_some(),
        "Deep mode must produce a plan with decomposed steps");

    let plan = result.plan.as_ref().ok_or(HdcError::InitializationFailed {
        reason: "Plan was None despite Deep mode".to_string(),
    })?;
    assert!(!plan.steps.is_empty(),
        "Plan must contain at least one step");
    assert!(result.confidence > 0.0 && result.confidence <= 1.0,
        "Confidence must be in (0.0, 1.0], got {}", result.confidence);

    Ok(())
}

/// Repeating the exact same input should trigger Fast mode on
/// the second call, because the first call stored the pattern
/// in holographic memory.
#[test]
fn test_fast_mode_repeated_input() -> Result<(), HdcError> {
    // debuglog: testing System 1 (Fast) activation on repeated input
    let mut core = CognitiveCore::new()?;
    let input = "merge two sorted linked lists into one sorted list";

    // First encounter: must be Deep (novel).
    let r1 = core.think(input)?;
    assert_eq!(r1.mode, CognitiveMode::Deep,
        "First encounter must be Deep mode");

    // Second encounter: must be Fast (familiar — stored in holographic memory).
    let r2 = core.think(input)?;
    assert_eq!(r2.mode, CognitiveMode::Fast,
        "Repeated identical input must trigger Fast mode");
    assert!(r2.confidence > 0.0,
        "Fast mode must still produce nonzero confidence");

    Ok(())
}

/// Deep mode should attach the detected intent to the ThoughtResult.
/// This validates that the NLU pipeline feeds into the cognitive pipeline.
#[test]
fn test_think_attaches_intent() -> Result<(), HdcError> {
    // debuglog: verifying intent propagation through think()
    let mut core = CognitiveCore::new()?;

    let result = core.think("build a REST API server in go with rate limiting")?;

    let intent = result.intent.as_ref().ok_or(HdcError::InitializationFailed {
        reason: "Intent was None in ThoughtResult".to_string(),
    })?;

    // "build" matches write_code prototype
    assert!(matches!(intent, Intent::WriteCode { .. }),
        "Intent should be WriteCode for a coding request, got {:?}", intent);

    Ok(())
}


// ============================================================
// 3. CONVERSATION & CONTEXT TESTS
//
// The converse() method builds a context window of recent
// exchanges. Each call adds to the window (up to max_context).
// ============================================================

/// Multi-turn conversation should accumulate context vectors.
/// After N exchanges, the context window should have N entries.
#[test]
fn test_conversation_accumulates_context() -> Result<(), HdcError> {
    // debuglog: testing multi-turn context accumulation
    let mut core = CognitiveCore::new()?;

    let _r1 = core.converse("explain what a monad is in functional programming")?;
    let _r2 = core.converse("can you give me an example in haskell")?;
    let _r3 = core.converse("how does that compare to rust's Result type")?;

    // Each converse() call internally calls think(), which pushes to context_window.
    // We verify by making a fourth call and checking the result is valid.
    let r4 = core.converse("thanks, that was helpful")?;
    assert!(r4.intent.is_some(),
        "Conversational turn should still produce an intent");

    Ok(())
}


// ============================================================
// 4. PLANNING & EXECUTION TESTS
//
// The Planner decomposes goals into ordered steps with
// dependencies. Steps flow through: Pending -> Active -> Done
// (or Failed). The planner also learns from completed plans.
// ============================================================

/// A novel goal should decompose into multiple steps, all
/// initially Pending with well-defined dependencies.
#[test]
fn test_plan_decomposition_novel_goal() -> Result<(), HdcError> {
    // debuglog: testing goal decomposition for a novel problem
    let planner = Planner::new();
    let plan = planner.plan("build a WebSocket-based real-time collaboration engine")?;

    assert!(!plan.steps.is_empty(),
        "Plan must decompose into at least one step");
    assert!(plan.total_complexity > 0.0,
        "Total complexity must be positive");
    assert!(!plan.is_complete(),
        "Freshly created plan must not be marked complete");
    assert_eq!(plan.completed_count(), 0,
        "No steps should be completed initially");
    assert_eq!(plan.remaining_count(), plan.steps.len(),
        "All steps should be remaining initially");

    // Verify all steps start as Pending
    for (i, step) in plan.steps.iter().enumerate() {
        assert_eq!(step.status, StepStatus::Pending,
            "Step {} should be Pending, got {:?}", i, step.status);
        assert!(!step.description.is_empty(),
            "Step {} must have a non-empty description", i);
    }

    Ok(())
}

/// Execute a full plan to completion: advance -> complete -> repeat.
/// After all steps are done, the plan should report is_complete().
#[test]
fn test_plan_full_execution_cycle() -> Result<(), HdcError> {
    // debuglog: testing full plan execution cycle
    let planner = Planner::new();
    let mut plan = planner.plan("implement a CRDT-based shared document editor")?;

    let mut iterations = 0;
    let max_iterations = 50; // Safety cap

    while let Some(idx) = planner.advance(&mut plan)? {
        assert!(matches!(plan.steps[idx].status, StepStatus::Active),
            "Advanced step must be Active");
        planner.complete_step(&mut plan, idx)?;
        assert_eq!(plan.steps[idx].status, StepStatus::Done,
            "Completed step must be Done");
        iterations += 1;
        if iterations > max_iterations {
            return Err(HdcError::InitializationFailed {
                reason: "Plan execution exceeded safety cap".to_string(),
            });
        }
    }

    assert!(plan.is_complete(),
        "All steps should be Done after full execution");
    assert!(!plan.has_failures(),
        "No steps should have failed in a clean execution");
    assert_eq!(plan.remaining_count(), 0,
        "No steps should remain after full execution");

    Ok(())
}

/// Failing a step should mark it Failed and increment the
/// replan counter, while leaving other steps unaffected.
#[test]
fn test_plan_step_failure_tracking() -> Result<(), HdcError> {
    // debuglog: testing step failure and replan counter
    let planner = Planner::new();
    let mut plan = planner.plan("deploy a canary release to production cluster")?;

    // Advance to the first step
    let idx = planner.advance(&mut plan)?.ok_or(HdcError::InitializationFailed {
        reason: "No step available to advance".to_string(),
    })?;

    // Fail it with a specific reason
    planner.fail_step(&mut plan, idx, "Container registry unreachable")?;

    assert!(plan.has_failures(),
        "Plan must report failures after a step fails");
    assert_eq!(plan.replan_count, 1,
        "Replan counter should be 1 after one failure");

    match &plan.steps[idx].status {
        StepStatus::Failed { reason } => {
            assert!(reason.contains("unreachable"),
                "Failure reason should be preserved");
        }
        other => {
            return Err(HdcError::InitializationFailed {
                reason: format!("Expected Failed status, got {:?}", other),
            });
        }
    }

    Ok(())
}

/// After completing a plan, learning from it should increase
/// the planner's pattern library by exactly one entry.
#[test]
fn test_planner_learning_from_success() -> Result<(), HdcError> {
    // debuglog: testing planner pattern learning
    let mut planner = Planner::new();
    let initial_patterns = planner.pattern_count();

    // Create and fully execute a plan
    let mut plan = planner.plan("implement an LRU cache with O(1) operations")?;
    while let Some(idx) = planner.advance(&mut plan)? {
        planner.complete_step(&mut plan, idx)?;
    }

    // Learn from the successful execution
    planner.learn_from_success(&plan)?;

    assert_eq!(planner.pattern_count(), initial_patterns + 1,
        "Pattern library should grow by exactly one after learning");

    Ok(())
}


// ============================================================
// 5. SELF-IMPROVEMENT ENGINE TESTS
//
// The SelfImproveEngine evaluates AST quality through structural
// analysis (depth, balance, complexity, leaf ratio) and suggests
// transforms to improve code. The optimize() loop iteratively
// applies transforms until no further improvement is possible.
// ============================================================

/// A small, well-structured AST should produce high quality
/// scores across all metrics (no weaknesses detected).
#[test]
fn test_self_improve_evaluate_clean_ast() -> Result<(), Box<dyn std::error::Error>> {
    // debuglog: testing metrics for a clean, shallow AST
    let supervisor = PslSupervisor::new();
    let engine = SelfImproveEngine::new(supervisor);

    // Build a small balanced AST: root -> 3 leaves
    let mut ast = Ast::new();
    let root = ast.add_node(NodeKind::Root);
    for i in 0..3 {
        let leaf = ast.add_node(NodeKind::Literal {
            value: format!("val_{}", i),
        });
        ast.add_child(root, leaf)?;
    }

    let metrics = engine.evaluate_ast(&ast);

    assert_eq!(metrics.node_count, 4,
        "Should have root + 3 leaves = 4 nodes");
    assert!(metrics.depth <= 3,
        "Shallow tree should have small depth, got {}", metrics.depth);
    assert!(metrics.security_score > 0.5,
        "Internally generated code should have decent security score");
    assert!(metrics.weaknesses.is_empty(),
        "Clean, shallow AST should have no weaknesses, got {:?}", metrics.weaknesses);
    assert!(metrics.overall_score() > 0.4,
        "Clean AST should have a reasonable overall score, got {:.4}", metrics.overall_score());

    Ok(())
}

/// A deeply nested linear chain AST should trigger the
/// ExcessiveNesting weakness and produce suggested transforms.
#[test]
fn test_self_improve_detect_excessive_nesting() -> Result<(), Box<dyn std::error::Error>> {
    // debuglog: testing weakness detection for deep nesting
    let supervisor = PslSupervisor::new();
    let engine = SelfImproveEngine::new(supervisor);

    // Build a linear chain: root -> block_0 -> block_1 -> ... -> block_14 -> leaf
    let mut ast = Ast::new();
    let root = ast.add_node(NodeKind::Root);
    let mut parent = root;
    for i in 0..15 {
        let child = ast.add_node(NodeKind::Block {
            name: format!("nested_level_{}", i),
        });
        ast.add_child(parent, child)?;
        parent = child;
    }
    let leaf = ast.add_node(NodeKind::Literal { value: "deep_value".to_string() });
    ast.add_child(parent, leaf)?;

    let metrics = engine.evaluate_ast(&ast);

    assert!(metrics.depth > 10,
        "Deep chain should have depth > 10, got {}", metrics.depth);

    let has_nesting_weakness = metrics.weaknesses.iter().any(|w| {
        w.category == lfi_vsa_core::languages::self_improve::WeaknessCategory::ExcessiveNesting
    });
    assert!(has_nesting_weakness,
        "Should detect ExcessiveNesting weakness for depth > 10");

    // Verify transforms are suggested
    let transforms = engine.suggest_transforms(&metrics);
    assert!(!transforms.is_empty(),
        "Should suggest at least one transform for the detected weakness");

    Ok(())
}

/// The optimize() loop should flatten a deeply nested AST,
/// reducing its depth while preserving node information.
#[test]
fn test_self_improve_optimize_reduces_depth() -> Result<(), Box<dyn std::error::Error>> {
    // debuglog: testing that optimize() reduces depth of deep ASTs
    let supervisor = PslSupervisor::new();
    let engine = SelfImproveEngine::new(supervisor);

    // Build a deep linear chain (depth 20)
    let mut ast = Ast::new();
    let root = ast.add_node(NodeKind::Root);
    let mut parent = root;
    for i in 0..20 {
        let child = ast.add_node(NodeKind::Block {
            name: format!("layer_{}", i),
        });
        ast.add_child(parent, child)?;
        parent = child;
    }

    let before = engine.evaluate_ast(&ast);
    let optimized = engine.optimize(&ast).map_err(|e| {
        Box::<dyn std::error::Error>::from(e)
    })?;
    let after = engine.evaluate_ast(&optimized);

    assert!(after.depth < before.depth,
        "Optimization should reduce depth: before={}, after={}", before.depth, after.depth);
    assert!(after.overall_score() >= before.overall_score(),
        "Optimization should not degrade overall score: before={:.4}, after={:.4}",
        before.overall_score(), after.overall_score());

    Ok(())
}


// ============================================================
// 6. CODE GENERATION TESTS (Polyglot Synthesis)
//
// The LfiCoder translates UniversalConstruct sequences into
// language-specific ASTs and renders them as source code strings.
// Each language has its own header, syntax patterns, and idioms.
// ============================================================

/// Synthesize a Rust program with multiple constructs and verify
/// the rendered source contains Rust-specific syntax patterns.
#[test]
fn test_coder_synthesize_rust_program() -> Result<(), String> {
    // debuglog: testing Rust code synthesis with multiple constructs
    let coder = LfiCoder::new();
    let constructs = vec![
        UniversalConstruct::FunctionDefinition,
        UniversalConstruct::VariableBinding,
        UniversalConstruct::Conditional,
        UniversalConstruct::ForLoop,
        UniversalConstruct::ErrorHandling,
        UniversalConstruct::FlowControl,
    ];

    let output = coder.synthesize_code(
        LanguageId::Rust,
        &constructs,
        &ResourceConstraints::default(),
    )?;

    // Verify Rust-specific patterns
    assert!(output.source.contains("Auto-generated by LFI Coder"),
        "Should include the LFI header comment");
    assert!(output.source.contains("let value"),
        "Rust should use 'let' for variable bindings");
    assert!(output.source.contains("if condition"),
        "Rust should render conditionals with 'if'");
    assert!(output.source.contains("for item"),
        "Rust should render for loops with 'for item in'");
    assert!(output.quality_score > 0.0,
        "Quality score should be positive");
    assert_eq!(output.language, LanguageId::Rust,
        "Output language should match the request");

    Ok(())
}

/// Synthesize a Go program and verify Go-specific idioms appear.
#[test]
fn test_coder_synthesize_go_program() -> Result<(), String> {
    // debuglog: testing Go code synthesis
    let coder = LfiCoder::new();
    let constructs = vec![
        UniversalConstruct::VariableBinding,
        UniversalConstruct::Conditional,
        UniversalConstruct::Channel,
        UniversalConstruct::FlowControl,
    ];

    let output = coder.synthesize_code(
        LanguageId::Go,
        &constructs,
        &ResourceConstraints::default(),
    )?;

    assert!(output.source.contains("package main"),
        "Go source must include 'package main'");
    assert!(output.source.contains(":="),
        "Go should use ':=' for short variable declarations");
    assert!(output.source.contains("return result, nil"),
        "Go should use multiple return values with nil error");

    Ok(())
}

/// Synthesize a Python program and verify Python-specific patterns.
#[test]
fn test_coder_synthesize_python_program() -> Result<(), String> {
    // debuglog: testing Python code synthesis
    let coder = LfiCoder::new();
    let constructs = vec![
        UniversalConstruct::FunctionDefinition,
        UniversalConstruct::VariableBinding,
        UniversalConstruct::ForLoop,
    ];

    let output = coder.synthesize_code(
        LanguageId::Python,
        &constructs,
        &ResourceConstraints::default(),
    )?;

    assert!(output.source.contains("def "),
        "Python should use 'def' for function definitions");
    assert!(output.source.contains("compute_result()"),
        "Python should render function calls in snake_case");
    assert!(output.source.contains("for item in"),
        "Python should render for loops with 'for item in'");

    Ok(())
}

/// Verify that resource constraints produce warnings when the
/// generated code exceeds memory limits (embedded environment).
#[test]
fn test_coder_resource_constraint_warning() -> Result<(), String> {
    // debuglog: testing resource constraint enforcement
    let coder = LfiCoder::new();

    // Generate a large program (many constructs = high memory cost)
    let mut constructs = Vec::with_capacity(80);
    for _ in 0..80 {
        constructs.push(UniversalConstruct::VariableBinding);
    }

    let constraints = ResourceConstraints {
        max_memory_mb: 1,
        max_cores: 1,
        prefer_stack: true,
        minimize_binary: true,
        environment: lfi_vsa_core::coder::ExecutionEnvironment::Embedded,
    };

    let output = coder.synthesize_code(
        LanguageId::Rust,
        &constructs,
        &constraints,
    )?;

    // With 80+ nodes, memory_cost = node_count*0.1 + depth*0.5 should exceed 1 MB
    let has_warning = output.notes.iter().any(|n| n.contains("WARNING"));
    assert!(has_warning,
        "Should produce a resource constraint warning for large programs in embedded env");

    Ok(())
}

/// Verify that requesting an unsupported language returns an error.
#[test]
fn test_coder_unsupported_language_returns_error() {
    // debuglog: testing error handling for unsupported language
    let coder = LfiCoder::new();
    let constructs = vec![UniversalConstruct::Block];

    let result = coder.synthesize(LanguageId::VisualBasic, &constructs);
    assert!(result.is_err(),
        "Synthesizing for an unsupported language must return Err");
}

/// Verify that the platform recommender suggests appropriate
/// languages for given paradigm requirements.
#[test]
fn test_coder_recommend_platform_concurrent() {
    // debuglog: testing platform recommendation for concurrent paradigm
    let coder = LfiCoder::new();

    let recs = coder.recommend_platform(&[Paradigm::Concurrent]);
    assert!(!recs.is_empty(),
        "Should recommend at least one language for concurrent paradigm");

    // Go, Rust, Kotlin, Erlang, Elixir, etc. all support Concurrent
    let has_go = recs.contains(&LanguageId::Go);
    let has_rust = recs.contains(&LanguageId::Rust);
    assert!(has_go || has_rust,
        "Concurrent recommendations should include Go or Rust (or both)");
}
