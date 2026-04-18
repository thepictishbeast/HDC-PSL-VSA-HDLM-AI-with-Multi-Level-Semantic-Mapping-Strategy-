// ============================================================
// RAG Trainer — Retrieval-Augmented Generation Improvement
//
// Tracks which retrieved facts lead to good vs bad responses.
// Over time, upweights facts that consistently improve answers
// and downweights facts that lead to poor responses.
//
// SUPERSOCIETY: Not all 59M facts are equally useful for RAG.
// The ones that consistently help should be retrieved first.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use crate::persistence::BrainDb;

/// Track how a fact performed when used in RAG context.
#[derive(Debug, Clone)]
pub struct RagSignal {
    pub fact_key: String,
    pub was_helpful: bool,
    pub confidence: f64,
    pub user_feedback: Option<bool>, // true=positive, false=negative
}

/// RAG fact effectiveness tracker.
pub struct RagTrainer {
    /// Per-fact signal counts.
    signals: HashMap<String, FactSignals>,
    db: Arc<BrainDb>,
}

#[derive(Debug, Clone, Default)]
struct FactSignals {
    times_retrieved: usize,
    positive_outcomes: usize,
    negative_outcomes: usize,
    total_confidence: f64,
}

impl FactSignals {
    fn effectiveness(&self) -> f64 {
        if self.times_retrieved < 3 {
            return 0.5; // Not enough data
        }
        let total = self.positive_outcomes + self.negative_outcomes;
        if total == 0 {
            return self.total_confidence / self.times_retrieved.max(1) as f64;
        }
        self.positive_outcomes as f64 / total as f64
    }
}

impl RagTrainer {
    pub fn new(db: Arc<BrainDb>) -> Self {
        Self {
            signals: HashMap::new(),
            db,
        }
    }

    /// Record a RAG retrieval signal.
    pub fn record(&mut self, signal: RagSignal) {
        let entry = self.signals.entry(signal.fact_key).or_default();
        entry.times_retrieved += 1;
        entry.total_confidence += signal.confidence;
        if signal.was_helpful {
            entry.positive_outcomes += 1;
        }
        if let Some(false) = signal.user_feedback {
            entry.negative_outcomes += 1;
        }
    }

    /// Get the RAG boost factor for a fact (multiply with search rank).
    /// Facts with high effectiveness get boosted, low get penalized.
    pub fn boost_factor(&self, fact_key: &str) -> f64 {
        match self.signals.get(fact_key) {
            Some(s) => {
                let eff = s.effectiveness();
                // Map 0.0-1.0 to 0.5-1.5 boost
                0.5 + eff
            }
            None => 1.0, // Unknown fact — neutral boost
        }
    }

    /// Get top-performing facts for RAG (most effective when retrieved).
    pub fn top_facts(&self, n: usize) -> Vec<(String, f64, usize)> {
        let mut ranked: Vec<_> = self.signals.iter()
            .filter(|(_, s)| s.times_retrieved >= 3)
            .map(|(key, s)| (key.clone(), s.effectiveness(), s.times_retrieved))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(n);
        ranked
    }

    /// Get worst-performing facts (candidates for quality improvement or removal).
    pub fn worst_facts(&self, n: usize) -> Vec<(String, f64, usize)> {
        let mut ranked: Vec<_> = self.signals.iter()
            .filter(|(_, s)| s.times_retrieved >= 3)
            .map(|(key, s)| (key.clone(), s.effectiveness(), s.times_retrieved))
            .collect();
        ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(n);
        ranked
    }

    /// Apply learned effectiveness back to fact quality scores.
    /// Facts that consistently help get quality boost, consistently
    /// hurt get quality penalty.
    pub fn apply_to_quality_scores(&self) {
        let conn = self.db.conn.lock().unwrap_or_else(|e| e.into_inner());
        for (key, signals) in &self.signals {
            if signals.times_retrieved < 5 { continue; }
            let eff = signals.effectiveness();
            // Only adjust if clearly good or bad
            if eff > 0.7 {
                // Boost quality by up to 0.1
                let _ = conn.execute(
                    "UPDATE facts SET quality_score = MIN(1.0, COALESCE(quality_score, 0.5) + 0.05) WHERE key = ?1",
                    rusqlite::params![key],
                );
            } else if eff < 0.3 {
                // Penalize quality by up to 0.1
                let _ = conn.execute(
                    "UPDATE facts SET quality_score = MAX(0.1, COALESCE(quality_score, 0.5) - 0.05) WHERE key = ?1",
                    rusqlite::params![key],
                );
            }
        }
    }

    /// Stats summary.
    pub fn stats(&self) -> RagStats {
        let tracked = self.signals.len();
        let evaluated = self.signals.values().filter(|s| s.times_retrieved >= 3).count();
        let avg_eff = if evaluated > 0 {
            self.signals.values()
                .filter(|s| s.times_retrieved >= 3)
                .map(|s| s.effectiveness())
                .sum::<f64>() / evaluated as f64
        } else { 0.0 };

        RagStats { tracked_facts: tracked, evaluated_facts: evaluated, avg_effectiveness: avg_eff }
    }
}

#[derive(Debug)]
pub struct RagStats {
    pub tracked_facts: usize,
    pub evaluated_facts: usize,
    pub avg_effectiveness: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_trainer() -> RagTrainer {
        let path = PathBuf::from(format!("/tmp/plausiden_test_rag_{}.db",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let db = Arc::new(BrainDb::open(&path).unwrap());
        RagTrainer::new(db)
    }

    #[test]
    fn test_record_and_boost() {
        let mut t = test_trainer();
        for _ in 0..5 {
            t.record(RagSignal { fact_key: "good_fact".into(), was_helpful: true, confidence: 0.9, user_feedback: Some(true) });
        }
        for _ in 0..5 {
            t.record(RagSignal { fact_key: "bad_fact".into(), was_helpful: false, confidence: 0.3, user_feedback: Some(false) });
        }
        assert!(t.boost_factor("good_fact") > 1.0);
        assert!(t.boost_factor("bad_fact") < 1.0);
        assert_eq!(t.boost_factor("unknown"), 1.0);
    }

    #[test]
    fn test_top_and_worst() {
        let mut t = test_trainer();
        for _ in 0..5 { t.record(RagSignal { fact_key: "best".into(), was_helpful: true, confidence: 0.9, user_feedback: Some(true) }); }
        for _ in 0..5 { t.record(RagSignal { fact_key: "worst".into(), was_helpful: false, confidence: 0.2, user_feedback: Some(false) }); }

        let top = t.top_facts(1);
        assert_eq!(top[0].0, "best");
        let worst = t.worst_facts(1);
        assert_eq!(worst[0].0, "worst");
    }

    #[test]
    fn test_stats() {
        let mut t = test_trainer();
        for _ in 0..5 { t.record(RagSignal { fact_key: "f1".into(), was_helpful: true, confidence: 0.8, user_feedback: None }); }
        let stats = t.stats();
        assert_eq!(stats.tracked_facts, 1);
        assert_eq!(stats.evaluated_facts, 1);
    }
}
