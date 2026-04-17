//! End-to-end integration tests for the LFI reasoning pipeline.
//!
//! These tests exercise the full chain: encode → reason → audit → explain.
//! They verify that all subsystems compose correctly.

use lfi_vsa_core::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use lfi_vsa_core::hdc::vector::BipolarVector;
use lfi_vsa_core::psl::supervisor::PslSupervisor;
use lfi_vsa_core::psl::axiom::{DimensionalityAxiom, AuditTarget};
use lfi_vsa_core::cognition::mcts::MctsEngine;
use lfi_vsa_core::cognition::reasoner::CognitiveCore;
use lfi_vsa_core::cognition::active_inference::{ActiveInferenceCore, InferenceOutcome};
use lfi_vsa_core::reasoning_provenance::{ProvenanceEngine, ProvenanceKind, InferenceSource};
use lfi_vsa_core::crypto_epistemology::EpistemicLedger;
use lfi_vsa_core::identity::{IdentityProver, IdentityKind};

#[test]
fn test_full_reasoning_pipeline() {
    // 1. ENCODE: Create a problem vector.
    let problem = HyperMemory::from_string("implement secure authentication", DIM_PROLETARIAT);
    let goal = HyperMemory::from_string("working auth system with tests", DIM_PROLETARIAT);

    // 2. REASON: Use MCTS to deliberate.
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));

    let mut engine = MctsEngine::new(problem, goal);
    engine.enable_provenance();
    let result = engine.deliberate(10, &supervisor);
    assert!(result.is_ok(), "MCTS deliberation should succeed");

    // 3. AUDIT: Verify the result passes PSL.
    let result_vec = result.unwrap();
    let raw_bits = result_vec.export_raw_bitvec();
    let bv = BipolarVector::from_bitvec(raw_bits).expect("bitvec conversion");
    let target = AuditTarget::Vector(bv);
    let verdict = supervisor.audit(&target).expect("audit should succeed");
    assert!(verdict.confidence > 0.0, "Result should have non-zero confidence");

    // 4. EXPLAIN: Provenance should have traces.
    let arena = engine.provenance().expect("provenance enabled");
    assert!(arena.len() >= 10, "10 iterations should produce 10+ traces");
}

#[test]
fn test_cognitive_core_with_provenance() {
    let mut core = CognitiveCore::new().expect("CognitiveCore init");
    let mut provenance = ProvenanceEngine::new();

    // Think about a novel problem.
    let (result, _trace_id) = core.think_with_provenance(
        "design a zero-knowledge proof system for voter verification",
        &mut provenance.arena,
        None,
        Some(42),
    ).expect("think should succeed");

    // Should use deep mode for a novel problem.
    assert!(result.confidence > 0.0);

    // Provenance should be traceable.
    let explanation = provenance.explain_conclusion(42);
    assert_eq!(explanation.kind, ProvenanceKind::TracedDerivation);
}

#[test]
fn test_active_inference_with_provenance() {
    let model = HyperMemory::new(DIM_PROLETARIAT);
    let mut core = ActiveInferenceCore::new(model.clone());
    core.set_target(model.clone());

    let mut provenance = ProvenanceEngine::new();
    let (outcome, _trace) = core.step_with_provenance(
        &model,
        &mut provenance.arena,
        None,
    ).expect("step should succeed");

    // At equilibrium, outcome should be Equilibrium.
    assert!(matches!(outcome, InferenceOutcome::Equilibrium { .. }));
    assert!(provenance.arena.len() >= 1);
}

#[test]
fn test_epistemic_ledger_with_provenance() {
    let mut ledger = EpistemicLedger::new();
    let mut provenance = ProvenanceEngine::new();
    let belief = HyperMemory::generate_seed(DIM_PROLETARIAT);

    // Record a traced derivation.
    provenance.arena.record_step(
        None,
        InferenceSource::System2Deliberation { iterations: 50 },
        vec!["deep analysis".into()],
        0.95,
        Some(100),
        "Thorough derivation of belief".into(),
        5000,
    );

    // Commit with provenance — should be tagged TRACED.
    let (idx, kind) = ledger.commit_belief_with_provenance(
        &belief, "tested_belief", &provenance, 100,
    );
    assert_eq!(kind, ProvenanceKind::TracedDerivation);
    assert!(ledger.commitments[idx].label.contains("TRACED"));

    // Commit without provenance — should be tagged RECONSTRUCTED.
    let (idx2, kind2) = ledger.commit_belief_with_provenance(
        &belief, "untraced_belief", &provenance, 999, // No trace for this
    );
    assert!(matches!(kind2, ProvenanceKind::ReconstructedRationalization { .. }));
    assert!(ledger.commitments[idx2].label.contains("RECONSTRUCTED"));
}

