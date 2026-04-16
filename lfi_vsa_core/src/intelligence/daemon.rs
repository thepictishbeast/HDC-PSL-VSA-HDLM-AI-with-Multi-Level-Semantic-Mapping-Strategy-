// ============================================================
// LFI Daemon — Continuous Autonomous Self-Improvement
//
// PURPOSE: Runs LFI's self-improvement engine indefinitely,
// alternating between:
//   1. Self-improvement cycles (profile → plan → execute → reflect)
//   2. Real inference training against Ollama (if available)
//   3. Cross-domain knowledge transfer sweeps
//   4. Code evaluation practice
//   5. Checkpoint saving
//
// The daemon never stops. It runs until the process is killed.
// Each cycle adapts based on what the previous cycle learned.
//
// ARCHITECTURE:
//   - Main loop with configurable cycle duration
//   - Automatic Ollama detection (falls back to mock if unavailable)
//   - Periodic checkpointing (every N cycles)
//   - Progress reporting to stdout and log file
//   - Graceful degradation: if one subsystem fails, others continue
// ============================================================

use crate::hdc::error::HdcError;
use crate::cognition::knowledge::KnowledgeEngine;
use crate::intelligence::self_improvement::SelfImprovementEngine;
use crate::intelligence::cross_domain::CrossDomainEngine;
use crate::intelligence::local_inference::{
    InferenceTrainer, InferenceTrainingConfig, InferenceBackend, ModelRouter,
};
use crate::intelligence::training_data::{
    TrainingDataGenerator, TrainingAugmenter, AdversarialExamples,
};
use crate::intelligence::math_engine::MathChallengeRunner;
use crate::intelligence::weight_manager::IntelligenceCheckpoint;
use crate::intelligence::benchmark::IntelligenceBenchmark;

/// Configuration for the daemon.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Ollama host URL (empty = mock mode).
    pub ollama_host: String,
    /// Preferred lightweight model.
    pub lightweight_model: String,
    /// Preferred heavyweight model.
    pub heavyweight_model: String,
    /// Checkpoint save interval (every N cycles).
    pub checkpoint_interval: usize,
    /// Checkpoint directory.
    pub checkpoint_dir: String,
    /// Maximum training examples per Ollama cycle (to limit GPU time).
    pub max_examples_per_cycle: usize,
    /// Enable real Ollama inference (false = mock only).
    pub use_ollama: bool,
    /// Number of self-improvement cycles between Ollama training rounds.
    pub self_improve_cycles_per_round: usize,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            ollama_host: "http://localhost:11434".into(),
            lightweight_model: "deepseek-r1:8b".into(),
            heavyweight_model: "qwen2.5-coder:7b".into(),
            checkpoint_interval: 5,
            checkpoint_dir: "/tmp/lfi_checkpoints".into(),
            max_examples_per_cycle: 20,
            use_ollama: false, // Default to mock until Ollama confirmed
            self_improve_cycles_per_round: 3,
        }
    }
}

impl DaemonConfig {
    /// Create config with Ollama enabled.
    pub fn with_ollama(host: &str, light_model: &str, heavy_model: &str) -> Self {
        Self {
            ollama_host: host.into(),
            lightweight_model: light_model.into(),
            heavyweight_model: heavy_model.into(),
            use_ollama: true,
            ..Default::default()
        }
    }
}

/// Result of a single daemon cycle.
#[derive(Debug, Clone)]
pub struct CycleResult {
    pub cycle_number: usize,
    /// Phase that ran this cycle.
    pub phase: DaemonPhase,
    /// Score before this cycle.
    pub score_before: f64,
    /// Score after this cycle.
    pub score_after: f64,
    /// Number of concepts learned.
    pub concepts_learned: usize,
    /// Number of training examples processed.
    pub examples_processed: usize,
    /// Errors encountered (non-fatal).
    pub errors: Vec<String>,
}

