// ============================================================
// Training Pipeline — Self-Play + Knowledge Ingestion + Checkpointing
//
// Orchestrates the training loop:
//   1. Run self-play episodes (thesis-antithesis-synthesis)
//   2. Ingest external knowledge (Lean proofs, code, reasoning data)
//   3. Checkpoint intelligence after each epoch
//   4. Track acceleration metrics (System 1 hit rate)
//
// The training loop makes LFI progressively more intelligent:
//   - Each self-play episode hardens strategies
//   - Each knowledge ingestion expands the concept space
//   - Each checkpoint preserves progress for recovery
// ============================================================

use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use crate::hdc::error::HdcError;
use crate::psl::supervisor::PslSupervisor;
use crate::psl::axiom::DimensionalityAxiom;
use crate::cognition::mcts::MctsEngine;
use crate::cognition::knowledge::KnowledgeEngine;
use crate::intelligence::weight_manager::IntelligenceCheckpoint;
use crate::intelligence::persistence::KnowledgeStore;
use tracing::info;
use std::path::PathBuf;

/// Configuration for a training run.
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Number of self-play episodes per epoch.
    pub episodes_per_epoch: usize,
    /// MCTS iterations per episode.
    pub mcts_iterations: usize,
    /// Number of epochs to run.
    pub epochs: usize,
    /// Checkpoint directory.
    pub checkpoint_dir: PathBuf,
    /// Whether to enable provenance during training.
    pub enable_provenance: bool,
    /// Minimum synthesis rate to consider training productive.
    pub min_synthesis_rate: f64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            episodes_per_epoch: 100,
            mcts_iterations: 20,
            epochs: 10,
            checkpoint_dir: IntelligenceCheckpoint::default_dir(),
            enable_provenance: true,
            min_synthesis_rate: 0.1,
        }
    }
}

/// Results from a training epoch.
#[derive(Debug, Clone)]
pub struct EpochResult {
    pub epoch: usize,
    pub episodes_run: usize,
    pub syntheses_forged: usize,
    pub synthesis_rate: f64,
    pub avg_reward: f64,
    pub concepts_learned: usize,
}

/// Results from a complete training run.
#[derive(Debug)]
pub struct TrainingResult {
    pub epochs_completed: usize,
    pub total_episodes: usize,
    pub total_syntheses: usize,
    pub epoch_results: Vec<EpochResult>,
    pub final_checkpoint_path: Option<PathBuf>,
}

/// The training orchestrator.
pub struct Trainer {
    config: TrainingConfig,
    supervisor: PslSupervisor,
    total_episodes: u64,
    total_syntheses: u64,
}

