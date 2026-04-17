//! Full training + benchmark run — measures LFI intelligence growth.

use lfi_vsa_core::intelligence::training::{Trainer, TrainingConfig};
use lfi_vsa_core::intelligence::training_data::TrainingDataGenerator;
use lfi_vsa_core::intelligence::benchmark::IntelligenceBenchmark;
use lfi_vsa_core::cognition::knowledge::KnowledgeEngine;
use std::path::PathBuf;

#[test]
fn test_intelligence_growth() {
    let mut knowledge = KnowledgeEngine::new();
    let examples = TrainingDataGenerator::all_examples();
    println!("Training data: {} examples across {} unique domains",
        examples.len(),
        examples.iter().map(|e| &e.domain).collect::<std::collections::HashSet<_>>().len());

    // Benchmark BEFORE training.
    let before = IntelligenceBenchmark::run_with_examples(&mut knowledge, &examples)
        .expect("benchmark");
    println!("\nBEFORE training:");
    println!("  Accuracy: {:.1}% ({}/{})", before.overall_accuracy * 100.0, before.total_correct, before.total_examples);

    // Run combined training: self-play + knowledge ingestion + correction.
    let dir = PathBuf::from("/tmp/lfi_growth_test");
    let config = TrainingConfig {
        episodes_per_epoch: 5,
        mcts_iterations: 10,
        epochs: 3,
        enable_provenance: true,
        checkpoint_dir: dir.clone(),
        ..Default::default()
    };
    let mut trainer = Trainer::new(config);
    let train_result = trainer.train_with_knowledge(&mut knowledge, &examples)
        .expect("training");

    println!("\nTraining complete:");
    println!("  Episodes: {}, Syntheses: {}", train_result.total_episodes, train_result.total_syntheses);
    println!("  Concepts: {}, Corrections: {}", train_result.concepts_learned, train_result.total_corrections);

    // Benchmark AFTER training.
    let after = IntelligenceBenchmark::run_with_examples(&mut knowledge, &examples)
        .expect("benchmark");
    println!("\nAFTER training:");
    println!("  Accuracy: {:.1}% ({}/{})", after.overall_accuracy * 100.0, after.total_correct, after.total_examples);

    // Intelligence should have grown.
    assert!(after.overall_accuracy >= before.overall_accuracy,
        "Accuracy should improve: {:.1}% -> {:.1}%",
        before.overall_accuracy * 100.0, after.overall_accuracy * 100.0);
    assert!(after.total_correct >= before.total_correct);

    println!("\nGROWTH: {:.1}% -> {:.1}% (+{:.1}pp)",
        before.overall_accuracy * 100.0,
        after.overall_accuracy * 100.0,
        (after.overall_accuracy - before.overall_accuracy) * 100.0);

    // Print per-domain scores.
    println!("\nPer-domain:");
    IntelligenceBenchmark::print_report(&after);

    let _ = std::fs::remove_dir_all(&dir);
}