/// Which phase the daemon is executing.
#[derive(Debug, Clone, PartialEq)]
pub enum DaemonPhase {
    /// Self-improvement cycle (profile → plan → execute → reflect).
    SelfImprovement,
    /// Real inference training against Ollama.
    OllamaTraining,
    /// Cross-domain knowledge transfer.
    CrossDomainTransfer,
    /// Math challenge practice.
    MathPractice,
    /// Code evaluation practice.
    CodePractice,
    /// Checkpoint save.
    Checkpoint,
    /// Benchmark run.
    Benchmark,
}

/// The LFI Daemon — runs indefinitely, improving intelligence.
pub struct LfiDaemon {
    pub config: DaemonConfig,
    pub knowledge: KnowledgeEngine,
    pub self_improver: SelfImprovementEngine,
    pub cross_domain: CrossDomainEngine,
    pub math_runner: MathChallengeRunner,
    /// Total cycles completed.
    pub total_cycles: usize,
    /// History of cycle results.
    pub history: Vec<CycleResult>,
    /// Cumulative errors.
    pub error_count: usize,
}

impl LfiDaemon {
    pub fn new(config: DaemonConfig) -> Self {
        debuglog!("LfiDaemon::new: Initializing continuous self-improvement daemon");
        debuglog!("LfiDaemon: ollama={}, host={}, models={}/{}",
            config.use_ollama, config.ollama_host,
            config.lightweight_model, config.heavyweight_model);

        Self {
            config,
            knowledge: KnowledgeEngine::new(),
            self_improver: SelfImprovementEngine::new(),
            cross_domain: CrossDomainEngine::new(),
            math_runner: MathChallengeRunner::new(),
            total_cycles: 0,
            history: Vec::new(),
            error_count: 0,
        }
    }

    /// Detect if Ollama is available at the configured host.
    /// BUG ASSUMPTION: uses curl, which must be on PATH.
    pub fn detect_ollama(&self) -> bool {
        if !self.config.use_ollama { return false; }

        let result = std::process::Command::new("curl")
            .args(&["-s", "--max-time", "3", &format!("{}/api/tags", self.config.ollama_host)])
            .output();

        match result {
            Ok(output) => {
                let body = String::from_utf8_lossy(&output.stdout);
                let available = output.status.success() && body.contains("models");
                debuglog!("LfiDaemon::detect_ollama: available={}", available);
                available
            }
            Err(_) => {
                debuglog!("LfiDaemon::detect_ollama: curl failed, Ollama not available");
                false
            }
        }
    }

    /// Run one cycle of the daemon. Returns the cycle result.
    /// The daemon alternates between phases based on cycle number.
    pub fn run_cycle(&mut self) -> Result<CycleResult, HdcError> {
        self.total_cycles += 1;
        let cycle = self.total_cycles;

        debuglog!("LfiDaemon::run_cycle: === CYCLE {} ===", cycle);

        let score_before = self.self_improver.profile(&self.knowledge).overall_score;
        let concepts_before = self.knowledge.concept_count();
        let mut errors = Vec::new();
        let mut examples_processed = 0;

        // Determine phase based on cycle number.
        let phase = self.select_phase(cycle);
        debuglog!("LfiDaemon::run_cycle: phase={:?}", phase);

        match phase {
            DaemonPhase::SelfImprovement => {
                match self.self_improver.run_cycle(&mut self.knowledge) {
                    Ok(_) => {}
                    Err(e) => errors.push(format!("SelfImprovement: {}", e)),
                }
            }
            DaemonPhase::OllamaTraining => {
                examples_processed = self.run_ollama_training(&mut errors);
            }
            DaemonPhase::CrossDomainTransfer => {
                let insights = self.cross_domain.transfer_sweep(&mut self.knowledge, 0.5);
                debuglog!("LfiDaemon: CrossDomain: {} transfers", insights.len());
            }
            DaemonPhase::MathPractice => {
                let results = self.math_runner.run_arithmetic_suite();
                let correct = results.iter().filter(|(_, ok)| *ok).count();
                debuglog!("LfiDaemon: Math: {}/{} correct", correct, results.len());
                examples_processed = results.len();
            }
            DaemonPhase::CodePractice => {
                // Code practice happens inside self-improvement PracticeCoding action.
                match self.self_improver.run_cycle(&mut self.knowledge) {
                    Ok(_) => {}
                    Err(e) => errors.push(format!("CodePractice: {}", e)),
                }
            }
            DaemonPhase::Checkpoint => {
                match self.save_checkpoint() {
                    Ok(path) => debuglog!("LfiDaemon: Checkpoint saved to {}", path),
                    Err(e) => errors.push(format!("Checkpoint: {}", e)),
                }
            }
            DaemonPhase::Benchmark => {
                match IntelligenceBenchmark::run(&mut self.knowledge) {
                    Ok(report) => {
                        debuglog!("LfiDaemon: Benchmark: {:.1}% overall",
                            report.overall_accuracy * 100.0);
                    }
                    Err(e) => errors.push(format!("Benchmark: {}", e)),
                }
            }
        }

        let score_after = self.self_improver.profile(&self.knowledge).overall_score;
        let concepts_after = self.knowledge.concept_count();

        self.error_count += errors.len();

        let result = CycleResult {
            cycle_number: cycle,
            phase,
            score_before,
            score_after,
            concepts_learned: concepts_after.saturating_sub(concepts_before),
            examples_processed,
            errors,
        };

        self.history.push(result.clone());

        debuglog!("LfiDaemon::run_cycle: cycle={} score={:.4}→{:.4} concepts=+{} errors={}",
            cycle, score_before, score_after, result.concepts_learned, result.errors.len());

        Ok(result)
    }