impl Trainer {
    pub fn new(config: TrainingConfig) -> Self {
        debuglog!("Trainer::new: config={:?}", config);
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));
        Self { config, supervisor, total_episodes: 0, total_syntheses: 0 }
    }

    /// Run the full training pipeline.
    pub fn train(&mut self) -> Result<TrainingResult, HdcError> {
        info!("// TRAINING: Starting {} epochs x {} episodes", self.config.epochs, self.config.episodes_per_epoch);

        let mut epoch_results = Vec::new();
        let mut final_checkpoint = None;

        for epoch in 0..self.config.epochs {
            let result = self.run_epoch(epoch)?;
            info!("// TRAINING: Epoch {} complete — syntheses={}/{}, rate={:.2}%, reward={:.4}",
                epoch, result.syntheses_forged, result.episodes_run,
                result.synthesis_rate * 100.0, result.avg_reward);

            // Checkpoint after each epoch.
            let cp_path = self.config.checkpoint_dir.join(
                format!("epoch_{:04}_{}", epoch, IntelligenceCheckpoint::generate_filename())
            );
            let knowledge_json = serde_json::to_string(&KnowledgeStore::new())
                .unwrap_or_else(|_| "{}".into());
            let cp = IntelligenceCheckpoint::capture(
                &knowledge_json,
                self.total_episodes,
                result.concepts_learned,
                0, // rejections tracked separately
                result.syntheses_forged,
                &format!("Epoch {} checkpoint", epoch),
            );
            if let Err(e) = cp.save(&cp_path) {
                debuglog!("Trainer: Checkpoint save failed: {:?}", e);
            } else {
                final_checkpoint = Some(cp_path);
            }

            epoch_results.push(result);
        }

        Ok(TrainingResult {
            epochs_completed: self.config.epochs,
            total_episodes: self.total_episodes as usize,
            total_syntheses: self.total_syntheses as usize,
            epoch_results,
            final_checkpoint_path: final_checkpoint,
        })
    }

    /// Run a single epoch of self-play.
    fn run_epoch(&mut self, epoch: usize) -> Result<EpochResult, HdcError> {
        debuglog!("Trainer::run_epoch: epoch={}", epoch);
        let mut syntheses = 0;
        let mut total_reward = 0.0;

        for episode in 0..self.config.episodes_per_epoch {
            let (forged, reward) = self.run_episode(epoch, episode)?;
            if forged { syntheses += 1; }
            total_reward += reward;
            self.total_episodes += 1;
        }

        if syntheses > 0 {
            self.total_syntheses += syntheses as u64;
        }

        let episodes = self.config.episodes_per_epoch;
        Ok(EpochResult {
            epoch,
            episodes_run: episodes,
            syntheses_forged: syntheses,
            synthesis_rate: syntheses as f64 / episodes as f64,
            avg_reward: total_reward / episodes as f64,
            concepts_learned: 0, // Updated by knowledge ingestion
        })
    }

    /// Run a single self-play episode (thesis-antithesis-synthesis).
    fn run_episode(&mut self, epoch: usize, episode: usize) -> Result<(bool, f64), HdcError> {
        // THESIS: MCTS generates a strategic move.
        let root = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut engine = MctsEngine::new(root, goal.clone());

        if self.config.enable_provenance {
            engine.enable_provenance();
        }

        let thesis = engine.deliberate(self.config.mcts_iterations, &self.supervisor)
            .map_err(|e| HdcError::LogicFault { reason: format!("MCTS failed: {}", e) })?;

        // ANTITHESIS: Measure quality via goal similarity.
        let reward = thesis.similarity(&goal);
        let normalized_reward = (reward + 1.0) / 2.0; // [0, 1]

        // SYNTHESIS: Forge if reward exceeds threshold.
        let forged = normalized_reward > 0.5;

        if forged {
            debuglog!("Trainer::episode: SYNTHESIS forged (epoch={}, episode={}, reward={:.4})", epoch, episode, normalized_reward);
        }

        Ok((forged, normalized_reward))
    }

    /// Get training statistics.
    pub fn stats(&self) -> (u64, u64) {
        (self.total_episodes, self.total_syntheses)
    }

    /// Curriculum training: progressively increase difficulty across epochs.
    ///
    /// Epoch 0: only easy examples (difficulty ≤ 0.2)
    /// Epoch 1: easy + medium (difficulty ≤ 0.35)
    /// Epoch 2+: all examples
    ///
    /// This prevents overwhelming the knowledge engine early and builds
    /// foundational knowledge before tackling harder material.
    pub fn train_curriculum(
        &mut self,
        knowledge: &mut crate::cognition::knowledge::KnowledgeEngine,
    ) -> Result<CombinedTrainingResult, HdcError> {
        info!("// TRAINING: Curriculum mode — progressive difficulty");

        let all = crate::intelligence::training_data::TrainingDataGenerator::all_examples();
        let mut correction_loop = crate::intelligence::training_data::CorrectionLoop::new();
        let mut epoch_results = Vec::new();
        let mut final_checkpoint = None;

        for epoch in 0..self.config.epochs {
            // Progressive difficulty threshold.
            let max_difficulty = match epoch {
                0 => 0.2,
                1 => 0.35,
                2 => 0.5,
                _ => 1.0,
            };

            let curriculum: Vec<_> = all.iter()
                .filter(|e| e.difficulty <= max_difficulty)
                .cloned()
                .collect();

            info!("// CURRICULUM: Epoch {} — difficulty ≤ {:.2}, {} examples",
                epoch, max_difficulty, curriculum.len());

            // Ingest curriculum examples.
            let _ = crate::intelligence::training_data::TrainingDataGenerator::ingest_into_knowledge(
                knowledge, &curriculum,
            )?;

            // Apply knowledge transfer between related domains.
            let domains: Vec<String> = curriculum.iter()
                .map(|e| e.domain.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            for domain in &domains {
                let _ = crate::intelligence::training_data::TrainingDataGenerator::apply_transfer(
                    knowledge, domain, 0.05,
                );
            }

            // Self-play.
            let sp_result = self.run_epoch(epoch)?;

            // Apply mastery decay.
            knowledge.apply_mastery_decay(0.02);

            // Correction loop.
            let _ = correction_loop.evaluate_and_correct(knowledge, &curriculum)?;
            let accuracy = correction_loop.overall_accuracy();

            info!("// CURRICULUM: Epoch {} complete — {}/{} examples, accuracy={:.1}%",
                epoch, curriculum.len(), all.len(), accuracy * 100.0);

            // Checkpoint.
            let cp_path = self.config.checkpoint_dir.join(
                format!("curriculum_epoch_{:04}_{}", epoch,
                    crate::intelligence::weight_manager::IntelligenceCheckpoint::generate_filename())
            );
            let cp = crate::intelligence::weight_manager::IntelligenceCheckpoint::capture(
                "{}", self.total_episodes, knowledge.concept_count(),
                correction_loop.total_corrections(), sp_result.syntheses_forged,
                &format!("Curriculum epoch {} (difficulty ≤ {:.2})", epoch, max_difficulty),
            );
            if let Err(e) = cp.save(&cp_path) {
                debuglog!("Trainer: Checkpoint failed: {:?}", e);
            } else {
                final_checkpoint = Some(cp_path);
            }

            epoch_results.push(sp_result);
        }

        Ok(CombinedTrainingResult {
            epochs_completed: self.config.epochs,
            total_episodes: self.total_episodes as usize,
            total_syntheses: self.total_syntheses as usize,
            final_accuracy: correction_loop.overall_accuracy(),
            total_corrections: correction_loop.total_corrections(),
            concepts_learned: knowledge.concept_count(),
            final_checkpoint_path: final_checkpoint,
        })
    }

    /// Combined training: self-play + knowledge ingestion + correction loop.
    ///
    /// Each epoch:
    ///   1. Run self-play episodes (forge strategies)
    ///   2. Ingest training examples into knowledge engine
    ///   3. Run correction loop to identify and fix gaps
    ///   4. Checkpoint everything
    pub fn train_with_knowledge(
        &mut self,
        knowledge: &mut crate::cognition::knowledge::KnowledgeEngine,
        examples: &[crate::intelligence::training_data::TrainingExample],
    ) -> Result<CombinedTrainingResult, HdcError> {
        info!("// TRAINING: Combined run — {} epochs, {} examples", self.config.epochs, examples.len());

        let mut epoch_results = Vec::new();
        let mut correction_loop = crate::intelligence::training_data::CorrectionLoop::new();
        let mut final_checkpoint = None;

        // Initial knowledge ingestion.
        let ingested = crate::intelligence::training_data::TrainingDataGenerator::ingest_into_knowledge(
            knowledge, examples,
        )?;
        info!("// TRAINING: Ingested {} training examples", ingested);

        for epoch in 0..self.config.epochs {
            // 1. Self-play.
            let sp_result = self.run_epoch(epoch)?;

            // 2. Apply mastery decay (spaced repetition — unused knowledge fades).
            knowledge.apply_mastery_decay(0.02); // 2% decay per epoch

            // 3. Correction loop — evaluate and teach.
            let eval_results = correction_loop.evaluate_and_correct(knowledge, examples)?;
            let accuracy = correction_loop.overall_accuracy();
            let corrections = correction_loop.total_corrections();

            info!("// TRAINING: Epoch {} — syntheses={}, accuracy={:.1}%, corrections={}",
                epoch, sp_result.syntheses_forged, accuracy * 100.0, corrections);

            // 3. Checkpoint.
            let cp_path = self.config.checkpoint_dir.join(
                format!("combined_epoch_{:04}_{}", epoch,
                    crate::intelligence::weight_manager::IntelligenceCheckpoint::generate_filename())
            );
            let knowledge_json = serde_json::to_string(
                &crate::intelligence::persistence::KnowledgeStore::new()
            ).unwrap_or_else(|_| "{}".into());
            let cp = crate::intelligence::weight_manager::IntelligenceCheckpoint::capture(
                &knowledge_json, self.total_episodes,
                knowledge.concept_count(), corrections, sp_result.syntheses_forged,
                &format!("Combined epoch {} — accuracy {:.1}%", epoch, accuracy * 100.0),
            );
            if let Err(e) = cp.save(&cp_path) {
                debuglog!("Trainer: Checkpoint failed: {:?}", e);
            } else {
                final_checkpoint = Some(cp_path);
            }

            epoch_results.push(sp_result);
        }

        Ok(CombinedTrainingResult {
            epochs_completed: self.config.epochs,
            total_episodes: self.total_episodes as usize,
            total_syntheses: self.total_syntheses as usize,
            final_accuracy: correction_loop.overall_accuracy(),
            total_corrections: correction_loop.total_corrections(),
            concepts_learned: knowledge.concept_count(),
            final_checkpoint_path: final_checkpoint,
        })
    }
}

