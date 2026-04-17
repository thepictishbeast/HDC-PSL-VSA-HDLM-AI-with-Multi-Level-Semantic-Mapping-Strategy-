//! Deployment readiness tests — verify LFI works on target hardware profiles.

use lfi_vsa_core::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use lfi_vsa_core::hdc::compute::{DeploymentProfile, ResourceEstimator};
use lfi_vsa_core::psl::supervisor::PslSupervisor;
use lfi_vsa_core::psl::axiom::*;
use lfi_vsa_core::cognition::mcts::MctsEngine;
use lfi_vsa_core::cognition::router::{SemanticRouter, IntelligenceTier};

#[test]
fn test_laptop_deployment_mcts() {
    let profile = DeploymentProfile::laptop();
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));
    supervisor.register_axiom(Box::new(EntropyAxiom::default()));

    let root = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let mut engine = MctsEngine::new(root, goal);
    engine.enable_provenance();

    // Laptop can handle full MCTS with provenance.
    let result = engine.deliberate(profile.max_mcts_iterations.min(50), &supervisor);
    assert!(result.is_ok(), "Laptop should handle MCTS: {:?}", result.err());
    assert!(engine.provenance().unwrap().len() > 0);
}

#[test]
fn test_phone_deployment_constrained_mcts() {
    let profile = DeploymentProfile::pixel_phone();
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));

    let root = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let mut engine = MctsEngine::new(root, goal);

    // Phone runs fewer iterations but still works.
    let result = engine.deliberate(profile.max_mcts_iterations.min(20), &supervisor);
    assert!(result.is_ok(), "Phone should handle constrained MCTS");
}

#[test]
fn test_router_adapts_to_profile() {
    let laptop = DeploymentProfile::laptop();
    let phone = DeploymentProfile::pixel_phone();
    let embedded = DeploymentProfile::embedded();

    // Laptop gets full BigBrain access.
    assert!(matches!(laptop.recommended_max_tier(), IntelligenceTier::BigBrain));

    // Phone gets Bridge tier.
    assert!(matches!(phone.recommended_max_tier(), IntelligenceTier::Bridge));

    // Embedded gets Pulse only.
    assert!(matches!(embedded.recommended_max_tier(), IntelligenceTier::Pulse));

    // Router should respect the profile.
    let mut router = SemanticRouter::new();
    router.set_max_tier(phone.recommended_max_tier());
    let input = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let decision = router.route_explained(&input);
    assert!(decision.tier as u8 <= IntelligenceTier::Bridge as u8,
        "Phone-capped router should not escalate to BigBrain");
}

#[test]
fn test_memory_fits_all_profiles() {
    let laptop = DeploymentProfile::laptop();
    let phone = DeploymentProfile::pixel_phone();

    // Laptop: 500k vectors should fit in 64GB.
    assert!(ResourceEstimator::fits_in_ram(
        laptop.estimated_ram_mb, DIM_PROLETARIAT, laptop.max_vectors
    ));

    // Phone: 50k vectors should fit in 12GB.
    assert!(ResourceEstimator::fits_in_ram(
        phone.estimated_ram_mb, DIM_PROLETARIAT, phone.max_vectors
    ));
}

#[test]
fn test_full_stack_on_phone_profile() {
    let _profile = DeploymentProfile::pixel_phone();

    // Build the full security stack within phone constraints.
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));
    supervisor.register_axiom(Box::new(EntropyAxiom::default()));
    supervisor.register_axiom(Box::new(InjectionDetectionAxiom));
    supervisor.register_axiom(Box::new(ExfiltrationDetectionAxiom));

    // Clean input should pass on phone.
    let clean = AuditTarget::Payload {
        source: "user".into(),
        fields: vec![("query".into(), "What is HDC?".into())],
    };
    let verdict = supervisor.audit(&clean).expect("audit");
    assert!(verdict.level.permits_execution());

    // Attack should still be caught on phone.
    let attack = AuditTarget::Payload {
        source: "attacker".into(),
        fields: vec![("data".into(), "'; DROP TABLE users; --".into())],
    };
    let verdict2 = supervisor.audit(&attack).expect("audit");
    assert!(!verdict2.level.permits_execution(), "Phone should still block attacks");
}
