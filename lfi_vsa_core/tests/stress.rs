//! Stress tests — validate LFI scalability for production deployment.
//!
//! Target hardware: i7 64GB RAM 3050Ti laptop, Pixel 10 Pro XL phone.
//! These tests verify the system handles real-world scale without
//! degradation or memory issues.

use lfi_vsa_core::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use lfi_vsa_core::hdc::vector::BipolarVector;
use lfi_vsa_core::hdc::holographic::HolographicMemory;
use lfi_vsa_core::psl::supervisor::PslSupervisor;
use lfi_vsa_core::psl::axiom::{DimensionalityAxiom, AuditTarget, EntropyAxiom};
use lfi_vsa_core::reasoning_provenance::TraceArena;
use lfi_vsa_core::cognition::mcts::MctsEngine;
use lfi_vsa_core::hdc::compute::ResourceEstimator;

/// Stress: 100 MCTS iterations with provenance recording.
#[test]
fn stress_mcts_100_iterations() {
    let root = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
    let mut engine = MctsEngine::new(root, goal);
    engine.enable_provenance();

    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));

    let start = std::time::Instant::now();
    let result = engine.deliberate(100, &supervisor);
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "100-iteration MCTS should succeed");
    assert!(engine.node_count() > 100);

    let arena = engine.provenance().unwrap();
    assert!(arena.len() >= 100, "Should have 100+ trace entries");

    println!("STRESS: 100 MCTS iterations in {:?} ({:.0} iter/sec)",
        elapsed, 100.0 / elapsed.as_secs_f64());
    assert!(elapsed.as_secs() < 30, "100 iterations should complete in under 30s");
}

/// Stress: 1000 PSL audits in sequence.
#[test]
fn stress_1000_psl_audits() {
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(DimensionalityAxiom));
    supervisor.register_axiom(Box::new(EntropyAxiom::default()));

    let vec = BipolarVector::new_random().expect("random");
    let target = AuditTarget::Vector(vec);

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let verdict = supervisor.audit(&target).expect("audit");
        assert!(verdict.confidence > 0.0);
    }
    let elapsed = start.elapsed();

    println!("STRESS: 1000 PSL audits in {:?} ({:.0} audits/sec)",
        elapsed, 1000.0 / elapsed.as_secs_f64());
    assert!(elapsed.as_secs() < 10, "1000 audits should complete in under 10s");
}

/// Stress: 500 holographic memory associations.
#[test]
fn stress_500_holographic_associations() {
    let mut mem = HolographicMemory::new();

    let start = std::time::Instant::now();
    for _ in 0..500 {
        let k = BipolarVector::new_random().expect("key");
        let v = BipolarVector::new_random().expect("value");
        mem.associate(&k, &v).expect("associate");
    }
    let elapsed = start.elapsed();

    assert_eq!(mem.association_count(), 500);
    assert!(mem.is_near_capacity(), "500 associations should be near capacity");

    println!("STRESS: 500 holographic associations in {:?}", elapsed);
    assert!(elapsed.as_secs() < 30);
}

/// Stress: 10,000 provenance trace entries.
#[test]
fn stress_10k_provenance_entries() {
    let mut arena = TraceArena::new();

    let start = std::time::Instant::now();
    for i in 0..10_000 {
        arena.record_step(
            None,
            lfi_vsa_core::reasoning_provenance::InferenceSource::MctsExpansion {
                action: "Decompose".into(),
                node_depth: i % 10,
            },
            vec![],
            0.5,
            None,
            format!("stress_{}", i),
            0,
        );
    }
    let elapsed = start.elapsed();

    assert_eq!(arena.len(), 10_000);

    println!("STRESS: 10k trace entries in {:?} ({:.0} entries/sec)",
        elapsed, 10000.0 / elapsed.as_secs_f64());
    assert!(elapsed.as_secs() < 5, "10k trace entries should be fast");
}

/// Stress: Verify resource estimation for target hardware.
#[test]
fn stress_resource_estimation() {
    // Laptop: i7 64GB RAM
    let laptop = ResourceEstimator::laptop_estimate();
    assert!(laptop.contains("YES"), "64GB laptop should fit: {}", laptop);

    // Phone: Pixel 10 Pro XL (~12GB)
    let phone = ResourceEstimator::phone_estimate();
    println!("Phone estimate: {}", phone);

    // Verify 100k vectors fit in 64GB.
    assert!(ResourceEstimator::fits_in_ram(64 * 1024, 10000, 100000));
    // Verify 10k vectors fit in 4GB (phone budget).
    assert!(ResourceEstimator::fits_in_ram(4 * 1024, 10000, 10000));
}

/// Stress: Rapid vector operations — bind/similarity throughput.
#[test]
fn stress_vector_throughput() {
    let a = BipolarVector::new_random().expect("a");
    let b = BipolarVector::new_random().expect("b");

    let start = std::time::Instant::now();
    for _ in 0..5000 {
        let _ = a.bind(&b).expect("bind");
        let _ = a.similarity(&b).expect("sim");
    }
    let elapsed = start.elapsed();

    let ops = 10000.0 / elapsed.as_secs_f64();
    println!("STRESS: 5000 bind+similarity pairs in {:?} ({:.0} ops/sec)", elapsed, ops);
    assert!(elapsed.as_secs() < 30, "5000 op pairs should complete in 30s");
}