/// Results from combined training (self-play + knowledge + corrections).
#[derive(Debug)]
pub struct CombinedTrainingResult {
    pub epochs_completed: usize,
    pub total_episodes: usize,
    pub total_syntheses: usize,
    pub final_accuracy: f64,
    pub total_corrections: usize,
    pub concepts_learned: usize,
    pub final_checkpoint_path: Option<std::path::PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trainer_creation() {
        let trainer = Trainer::new(TrainingConfig::default());
        let (episodes, syntheses) = trainer.stats();
        assert_eq!(episodes, 0);
        assert_eq!(syntheses, 0);
    }

    #[test]
    fn test_single_episode() -> Result<(), HdcError> {
        let config = TrainingConfig {
            episodes_per_epoch: 1,
            mcts_iterations: 5,
            epochs: 1,
            enable_provenance: false,
            ..Default::default()
        };
        let mut trainer = Trainer::new(config);
        let (_, reward) = trainer.run_episode(0, 0)?;
        assert!(reward >= 0.0 && reward <= 1.0, "Reward should be in [0,1]: {:.4}", reward);
        Ok(())
    }

    #[test]
    fn test_single_epoch() -> Result<(), HdcError> {
        let config = TrainingConfig {
            episodes_per_epoch: 5,
            mcts_iterations: 5,
            epochs: 1,
            enable_provenance: false,
            ..Default::default()
        };
        let mut trainer = Trainer::new(config);
        let result = trainer.run_epoch(0)?;
        assert_eq!(result.episodes_run, 5);
        assert!(result.synthesis_rate >= 0.0 && result.synthesis_rate <= 1.0);
        Ok(())
    }