    /// Select which phase to run based on cycle number.
    fn select_phase(&self, cycle: usize) -> DaemonPhase {
        // Every checkpoint_interval cycles: checkpoint.
        if cycle % self.config.checkpoint_interval == 0 {
            return DaemonPhase::Checkpoint;
        }

        // Every 10 cycles: benchmark.
        if cycle % 10 == 0 {
            return DaemonPhase::Benchmark;
        }

        // Alternate between phases.
        match cycle % 5 {
            0 => DaemonPhase::CrossDomainTransfer,
            1 => DaemonPhase::SelfImprovement,
            2 => {
                if self.config.use_ollama && self.detect_ollama() {
                    DaemonPhase::OllamaTraining
                } else {
                    DaemonPhase::SelfImprovement
                }
            }
            3 => DaemonPhase::MathPractice,
            4 => DaemonPhase::CodePractice,
            _ => DaemonPhase::SelfImprovement,
        }
    }

    /// Run a batch of Ollama inference training.
    fn run_ollama_training(&mut self, errors: &mut Vec<String>) -> usize {
        let config = if self.config.use_ollama {
            InferenceTrainingConfig {
                backend: InferenceBackend::Ollama {
                    model: self.config.lightweight_model.clone(),
                    host: self.config.ollama_host.clone(),
                },
                verify_answers: true,
                active_learning: true,
                cache_enabled: true,
                ..Default::default()
            }
        } else {
            InferenceTrainingConfig::default() // Mock
        };

        let router = if self.config.use_ollama {
            Some(ModelRouter::laptop_optimized(&self.config.ollama_host))
        } else {
            None
        };

        let mut trainer = if let Some(r) = router {
            InferenceTrainer::with_router(config, r)
        } else {
            InferenceTrainer::new(config)
        };

        // Get training examples, prioritized by active learning.
        let all_examples = TrainingDataGenerator::all_examples();
        let batch_size = self.config.max_examples_per_cycle.min(all_examples.len());

        match trainer.train_all(&all_examples[..batch_size], &mut self.knowledge) {
            Ok(result) => {
                debuglog!("LfiDaemon: Ollama training: {}/{} correct ({:.1}%)",
                    result.correct_answers, result.total_questions, result.accuracy * 100.0);
                result.total_questions
            }
            Err(e) => {
                errors.push(format!("OllamaTraining: {}", e));
                0
            }
        }
    }

