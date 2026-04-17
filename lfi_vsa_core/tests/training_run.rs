//! Full training run — generates real intelligence checkpoints.
//!
//! This test runs a meaningful training session and verifies
//! that intelligence is preserved across save/load cycles.

use lfi_vsa_core::intelligence::training::{Trainer, TrainingConfig};
use lfi_vsa_core::intelligence::weight_manager::IntelligenceCheckpoint;
use std::path::PathBuf;

/// Run a real training session: 5 epochs x 10 episodes = 50 episodes.
#[test]
fn test_real_training_session() {
    let dir = PathBuf::from("/root/.lfi/checkpoints");
    let config = TrainingConfig {
        episodes_per_epoch: 10,
        mcts_iterations: 10,
        epochs: 5,
        enable_provenance: true,
        checkpoint_dir: dir.clone(),
        ..Default::default()
    };

    let mut trainer = Trainer::new(config);
    let result = trainer.train().expect("training should succeed");

    // Verify training completed.
    assert_eq!(result.epochs_completed, 5);
    assert_eq!(result.total_episodes, 50);

    // Verify synthesis happened.
    assert!(result.total_syntheses > 0, "Should forge at least some syntheses");
    println!("Training complete: {} episodes, {} syntheses ({:.0}%)",
        result.total_episodes, result.total_syntheses,
        result.total_syntheses as f64 / result.total_episodes as f64 * 100.0);

    // Verify checkpoint exists and loads.
    let cp_path = result.final_checkpoint_path.expect("should have checkpoint");
    assert!(cp_path.exists(), "Checkpoint file should exist");
    let loaded = IntelligenceCheckpoint::load(&cp_path).expect("should load");
    assert_eq!(loaded.episodes_completed, 50);

    // Print epoch progression.
    for er in &result.epoch_results {
        println!("  Epoch {}: synthesis_rate={:.0}%, avg_reward={:.4}",
            er.epoch, er.synthesis_rate * 100.0, er.avg_reward);
    }
}

/// Verify intelligence survives checkpoint save/load cycle.
#[test]
fn test_intelligence_persistence() {
    let dir = PathBuf::from("/tmp/lfi_persist_test");
    let config = TrainingConfig {
        episodes_per_epoch: 5,
        mcts_iterations: 5,
        epochs: 2,
        enable_provenance: false,
        checkpoint_dir: dir.clone(),
        ..Default::default()
    };

    let mut trainer = Trainer::new(config);
    let result = trainer.train().expect("training");
    let cp_path = result.final_checkpoint_path.expect("checkpoint");

    // Load the checkpoint.
    let cp = IntelligenceCheckpoint::load(&cp_path).expect("load");
    assert_eq!(cp.episodes_completed, 10);
    assert!(cp.version == 1);
    assert!(!cp.integrity_hash.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}