#[test]
fn test_identity_gates_agent_access() {
    let proof = IdentityProver::commit("Test User", "cred123", "lic456", "pass789", IdentityKind::Sovereign);
    assert!(IdentityProver::verify(&proof, "Test User", "cred123", "lic456", "pass789"));
    assert!(!IdentityProver::verify(&proof, "Fake User", "cred123", "lic456", "pass789"));

    // Password-only verification.
    assert!(IdentityProver::verify_password(&proof, "pass789"));
    assert!(!IdentityProver::verify_password(&proof, "wrong"));
}

#[test]
fn test_psl_governance_chain() {
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));
    supervisor.register_axiom(Box::new(lfi_vsa_core::psl::axiom::EntropyAxiom::default()));

    let vec = BipolarVector::new_random().expect("random vector");
    let target = AuditTarget::Vector(vec);

    // Multi-axiom audit should compose correctly.
    let verdict = supervisor.audit(&target).expect("audit should succeed");
    assert!(verdict.confidence > 0.0);

    // With provenance.
    let mut arena = lfi_vsa_core::reasoning_provenance::TraceArena::new();
    let (verdict2, traces) = supervisor.audit_with_provenance(&target, &mut arena, None)
        .expect("provenance audit should succeed");
    assert_eq!(traces.len(), 2, "Two axioms should produce two traces");
    assert!(verdict2.confidence > 0.0);
}

#[test]
fn test_cross_subsystem_vector_compatibility() {
    // Verify that vectors created by different subsystems are compatible.
    let hm = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let bv = BipolarVector::new_random().expect("random BV");

    // HyperMemory → BitVec → BipolarVector round trip.
    let bits = hm.export_raw_bitvec();
    let bv_from_hm = BipolarVector::from_bitvec(bits).expect("conversion");
    assert_eq!(bv_from_hm.dim(), DIM_PROLETARIAT);

    // Both should be auditable by PSL.
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));

    let v1 = supervisor.audit(&AuditTarget::Vector(bv)).expect("audit BV");
    let v2 = supervisor.audit(&AuditTarget::Vector(bv_from_hm)).expect("audit HM→BV");
    assert!(v1.confidence > 0.0 && v2.confidence > 0.0);
}

/// Self-play thesis-antithesis-synthesis loop with provenance.
#[test]
fn test_self_play_with_provenance() {
    use lfi_vsa_core::reasoning_provenance::{ProvenanceEngine, ProvenanceKind, InferenceSource};

    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));

    let mut provenance = ProvenanceEngine::new();
    let conclusion_id = 1000;

    // THESIS: MCTS generates a strategic move.
    let root = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let mut engine = MctsEngine::new(root, goal);
    engine.enable_provenance();
    let thesis = engine.deliberate(10, &supervisor).expect("thesis should succeed");

    // Record thesis in provenance.
    let thesis_trace = provenance.arena.record_step(
        None,
        InferenceSource::SelfPlayEpisode { generation: 1 },
        vec!["thesis_strategy".into()],
        0.8, None, "Self-play thesis generated".into(), 0,
    );

    // ANTITHESIS: PSL audits the thesis with provenance.
    let raw_bits = thesis.export_raw_bitvec();
    let thesis_bv = BipolarVector::from_bitvec(raw_bits).expect("conversion");
    let target = AuditTarget::Vector(thesis_bv);
    let mut audit_arena = lfi_vsa_core::reasoning_provenance::TraceArena::new();
    let (verdict, _audit_traces) = supervisor.audit_with_provenance(
        &target, &mut audit_arena, Some(thesis_trace),
    ).expect("audit should succeed");

    let antithesis_trace = provenance.arena.record_step(
        Some(thesis_trace),
        InferenceSource::PslAxiomEvaluation {
            axiom_id: "DimensionalityAxiom".into(),
            relevance: 1.0,
        },
        vec!["thesis_audited".into()],
        verdict.confidence,
        None,
        format!("PSL audit: conf={:.4}", verdict.confidence),
        0,
    );

    // SYNTHESIS: If both agree, forge the hardened strategy.
    let _synthesis_trace = provenance.arena.record_step(
        Some(antithesis_trace),
        InferenceSource::SelfPlayEpisode { generation: 1 },
        vec!["thesis".into(), "antithesis".into()],
        0.9,
        Some(conclusion_id),
        "Synthesis: hardened strategy forged".into(),
        0,
    );

    // Verify the full chain is traceable.
    let explanation = provenance.explain_conclusion(conclusion_id);
    assert_eq!(explanation.kind, ProvenanceKind::TracedDerivation,
        "Self-play synthesis should be traced");
    assert_eq!(explanation.depth, 2, "Thesis→Antithesis→Synthesis = depth 2");
    assert_eq!(explanation.trace_chain.len(), 3, "Should have 3 steps in chain");
}