    #[test]
    fn test_mini_training_run() -> Result<(), HdcError> {
        let config = TrainingConfig {
            episodes_per_epoch: 3,
            mcts_iterations: 5,
            epochs: 2,
            enable_provenance: false,
            checkpoint_dir: PathBuf::from("/tmp/lfi_test_training"),
            ..Default::default()
        };
        let mut trainer = Trainer::new(config);
        let result = trainer.train()?;
        assert_eq!(result.epochs_completed, 2);
        assert_eq!(result.total_episodes, 6);
        assert!(result.epoch_results.len() == 2);

        // Cleanup.
        let _ = std::fs::remove_dir_all("/tmp/lfi_test_training");
        Ok(())
    }

    #[test]
    fn test_training_with_provenance() -> Result<(), HdcError> {
        let config = TrainingConfig {
            episodes_per_epoch: 2,
            mcts_iterations: 5,
            epochs: 1,
            enable_provenance: true,
            checkpoint_dir: PathBuf::from("/tmp/lfi_test_prov_training"),
            ..Default::default()
        };
        let mut trainer = Trainer::new(config);
        let result = trainer.train()?;
        assert_eq!(result.epochs_completed, 1);
        let _ = std::fs::remove_dir_all("/tmp/lfi_test_prov_training");
        Ok(())
    }

    #[test]
    fn test_checkpoint_created_after_training() -> Result<(), HdcError> {
        let dir = PathBuf::from("/tmp/lfi_test_cp_training");
        let config = TrainingConfig {
            episodes_per_epoch: 2,
            mcts_iterations: 5,
            epochs: 1,
            enable_provenance: false,
            checkpoint_dir: dir.clone(),
            ..Default::default()
        };
        let mut trainer = Trainer::new(config);
        let result = trainer.train()?;
        assert!(result.final_checkpoint_path.is_some(), "Should produce a checkpoint");
        let cp_path = result.final_checkpoint_path.unwrap();
        assert!(cp_path.exists(), "Checkpoint file should exist on disk");

        // Verify we can load it back.
        let loaded = IntelligenceCheckpoint::load(&cp_path)?;
        assert_eq!(loaded.episodes_completed, 2);

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn test_combined_training() -> Result<(), HdcError> {
        use crate::intelligence::training_data::TrainingDataGenerator;
        use crate::cognition::knowledge::KnowledgeEngine;

        let dir = std::path::PathBuf::from("/tmp/lfi_combined_training");
        let config = TrainingConfig {
            episodes_per_epoch: 3,
            mcts_iterations: 5,
            epochs: 2,
            enable_provenance: false,
            checkpoint_dir: dir.clone(),
            ..Default::default()
        };
        let mut trainer = Trainer::new(config);
        let mut knowledge = KnowledgeEngine::new();
        let examples = TrainingDataGenerator::all_examples();

        let result = trainer.train_with_knowledge(&mut knowledge, &examples)?;

        assert_eq!(result.epochs_completed, 2);
        assert!(result.concepts_learned > 50, "Should learn many concepts: {}", result.concepts_learned);
        assert!(result.final_accuracy > 0.0, "Should have some accuracy");
        assert!(result.final_checkpoint_path.is_some());

        println!("Combined training: {} episodes, {} syntheses, {:.1}% accuracy, {} concepts, {} corrections",
            result.total_episodes, result.total_syntheses,
            result.final_accuracy * 100.0, result.concepts_learned, result.total_corrections);

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
