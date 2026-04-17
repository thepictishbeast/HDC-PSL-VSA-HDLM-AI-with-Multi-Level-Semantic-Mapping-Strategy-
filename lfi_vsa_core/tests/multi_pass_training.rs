//! Multi-pass training — runs curriculum learning with correction and transfer,
//! measures accuracy improvement across passes.

use lfi_vsa_core::intelligence::training::{Trainer, TrainingConfig};
use lfi_vsa_core::intelligence::training_data::{TrainingDataGenerator, CorrectionLoop};
use lfi_vsa_core::intelligence::benchmark::IntelligenceBenchmark;
use lfi_vsa_core::cognition::knowledge::KnowledgeEngine;
use std::path::PathBuf;

#[test]
fn test_multi_pass_accuracy_improvement() {
    let mut knowledge = KnowledgeEngine::new();
    let examples = TrainingDataGenerator::all_examples();

    println!("Training corpus: {} examples across {} domains",
        examples.len(), TrainingDataGenerator::domains().len());

    // Track accuracy across multiple evaluation passes.
    let mut accuracies = Vec::new();

    for pass in 0..4 {
        // Evaluate current state.
        let mut loop_ = CorrectionLoop::new();
        loop_.evaluate_and_correct(&mut knowledge, &examples).expect("eval");
        let accuracy = loop_.overall_accuracy();
        accuracies.push(accuracy);

        println!("Pass {}: accuracy={:.1}%, corrections={}, concepts={}",
            pass, accuracy * 100.0, loop_.total_corrections(), knowledge.concept_count());

        // Apply knowledge transfer after each pass.
        for domain in TrainingDataGenerator::domains() {
            let _ = TrainingDataGenerator::apply_transfer(&mut knowledge, &domain, 0.03);
        }
    }

    // Accuracy should be monotonically non-decreasing.
    for i in 1..accuracies.len() {
        assert!(accuracies[i] >= accuracies[i-1],
            "Accuracy should not decrease: pass {} ({:.1}%) < pass {} ({:.1}%)",
            i, accuracies[i] * 100.0, i-1, accuracies[i-1] * 100.0);
    }

    // Final accuracy should be very high.
    let final_acc = *accuracies.last().unwrap();
    assert!(final_acc >= 0.9,
        "After 4 passes should be >= 90%, got {:.1}%", final_acc * 100.0);

    println!("\nAccuracy progression: {}", accuracies.iter()
        .enumerate()
        .map(|(i, a)| format!("pass{}={:.1}%", i, a * 100.0))
        .collect::<Vec<_>>()
        .join(" → "));
}

#[test]
fn test_curriculum_then_benchmark() {
    let dir = PathBuf::from("/tmp/lfi_curriculum_bench");
    let config = TrainingConfig {
        episodes_per_epoch: 3,
        mcts_iterations: 5,
        epochs: 4, // 4 epochs = easy → medium → hard → all
        enable_provenance: false,
        checkpoint_dir: dir.clone(),
        ..Default::default()
    };

    let mut trainer = Trainer::new(config);
    let mut knowledge = KnowledgeEngine::new();

    let result = trainer.train_curriculum(&mut knowledge).expect("curriculum training");

    println!("Curriculum training: {} episodes, {} syntheses, {} concepts, accuracy={:.1}%",
        result.total_episodes, result.total_syntheses, result.concepts_learned,
        result.final_accuracy * 100.0);

    // Run benchmark.
    let report = IntelligenceBenchmark::run(&mut knowledge).expect("benchmark");
    IntelligenceBenchmark::print_report(&report);

    assert!(report.overall_accuracy > 0.0);
    assert!(report.domains_evaluated >= 20);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_domain_count_and_example_count() {
    let examples = TrainingDataGenerator::all_examples();
    let domains = TrainingDataGenerator::domains();

    println!("Total examples: {}", examples.len());
    println!("Total domains: {}", domains.len());

    // Print per-domain counts.
    for domain in &domains {
        let count = examples.iter().filter(|e| e.domain == *domain).count();
        println!("  {:<16} {} examples", domain, count);
    }

    assert!(examples.len() >= 240, "Should have 240+ examples, got {}", examples.len());
    assert!(domains.len() >= 28, "Should have 28+ domains, got {}", domains.len());
}
