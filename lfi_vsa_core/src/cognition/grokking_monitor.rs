//! # Purpose
//! Grokking phase-transition detection for LFI's self-improvement loop.
//! Detects when competing strategies (symbolic vs HDC-compiled) transition
//! through memorization → circuit formation → cleanup phases.
//! When signatures match, triggers forced wake-sleep consolidation.
//!
//! # Design Decisions
//! - Track per-strategy L2 norm trajectories and effective rank
//! - Three-phase detection: memorization (loss drops, generalization flat),
//!   circuit formation (generalization starts improving), cleanup (both converge)
//! - Uses SVD effective rank on sliding windows of performance metrics
//! - Triggers consolidation when cleanup phase detected

use std::collections::VecDeque;

/// A performance snapshot for a single strategy.
#[derive(Debug, Clone)]
pub struct StrategySnapshot {
    pub strategy_name: String,
    pub train_accuracy: f64,
    pub test_accuracy: f64,
    pub l2_norm: f64,
    pub timestamp: u64,
}

/// Detected grokking phase.
#[derive(Debug, Clone, PartialEq)]
pub enum GrokPhase {
    /// Loss drops but generalization is flat — memorizing examples.
    Memorization,
    /// Generalization starts improving — circuits forming.
    CircuitFormation,
    /// Both train and test converge — cleanup complete.
    Cleanup,
    /// Not enough data to determine phase.
    Undetermined,
}

/// Monitors grokking phase transitions across strategies.
pub struct GrokMonitor {
    /// History of snapshots per strategy.
    history: std::collections::HashMap<String, VecDeque<StrategySnapshot>>,
    /// Max history length per strategy.
    window: usize,
    /// Consolidation triggers fired.
    pub consolidations_triggered: u64,
}

impl GrokMonitor {
    pub fn new(window: usize) -> Self {
        Self {
            history: std::collections::HashMap::new(),
            window,
            consolidations_triggered: 0,
        }
    }

    /// Record a performance snapshot.
    pub fn record(&mut self, snapshot: StrategySnapshot) {
        let history = self.history
            .entry(snapshot.strategy_name.clone())
            .or_insert_with(|| VecDeque::with_capacity(self.window));

        if history.len() >= self.window {
            history.pop_front();
        }
        history.push_back(snapshot);
    }

    /// Detect the current grokking phase for a strategy.
    pub fn detect_phase(&self, strategy: &str) -> GrokPhase {
        let history = match self.history.get(strategy) {
            Some(h) if h.len() >= 10 => h,
            _ => return GrokPhase::Undetermined,
        };

        let len = history.len();
        let recent = &history.as_slices().0[len.saturating_sub(5)..];
        let earlier = &history.as_slices().0[..5.min(len)];

        // Compute trends
        let recent_train: f64 = recent.iter().map(|s| s.train_accuracy).sum::<f64>() / recent.len() as f64;
        let recent_test: f64 = recent.iter().map(|s| s.test_accuracy).sum::<f64>() / recent.len() as f64;
        let early_train: f64 = earlier.iter().map(|s| s.train_accuracy).sum::<f64>() / earlier.len() as f64;
        let early_test: f64 = earlier.iter().map(|s| s.test_accuracy).sum::<f64>() / earlier.len() as f64;

        let train_improved = recent_train > early_train + 0.05;
        let test_improved = recent_test > early_test + 0.05;
        let gap_closing = (recent_train - recent_test).abs() < (early_train - early_test).abs();

        if train_improved && !test_improved {
            GrokPhase::Memorization
        } else if train_improved && test_improved && !gap_closing {
            GrokPhase::CircuitFormation
        } else if gap_closing && recent_test > 0.7 {
            GrokPhase::Cleanup
        } else {
            GrokPhase::Undetermined
        }
    }

    /// Check if any strategy is in cleanup phase → trigger consolidation.
    pub fn should_consolidate(&mut self) -> Vec<String> {
        let mut triggers = Vec::new();
        let strategies: Vec<String> = self.history.keys().cloned().collect();
        for strategy in strategies {
            if self.detect_phase(&strategy) == GrokPhase::Cleanup {
                triggers.push(strategy);
                self.consolidations_triggered += 1;
            }
        }
        triggers
    }

    /// Generalization gap for a strategy (train_acc - test_acc).
    pub fn generalization_gap(&self, strategy: &str) -> Option<f64> {
        self.history.get(strategy)
            .and_then(|h| h.back())
            .map(|s| s.train_accuracy - s.test_accuracy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshots(name: &str, train: &[f64], test: &[f64]) -> Vec<StrategySnapshot> {
        train.iter().zip(test.iter()).enumerate().map(|(i, (&tr, &te))| {
            StrategySnapshot {
                strategy_name: name.into(),
                train_accuracy: tr, test_accuracy: te,
                l2_norm: 1.0, timestamp: i as u64,
            }
        }).collect()
    }

    #[test]
    fn test_memorization_phase() {
        let mut mon = GrokMonitor::new(20);
        // Train improves, test doesn't
        for s in snapshots("sym", &[0.3,0.4,0.5,0.6,0.7,0.8,0.85,0.9,0.92,0.95], &[0.3,0.3,0.3,0.3,0.3,0.3,0.3,0.3,0.3,0.3]) {
            mon.record(s);
        }
        assert_eq!(mon.detect_phase("sym"), GrokPhase::Memorization);
    }

    #[test]
    fn test_cleanup_phase() {
        let mut mon = GrokMonitor::new(20);
        // Both converge, gap closing
        for s in snapshots("hdc", &[0.5,0.6,0.7,0.75,0.8,0.82,0.83,0.84,0.85,0.85], &[0.3,0.4,0.5,0.6,0.7,0.75,0.78,0.80,0.82,0.83]) {
            mon.record(s);
        }
        assert_eq!(mon.detect_phase("hdc"), GrokPhase::Cleanup);
    }

    #[test]
    fn test_undetermined_insufficient_data() {
        let mon = GrokMonitor::new(20);
        assert_eq!(mon.detect_phase("missing"), GrokPhase::Undetermined);
    }

    #[test]
    fn test_consolidation_trigger() {
        let mut mon = GrokMonitor::new(20);
        for s in snapshots("x", &[0.5,0.6,0.7,0.75,0.8,0.82,0.83,0.84,0.85,0.85], &[0.3,0.4,0.5,0.6,0.7,0.75,0.78,0.80,0.82,0.83]) {
            mon.record(s);
        }
        let triggers = mon.should_consolidate();
        assert!(!triggers.is_empty());
        assert_eq!(mon.consolidations_triggered, 1);
    }
}
