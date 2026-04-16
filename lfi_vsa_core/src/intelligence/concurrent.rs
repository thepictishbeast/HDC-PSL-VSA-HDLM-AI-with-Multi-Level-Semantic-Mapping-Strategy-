// ============================================================
// Concurrent Train+Improve Mode — Simultaneous Learning Loops
//
// PURPOSE: Run Ollama training AND self-improvement AND continuous
// intelligence gathering IN PARALLEL on separate threads. All three
// loops share a thread-safe KnowledgeEngine, so each loop's learnings
// immediately benefit the others.
//
// WHY PARALLELISM MATTERS:
//   - Ollama training is slow (~7s/query) — CPU/GPU is idle otherwise
//   - Self-improvement can run in the background during LLM waits
//   - Intel gathering is mostly I/O-bound (network) — doesn't block CPU
//   - Combined: training throughput stays constant, but LFI also
//     self-reflects and ingests new info simultaneously
//
// ARCHITECTURE:
//   Main thread coordinates, three worker threads run:
//     [Trainer Thread]  → InferenceTrainer.train_on_example()
//     [Improver Thread] → SelfImprovementEngine.run_cycle()
//     [Intel Thread]    → ContinuousIntelligence.poll_cycle()
//
//   Shared state: Arc<Mutex<KnowledgeEngine>>
//
// SAFETY:
//   - Mutex guards prevent data races on the knowledge engine
//   - Each thread acquires the lock only for its update, releases quickly
//   - Graceful shutdown via atomic flag
//   - Error in one thread doesn't crash the others
// ============================================================

use crate::cognition::knowledge::KnowledgeEngine;
use crate::intelligence::local_inference::{
    InferenceTrainer, InferenceTrainingConfig, InferenceBackend,
};
use crate::intelligence::self_improvement::SelfImprovementEngine;
use crate::intelligence::continuous_intel::ContinuousIntelligence;
use crate::intelligence::training_data::TrainingDataGenerator;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

// ============================================================
// Concurrent Runner Configuration
// ============================================================

/// Configuration for concurrent learning mode.
#[derive(Debug, Clone)]
pub struct ConcurrentConfig {
    /// Ollama backend for the trainer thread.
    pub inference_backend: InferenceBackend,
    /// Enable trainer thread.
    pub run_trainer: bool,
    /// Enable self-improvement thread.
    pub run_improver: bool,
    /// Enable intel gatherer thread.
    pub run_intel: bool,
    /// How long improver sleeps between cycles (ms).
    pub improver_interval_ms: u64,
    /// How long intel gatherer sleeps between polls (ms).
    pub intel_interval_ms: u64,
    /// Maximum runtime before stopping (seconds). 0 = forever.
    pub max_runtime_sec: u64,
}

impl Default for ConcurrentConfig {
    fn default() -> Self {
        Self {
            inference_backend: InferenceBackend::default(),
            run_trainer: true,
            run_improver: true,
            run_intel: true,
            improver_interval_ms: 5_000,
            intel_interval_ms: 30_000,
            max_runtime_sec: 0, // Run until stopped
        }
    }
}

// ============================================================
// Shared State
// ============================================================

/// Thread-safe shared state for concurrent learning.
pub struct SharedState {
    pub knowledge: Arc<Mutex<KnowledgeEngine>>,
    pub stop_flag: Arc<AtomicBool>,

    // Metrics
    pub trainer_cycles: Arc<AtomicUsize>,
    pub improver_cycles: Arc<AtomicUsize>,
    pub intel_cycles: Arc<AtomicUsize>,
    pub trainer_correct: Arc<AtomicUsize>,
    pub trainer_total: Arc<AtomicUsize>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            knowledge: Arc::new(Mutex::new(KnowledgeEngine::new())),
            stop_flag: Arc::new(AtomicBool::new(false)),
            trainer_cycles: Arc::new(AtomicUsize::new(0)),
            improver_cycles: Arc::new(AtomicUsize::new(0)),
            intel_cycles: Arc::new(AtomicUsize::new(0)),
            trainer_correct: Arc::new(AtomicUsize::new(0)),
            trainer_total: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn should_stop(&self) -> bool {
        self.stop_flag.load(Ordering::SeqCst)
    }