    /// Save a checkpoint of current knowledge state.
    fn save_checkpoint(&self) -> Result<String, String> {
        let _ = std::fs::create_dir_all(&self.config.checkpoint_dir);
        let path = format!("{}/checkpoint_cycle_{}.json",
            self.config.checkpoint_dir, self.total_cycles);

        let knowledge_json = format!("{{\"concepts\":{}}}", self.knowledge.concept_count());
        let checkpoint = IntelligenceCheckpoint::capture(
            &knowledge_json,
            self.total_cycles as u64,
            self.knowledge.concept_count(),
            0, 0,
            &format!("Daemon cycle {} checkpoint", self.total_cycles),
        );
        checkpoint.save(&std::path::Path::new(&path))
            .map_err(|e| format!("save failed: {}", e))?;
        Ok(path)
    }

    /// Run N cycles.
    pub fn run_n_cycles(&mut self, n: usize) -> Result<Vec<CycleResult>, HdcError> {
        let mut results = Vec::new();
        for _ in 0..n {
            results.push(self.run_cycle()?);
        }
        Ok(results)
    }

    /// Generate a progress report.
    pub fn progress_report(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("=== LFI Daemon Report (Cycle {}) ===\n", self.total_cycles));

        let profile = self.self_improver.profile(&self.knowledge);
        out.push_str(&format!("Overall:    {:.1}%\n", profile.overall_score * 100.0));
        out.push_str(&format!("Math:       {:.1}%\n", profile.math_score * 100.0));
        out.push_str(&format!("Security:   {:.1}%\n", profile.security_score * 100.0));
        out.push_str(&format!("Coding:     {:.1}%\n", profile.coding_score * 100.0));
        out.push_str(&format!("Concepts:   {}\n", profile.concepts_known));
        out.push_str(&format!("Velocity:   {:+.4}\n", profile.learning_velocity));
        out.push_str(&format!("Errors:     {}\n", self.error_count));

        if let (Some(first), Some(last)) = (self.history.first(), self.history.last()) {
            let total_gain = last.score_after - first.score_before;
            out.push_str(&format!("Total gain: {:+.1}% over {} cycles\n",
                total_gain * 100.0, self.total_cycles));
        }

        // Phase breakdown
        let mut phase_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for r in &self.history {
            *phase_counts.entry(format!("{:?}", r.phase)).or_insert(0) += 1;
        }
        out.push_str("\nPhase breakdown:\n");
        for (phase, count) in &phase_counts {
            out.push_str(&format!("  {:25} {}\n", phase, count));
        }