/// Knowledge engine + CognitiveCore + PSL end-to-end.
#[test]
fn test_knowledge_driven_reasoning() {
    

    let mut core = CognitiveCore::new().expect("init");

    // Teach the system something.
    core.knowledge.learn("quantum_key_distribution", &["security", "quantum"], true).expect("learn");

    // Ask it about something it should know.
    let result = core.think("explain quantum key distribution for secure voting").expect("think");
    assert!(result.confidence > 0.0, "Should produce a result with some confidence");
    assert!(result.intent.is_some(), "Should detect an intent");
}

// ============================================================
// LfiAgent end-to-end provenance flow
// ============================================================

/// Think → provenance arena → query: the full loop an API user would take.
#[test]
fn test_agent_think_then_query_provenance() {
    use lfi_vsa_core::agent::LfiAgent;

    let mut agent = LfiAgent::new().expect("agent init");
    let input = "what is information sovereignty";
    let (_result, cid) = agent.think_traced(input).expect("think_traced");
    assert_eq!(cid, LfiAgent::conclusion_id_for_input(input));

    let engine = agent.provenance.lock();
    let explanation = engine.explain_conclusion(cid);
    assert_eq!(explanation.kind, ProvenanceKind::TracedDerivation,
        "think_traced must produce TracedDerivation");
    assert!(!explanation.trace_chain.is_empty(),
        "chain must not be empty after think_traced");
    for &tid in &explanation.trace_chain {
        assert!(engine.arena.get(tid).is_some(),
            "chain references must resolve to live entries");
    }
}

/// Chat → provenance: same invariant via the conversational path.
#[test]
fn test_agent_chat_then_query_provenance() {
    use lfi_vsa_core::agent::LfiAgent;

    let mut agent = LfiAgent::new().expect("agent init");
    let (_resp, cid) = agent.chat_traced("hello who are you").expect("chat_traced");

    let engine = agent.provenance.lock();
    let explanation = engine.explain_conclusion(cid);
    assert_eq!(explanation.kind, ProvenanceKind::TracedDerivation,
        "chat_traced must produce TracedDerivation");
}

/// Multiple thinks accumulate distinct traces.
#[test]
fn test_multiple_thinks_accumulate_distinct_traces() {
    use lfi_vsa_core::agent::LfiAgent;

    let mut agent = LfiAgent::new().expect("agent init");
    let (_, cid_a) = agent.think_traced("what is a hypervector").expect("think a");
    let (_, cid_b) = agent.think_traced("define probabilistic soft logic").expect("think b");
    let (_, cid_c) = agent.think_traced("explain active inference").expect("think c");

    let engine = agent.provenance.lock();
    assert_eq!(engine.explain_conclusion(cid_a).kind, ProvenanceKind::TracedDerivation);
    assert_eq!(engine.explain_conclusion(cid_b).kind, ProvenanceKind::TracedDerivation);
    assert_eq!(engine.explain_conclusion(cid_c).kind, ProvenanceKind::TracedDerivation);
    assert!(engine.trace_count() >= 3);
}

/// Exporting and re-importing the arena preserves query results.
#[test]
fn test_export_reimport_preserves_queries() {
    use lfi_vsa_core::agent::LfiAgent;
    use lfi_vsa_core::reasoning_provenance::TraceArena;

    let mut agent = LfiAgent::new().expect("agent init");
    let (_, cid) = agent.think_traced("test input for export").expect("think");

    let json = {
        let engine = agent.provenance.lock();
        engine.arena.to_json().expect("serialize")
    };

    let restored = TraceArena::from_json(&json).expect("deserialize");
    assert!(restored.len() >= 1);
    assert!(
        restored.best_trace_for_conclusion(cid).is_some(),
        "Restored arena must still answer queries for original cid"
    );
}