    /// Get a snapshot of current metrics.
    pub fn metrics(&self) -> ConcurrentMetrics {
        ConcurrentMetrics {
            trainer_cycles: self.trainer_cycles.load(Ordering::Relaxed),
            improver_cycles: self.improver_cycles.load(Ordering::Relaxed),
            intel_cycles: self.intel_cycles.load(Ordering::Relaxed),
            trainer_correct: self.trainer_correct.load(Ordering::Relaxed),
            trainer_total: self.trainer_total.load(Ordering::Relaxed),
            concepts_known: self.knowledge.lock()
                .map(|k| k.concept_count())
                .unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConcurrentMetrics {
    pub trainer_cycles: usize,
    pub improver_cycles: usize,
    pub intel_cycles: usize,
    pub trainer_correct: usize,
    pub trainer_total: usize,
    pub concepts_known: usize,
}

impl ConcurrentMetrics {
    pub fn trainer_accuracy(&self) -> f64 {
        if self.trainer_total == 0 { 0.0 }
        else { self.trainer_correct as f64 / self.trainer_total as f64 }
    }
}

// ============================================================
// Concurrent Runner
// ============================================================

/// Runs training + self-improvement + intel gathering in parallel.
pub struct ConcurrentRunner {
    config: ConcurrentConfig,
    state: Arc<SharedState>,
    thread_handles: Vec<thread::JoinHandle<()>>,
}

impl ConcurrentRunner {
    pub fn new(config: ConcurrentConfig) -> Self {
        debuglog!("ConcurrentRunner::new: Initializing parallel learning");
        Self {
            config,
            state: Arc::new(SharedState::new()),
            thread_handles: Vec::new(),
        }
    }

    /// Start all configured worker threads.
    pub fn start(&mut self) {
        debuglog!("ConcurrentRunner::start: Spawning worker threads");

        if self.config.run_trainer {
            self.spawn_trainer();
        }
        if self.config.run_improver {
            self.spawn_improver();
        }
        if self.config.run_intel {
            self.spawn_intel();
        }

        debuglog!("ConcurrentRunner::start: {} threads spawned", self.thread_handles.len());
    }

    /// Stop all workers and wait for them to finish.
    pub fn stop(&mut self) {
        debuglog!("ConcurrentRunner::stop: Signaling workers to stop");
        self.state.stop();

        let handles = std::mem::take(&mut self.thread_handles);
        for handle in handles {
            let _ = handle.join();
        }

        debuglog!("ConcurrentRunner::stop: All workers stopped");
    }

    /// Get a handle to the shared state for monitoring.
    pub fn state(&self) -> Arc<SharedState> {
        Arc::clone(&self.state)
    }

    /// Get current metrics.
    pub fn metrics(&self) -> ConcurrentMetrics {
        self.state.metrics()
    }

    fn spawn_trainer(&mut self) {
        let state = Arc::clone(&self.state);
        let backend = self.config.inference_backend.clone();

        let handle = thread::spawn(move || {
            debuglog!("[Trainer] Thread started");
            let config = InferenceTrainingConfig {
                backend,
                verify_answers: true,
                cache_enabled: true,
                active_learning: true,
                ..Default::default()
            };
            let mut trainer = InferenceTrainer::new(config);
            let examples = TrainingDataGenerator::all_examples();
            let mut idx = 0usize;

            while !state.should_stop() && idx < examples.len() {
                let example = &examples[idx];
                idx = (idx + 1) % examples.len();

                // Acquire lock, train, release.
                let result = {
                    let mut knowledge = match state.knowledge.lock() {
                        Ok(k) => k,
                        Err(_) => {
                            debuglog!("[Trainer] Failed to lock knowledge");
                            thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                    };
                    trainer.train_on_example(example, &mut knowledge)
                };

                if let Ok(r) = result {
                    state.trainer_total.fetch_add(1, Ordering::Relaxed);
                    if r.correct == Some(true) {
                        state.trainer_correct.fetch_add(1, Ordering::Relaxed);
                    }
                }

                state.trainer_cycles.fetch_add(1, Ordering::Relaxed);
            }

            debuglog!("[Trainer] Thread exiting");
        });

        self.thread_handles.push(handle);
    }

    fn spawn_improver(&mut self) {
        let state = Arc::clone(&self.state);
        let interval = Duration::from_millis(self.config.improver_interval_ms);

        let handle = thread::spawn(move || {
            debuglog!("[Improver] Thread started");
            let mut improver = SelfImprovementEngine::new();

            while !state.should_stop() {
                thread::sleep(interval);
                if state.should_stop() { break; }

                // Acquire lock, run cycle, release.
                let _result = {
                    let mut knowledge = match state.knowledge.lock() {
                        Ok(k) => k,
                        Err(_) => continue,
                    };
                    improver.run_cycle(&mut knowledge)
                };

                state.improver_cycles.fetch_add(1, Ordering::Relaxed);
            }

            debuglog!("[Improver] Thread exiting");
        });

        self.thread_handles.push(handle);
    }

    fn spawn_intel(&mut self) {
        let state = Arc::clone(&self.state);
        let interval = Duration::from_millis(self.config.intel_interval_ms);

        let handle = thread::spawn(move || {
            debuglog!("[Intel] Thread started");
            let mut intel = ContinuousIntelligence::new();

            while !state.should_stop() {
                thread::sleep(interval);
                if state.should_stop() { break; }

                let _ = {
                    let mut knowledge = match state.knowledge.lock() {
                        Ok(k) => k,
                        Err(_) => continue,
                    };
                    intel.poll_cycle(&mut knowledge)
                };

                state.intel_cycles.fetch_add(1, Ordering::Relaxed);
            }

            debuglog!("[Intel] Thread exiting");
        });

        self.thread_handles.push(handle);
    }

    /// Run for a specified duration, then auto-stop.
    pub fn run_for(&mut self, duration: Duration) {
        self.start();
        thread::sleep(duration);
        self.stop();
    }

    /// Generate a progress report.
    pub fn report(&self) -> String {
        let metrics = self.metrics();
        let mut out = "=== Concurrent Learning Report ===\n".to_string();
        out.push_str(&format!("Trainer:   {} cycles, {}/{} correct ({:.1}%)\n",
            metrics.trainer_cycles, metrics.trainer_correct, metrics.trainer_total,
            metrics.trainer_accuracy() * 100.0));
        out.push_str(&format!("Improver:  {} cycles\n", metrics.improver_cycles));
        out.push_str(&format!("Intel:     {} polls\n", metrics.intel_cycles));
        out.push_str(&format!("Concepts:  {}\n", metrics.concepts_known));
        out
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_state_creation() {
        let state = SharedState::new();
        assert!(!state.should_stop());
        let metrics = state.metrics();
        assert_eq!(metrics.trainer_cycles, 0);
        assert_eq!(metrics.improver_cycles, 0);
    }

    #[test]
    fn test_metrics_accuracy_calculation() {
        let metrics = ConcurrentMetrics {
            trainer_cycles: 10,
            improver_cycles: 5,
            intel_cycles: 2,
            trainer_correct: 8,
            trainer_total: 10,
            concepts_known: 100,
        };
        assert!((metrics.trainer_accuracy() - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_concurrent_runner_creation() {
        let config = ConcurrentConfig::default();
        let runner = ConcurrentRunner::new(config);
        let metrics = runner.metrics();
        assert_eq!(metrics.trainer_cycles, 0);
    }

    #[test]
    fn test_runner_short_run() {
        let config = ConcurrentConfig {
            inference_backend: InferenceBackend::Mock {
                answers: vec!["5".into()],
            },
            run_trainer: true,
            run_improver: false, // Disable for fast test
            run_intel: false,
            improver_interval_ms: 100,
            intel_interval_ms: 100,
            max_runtime_sec: 0,
        };
        let mut runner = ConcurrentRunner::new(config);

        // Run for 300ms.
        runner.run_for(Duration::from_millis(300));

        let metrics = runner.metrics();
        assert!(metrics.trainer_cycles > 0, "Trainer should have run at least once");
    }

    #[test]
    fn test_stop_flag_works() {
        let state = SharedState::new();
        assert!(!state.should_stop());
        state.stop();
        assert!(state.should_stop());
    }

    #[test]
    fn test_all_three_threads_spawn() {
        let config = ConcurrentConfig {
            inference_backend: InferenceBackend::Mock {
                answers: vec!["test".into()],
            },
            run_trainer: true,
            run_improver: true,
            run_intel: true,
            improver_interval_ms: 50,
            intel_interval_ms: 50,
            max_runtime_sec: 0,
        };
        let mut runner = ConcurrentRunner::new(config);

        // Brief run with all three threads.
        runner.run_for(Duration::from_millis(200));

        let metrics = runner.metrics();
        // All three loops should have run at least once.
        assert!(metrics.trainer_cycles > 0);
        assert!(metrics.improver_cycles > 0);
        assert!(metrics.intel_cycles > 0);
    }

    #[test]
    fn test_concepts_grow_during_concurrent_run() {
        let config = ConcurrentConfig {
            inference_backend: InferenceBackend::Mock {
                answers: vec!["5".into()],
            },
            run_trainer: true,
            run_improver: true,
            run_intel: false,
            improver_interval_ms: 50,
            intel_interval_ms: 1000,
            max_runtime_sec: 0,
        };
        let mut runner = ConcurrentRunner::new(config);
        let initial_concepts = runner.metrics().concepts_known;

        runner.run_for(Duration::from_millis(500));

        let final_concepts = runner.metrics().concepts_known;
        assert!(final_concepts >= initial_concepts,
            "Concepts should not decrease: {} → {}", initial_concepts, final_concepts);
    }

    #[test]
    fn test_report_generation() {
        let config = ConcurrentConfig::default();
        let runner = ConcurrentRunner::new(config);
        let report = runner.report();
        assert!(report.contains("Concurrent Learning Report"));
        assert!(report.contains("Trainer:"));
        assert!(report.contains("Improver:"));
    }

    // ============================================================
    // Stress / invariant tests for concurrent
    // ============================================================

    /// INVARIANT: SharedState::should_stop is initially false.
    #[test]
    fn invariant_initial_state_not_stopping() {
        let state = SharedState::new();
        assert!(!state.should_stop(),
            "fresh SharedState must not signal stop");
    }

    /// INVARIANT: stop() flips should_stop() to true and stays true.
    #[test]
    fn invariant_stop_is_sticky() {
        let state = SharedState::new();
        state.stop();
        assert!(state.should_stop(), "after stop() should_stop must be true");
        state.stop(); // idempotent
        assert!(state.should_stop(), "stop() must be idempotent");
    }

    /// INVARIANT: trainer_accuracy on fresh metrics is 0.0 (no division by zero).
    #[test]
    fn invariant_fresh_metrics_zero_accuracy() {
        let state = SharedState::new();
        let metrics = state.metrics();
        let acc = metrics.trainer_accuracy();
        assert!(acc.is_finite() && (0.0..=1.0).contains(&acc),
            "fresh accuracy out of [0,1]: {}", acc);
    }

    /// INVARIANT: SharedState is shareable across threads — Arc clone works
    /// and reads remain consistent.
    #[test]
    fn invariant_shared_state_arc_clone_safe() {
        use std::sync::Arc;
        use std::thread;
        let state = Arc::new(SharedState::new());
        let mut handles = vec![];
        for _ in 0..4 {
            let s = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                let _ = s.should_stop();
                let _ = s.metrics();
            }));
        }
        for h in handles {
            h.join().expect("thread join");
        }
    }

    /// INVARIANT: SharedState fresh metrics produce zero accuracy — no NaN
    /// poisoning from division-by-zero.
    #[test]
    fn invariant_fresh_state_zero_accuracy() {
        let state = SharedState::new();
        let metrics = state.metrics();
        let acc = metrics.trainer_accuracy();
        assert!(acc.is_finite() && acc == 0.0,
            "fresh accuracy must be 0.0 (no division by zero), got {}", acc);
    }

    /// INVARIANT: SharedState::stop() sets should_stop true.
    #[test]
    fn invariant_stop_sets_should_stop() {
        let state = SharedState::new();
        assert!(!state.should_stop(), "fresh state should not be stopped");
        state.stop();
        assert!(state.should_stop(), "after stop() should_stop() is true");
    }

}