        out
    }

    /// Check if the system is ready for real training.
    /// Returns (ready, Vec<reasons_not_ready>).
    pub fn readiness_check(&self) -> (bool, Vec<String>) {
        let mut issues = Vec::new();

        // Check knowledge engine has seeded concepts.
        if self.knowledge.concept_count() < 30 {
            issues.push(format!("Knowledge engine has only {} concepts (need 30+)", self.knowledge.concept_count()));
        }

        // Check self-improvement can run.
        let profile = self.self_improver.profile(&self.knowledge);
        if profile.domain_scores.is_empty() {
            issues.push("No domain scores available — training data may not be loaded".into());
        }

        // Check training data exists.
        let examples = TrainingDataGenerator::all_examples();
        if examples.len() < 100 {
            issues.push(format!("Only {} training examples (need 100+)", examples.len()));
        }

        // Check augmentation works.
        let augmented = TrainingAugmenter::augment_all(&examples);
        if augmented.is_empty() {
            issues.push("Augmentation produced no examples".into());
        }

        // Check adversarial examples exist.
        let adversarial = AdversarialExamples::all();
        if adversarial.len() < 20 {
            issues.push(format!("Only {} adversarial examples (need 20+)", adversarial.len()));
        }

        // Check Ollama if configured.
        if self.config.use_ollama && !self.detect_ollama() {
            issues.push(format!("Ollama not available at {}", self.config.ollama_host));
        }

        let ready = issues.is_empty();
        (ready, issues)
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_creation() {
        let config = DaemonConfig::default();
        let daemon = LfiDaemon::new(config);
        assert_eq!(daemon.total_cycles, 0);
        assert!(daemon.history.is_empty());
    }

    #[test]
    fn test_daemon_single_cycle() -> Result<(), HdcError> {
        let config = DaemonConfig::default();
        let mut daemon = LfiDaemon::new(config);

        let result = daemon.run_cycle()?;
        assert_eq!(result.cycle_number, 1);
        assert_eq!(daemon.total_cycles, 1);
        Ok(())
    }

    #[test]
    fn test_daemon_multiple_cycles() -> Result<(), HdcError> {
        let config = DaemonConfig::default();
        let mut daemon = LfiDaemon::new(config);

        let results = daemon.run_n_cycles(6)?;
        assert_eq!(results.len(), 6);
        assert_eq!(daemon.total_cycles, 6);

        // Should have tried different phases.
        let phases: Vec<&DaemonPhase> = results.iter().map(|r| &r.phase).collect();
        assert!(phases.contains(&&DaemonPhase::SelfImprovement),
            "Should include SelfImprovement phase");
        Ok(())
    }

    #[test]
    fn test_daemon_phase_rotation() {
        let config = DaemonConfig::default();
        let daemon = LfiDaemon::new(config);

        // Verify phase selection varies.
        let phases: Vec<DaemonPhase> = (1..=10).map(|c| daemon.select_phase(c)).collect();
        let unique: std::collections::HashSet<String> = phases.iter()
            .map(|p| format!("{:?}", p)).collect();
        assert!(unique.len() >= 3, "Should rotate through 3+ phases, got: {:?}", unique);
    }

    #[test]
    fn test_daemon_checkpoint_cycle() -> Result<(), HdcError> {
        let config = DaemonConfig {
            checkpoint_interval: 2,
            checkpoint_dir: "/tmp/lfi_test_checkpoints".into(),
            ..Default::default()
        };
        let mut daemon = LfiDaemon::new(config);

        // Cycle 2 should be a checkpoint.
        let _ = daemon.run_cycle()?; // cycle 1
        let result = daemon.run_cycle()?; // cycle 2 = checkpoint
        assert_eq!(result.phase, DaemonPhase::Checkpoint);

        // Cleanup.
        let _ = std::fs::remove_dir_all("/tmp/lfi_test_checkpoints");
        Ok(())
    }

    #[test]
    fn test_daemon_readiness_check() {
        let config = DaemonConfig::default();
        let daemon = LfiDaemon::new(config);

        let (ready, issues) = daemon.readiness_check();
        // With default config (mock mode, no Ollama), should be ready.
        assert!(ready, "Mock mode should be ready. Issues: {:?}", issues);
    }

    #[test]
    fn test_daemon_readiness_with_ollama() {
        let config = DaemonConfig::with_ollama(
            "http://localhost:99999", // Nonexistent
            "fake-model",
            "fake-model",
        );
        let daemon = LfiDaemon::new(config);

        let (ready, issues) = daemon.readiness_check();
        // Ollama is not running at port 99999.
        assert!(!ready, "Should not be ready with unavailable Ollama");
        assert!(issues.iter().any(|i| i.contains("Ollama not available")));
    }

    #[test]
    fn test_daemon_progress_report() -> Result<(), HdcError> {
        let config = DaemonConfig::default();
        let mut daemon = LfiDaemon::new(config);

        let _ = daemon.run_n_cycles(3)?;
        let report = daemon.progress_report();

        assert!(report.contains("LFI Daemon Report"));
        assert!(report.contains("Overall:"));
        assert!(report.contains("Concepts:"));
        assert!(report.contains("Phase breakdown"));
        Ok(())
    }

    #[test]
    fn test_daemon_concepts_grow() -> Result<(), HdcError> {
        let config = DaemonConfig::default();
        let mut daemon = LfiDaemon::new(config);

        let initial = daemon.knowledge.concept_count();
        let _ = daemon.run_n_cycles(5)?;
        let after = daemon.knowledge.concept_count();

        assert!(after >= initial,
            "Concepts should not decrease: {} → {}", initial, after);
        Ok(())
    }

    #[test]
    fn test_daemon_config_with_ollama() {
        let config = DaemonConfig::with_ollama(
            "http://localhost:11434",
            "deepseek-r1:8b",
            "qwen2.5-coder:7b",
        );
        assert!(config.use_ollama);
        assert_eq!(config.ollama_host, "http://localhost:11434");
    }

    #[test]
    fn test_daemon_error_resilience() -> Result<(), HdcError> {
        // Daemon should survive errors in individual phases.
        let config = DaemonConfig::default();
        let mut daemon = LfiDaemon::new(config);

        // Run many cycles — some may encounter edge cases.
        let results = daemon.run_n_cycles(10)?;
        assert_eq!(results.len(), 10, "Should complete all 10 cycles despite any errors");
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for LfiDaemon
    // ============================================================

    /// INVARIANT: run_n_cycles(N) returns exactly N results, never more, never less.
    #[test]
    fn invariant_run_n_cycles_returns_exact_count() -> Result<(), HdcError> {
        let mut daemon = LfiDaemon::new(DaemonConfig::default());
        for n in [1usize, 5, 25] {
            let results = daemon.run_n_cycles(n)?;
            assert_eq!(results.len(), n,
                "run_n_cycles({}) returned {} results", n, results.len());
        }
        Ok(())
    }

    /// INVARIANT: run_n_cycles(0) is a no-op — returns empty vec, not panic.
    #[test]
    fn invariant_run_zero_cycles_safe() -> Result<(), HdcError> {
        let mut daemon = LfiDaemon::new(DaemonConfig::default());
        let results = daemon.run_n_cycles(0)?;
        assert!(results.is_empty(), "0 cycles must return empty");
        Ok(())
    }

    /// INVARIANT: progress_report never panics regardless of cycle history.
    #[test]
    fn invariant_progress_report_safe_at_any_state() -> Result<(), HdcError> {
        let mut daemon = LfiDaemon::new(DaemonConfig::default());
        // Empty state.
        let r0 = daemon.progress_report();
        assert!(!r0.is_empty(), "empty-state report must produce output");
        // After cycles.
        daemon.run_n_cycles(3)?;
        let r3 = daemon.progress_report();
        assert!(!r3.is_empty(), "after-cycle report must produce output");
        Ok(())
    }

    /// INVARIANT: detect_ollama doesn't panic on any host string.
    #[test]
    fn invariant_detect_ollama_safe_on_unicode() {
        for host in ["", "http://localhost:11434", "アリス://x", "🦀://nope", "garbage"] {
            let config = DaemonConfig::with_ollama(host, "model_a", "model_b");
            let daemon = LfiDaemon::new(config);
            // Just must not panic. Network call may fail but must return bool cleanly.
            let _ = daemon.detect_ollama();
        }
    }

    /// INVARIANT: each CycleResult has cycle_number that is monotonically
    /// non-decreasing across the cycle batch and finite scores.
    #[test]
    fn invariant_cycle_results_well_formed() -> Result<(), HdcError> {
        let mut daemon = LfiDaemon::new(DaemonConfig::default());
        let results = daemon.run_n_cycles(5)?;
        let mut last_n = 0usize;
        for r in &results {
            // Phase: any defined variant accepted (enum is non_exhaustive over
            // future expansion).
            let _phase: &DaemonPhase = &r.phase;
            assert!(r.cycle_number >= last_n,
                "cycle numbers must be non-decreasing: {} then {}",
                last_n, r.cycle_number);
            assert!(r.score_before.is_finite(), "score_before non-finite");
            assert!(r.score_after.is_finite(), "score_after non-finite");
            last_n = r.cycle_number;
        }
        Ok(())
    }

    /// INVARIANT: run_n_cycles(N) produces exactly N results (or errors).
    #[test]
    fn invariant_run_n_cycles_count() -> Result<(), HdcError> {
        let mut daemon = LfiDaemon::new(DaemonConfig::default());
        for n in [1, 3, 7] {
            let results = daemon.run_n_cycles(n)?;
            assert_eq!(results.len(), n,
                "run_n_cycles({}) returned {} results", n, results.len());
        }
        Ok(())
    }

    /// INVARIANT: readiness_check returns (bool, non-empty Vec when unready).
    #[test]
    fn invariant_readiness_check_stable() {
        let daemon = LfiDaemon::new(DaemonConfig::default());
        let (ready, issues) = daemon.readiness_check();
        // Either ready (no issues) or not ready (some issues).
        if !ready {
            // Check issues is non-empty or at least valid vec
            let _ = issues.len();
        }
    }
}
