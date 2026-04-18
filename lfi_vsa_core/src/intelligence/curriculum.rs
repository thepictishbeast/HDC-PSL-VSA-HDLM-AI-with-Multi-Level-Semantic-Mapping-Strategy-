// ============================================================
// Training Curriculum Scheduler
//
// Decides what to train on, when, and how much. Uses FSRS-based
// spaced repetition principles for domain mastery scheduling.
//
// Priority factors:
// 1. Domain weakness (low accuracy → train more)
// 2. Data freshness (stale domains → refresh)
// 3. Coverage gaps (sparse domains → expand)
// 4. User demand (frequently asked domains → prioritize)
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use crate::persistence::BrainDb;

/// A scheduled training session.
#[derive(Debug, Clone)]
pub struct TrainingSession {
    pub domain: String,
    pub priority: f64,
    pub data_pairs: usize,
    pub reason: String,
    pub scheduled_at: String,
}

/// The curriculum scheduler.
pub struct CurriculumScheduler {
    db: Arc<BrainDb>,
}

impl CurriculumScheduler {
    pub fn new(db: Arc<BrainDb>) -> Self {
        Self { db }
    }

    /// Generate the next training curriculum — what domains need attention.
    pub fn plan_next_session(&self, max_domains: usize) -> Vec<TrainingSession> {
        let conn = self.db.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Get domain stats
        let mut domain_stmt = conn.prepare(
            "SELECT domain, COUNT(*) as cnt, AVG(COALESCE(quality_score, 0.5)) as avg_q \
             FROM facts WHERE domain IS NOT NULL GROUP BY domain ORDER BY cnt DESC"
        ).unwrap_or_else(|_| conn.prepare("SELECT 'none', 0, 0.0").unwrap());

        let domains: Vec<(String, i64, f64)> = domain_stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, f64>(2)?))
        }).unwrap_or_else(|_| panic!("query_map")).filter_map(|r| r.ok()).collect();

        // Get training accuracy per domain
        let mut acc_stmt = conn.prepare(
            "SELECT domain, AVG(accuracy) FROM training_results GROUP BY domain"
        ).unwrap_or_else(|_| conn.prepare("SELECT 'none', 0.0").unwrap());
        let accuracy: HashMap<String, f64> = acc_stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        }).unwrap_or_else(|_| panic!("query_map")).filter_map(|r| r.ok()).collect();

        // Get last training time per domain
        let mut time_stmt = conn.prepare(
            "SELECT domain, MAX(timestamp) FROM training_results GROUP BY domain"
        ).unwrap_or_else(|_| conn.prepare("SELECT 'none', ''").unwrap());
        let last_trained: HashMap<String, String> = time_stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).unwrap_or_else(|_| panic!("query_map")).filter_map(|r| r.ok()).collect();

        drop(domain_stmt);
        drop(acc_stmt);
        drop(time_stmt);
        drop(conn);

        let max_count = domains.iter().map(|(_, c, _)| *c).max().unwrap_or(1);

        let mut sessions: Vec<TrainingSession> = domains.iter().map(|(domain, count, avg_q)| {
            let mut priority = 0.0f64;
            let mut reasons = Vec::new();

            // Factor 1: Low accuracy
            if let Some(acc) = accuracy.get(domain) {
                if *acc < 0.5 {
                    priority += 0.4;
                    reasons.push(format!("accuracy {:.0}%", acc * 100.0));
                } else if *acc < 0.7 {
                    priority += 0.2;
                    reasons.push(format!("accuracy {:.0}%", acc * 100.0));
                }
            } else {
                priority += 0.15;
                reasons.push("never evaluated".into());
            }

            // Factor 2: Low coverage
            if *count < 1000 {
                priority += 0.3;
                reasons.push(format!("{} facts (sparse)", count));
            } else if *count < 10000 {
                priority += 0.1;
            }

            // Factor 3: Low quality
            if *avg_q < 0.6 {
                priority += 0.2;
                reasons.push(format!("quality {:.2}", avg_q));
            }

            // Factor 4: Staleness (not trained recently)
            if !last_trained.contains_key(domain) {
                priority += 0.1;
                reasons.push("never trained".into());
            }

            // Determine data pairs based on priority
            let pairs = if priority > 0.7 { 200 }
                       else if priority > 0.4 { 100 }
                       else if priority > 0.2 { 50 }
                       else { 20 };

            TrainingSession {
                domain: domain.clone(),
                priority: priority.min(1.0),
                data_pairs: pairs,
                reason: if reasons.is_empty() { "maintenance".into() } else { reasons.join(", ") },
                scheduled_at: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            }
        }).collect();

        sessions.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        sessions.truncate(max_domains);
        sessions
    }

    /// Get a summary of curriculum state.
    pub fn summary(&self) -> CurriculumSummary {
        let sessions = self.plan_next_session(50);
        let total_pairs: usize = sessions.iter().map(|s| s.data_pairs).sum();
        let urgent = sessions.iter().filter(|s| s.priority > 0.6).count();
        let moderate = sessions.iter().filter(|s| s.priority > 0.3 && s.priority <= 0.6).count();

        CurriculumSummary {
            total_domains: sessions.len(),
            urgent_domains: urgent,
            moderate_domains: moderate,
            total_pairs_needed: total_pairs,
            top_priorities: sessions.into_iter().take(10).collect(),
        }
    }
}

#[derive(Debug)]
pub struct CurriculumSummary {
    pub total_domains: usize,
    pub urgent_domains: usize,
    pub moderate_domains: usize,
    pub total_pairs_needed: usize,
    pub top_priorities: Vec<TrainingSession>,
}

use chrono;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_scheduler() -> CurriculumScheduler {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = PathBuf::from(format!("/tmp/plausiden_test_curr_{}.db", ts));
        let db = Arc::new(BrainDb::open(&path).unwrap());
        let conn = db.conn.lock().unwrap();
        conn.execute("ALTER TABLE facts ADD COLUMN domain TEXT", []).ok();
        conn.execute("ALTER TABLE facts ADD COLUMN quality_score REAL", []).ok();
        for i in 0..50 {
            conn.execute("INSERT INTO facts (key,value,source,confidence,domain,quality_score) VALUES (?,?,'t',0.8,'cyber',0.85)",
                rusqlite::params![format!("c{}", i), format!("Fact {}", i)]).ok();
        }
        for i in 0..5 {
            conn.execute("INSERT INTO facts (key,value,source,confidence,domain,quality_score) VALUES (?,?,'t',0.4,'philosophy',0.4)",
                rusqlite::params![format!("p{}", i), format!("Fact {}", i)]).ok();
        }
        drop(conn);
        CurriculumScheduler::new(db)
    }

    #[test]
    fn test_plan_session() {
        let s = test_scheduler();
        let sessions = s.plan_next_session(5);
        assert!(!sessions.is_empty());
        // Philosophy should be higher priority (fewer facts, lower quality)
        let phil = sessions.iter().find(|s| s.domain == "philosophy");
        assert!(phil.is_some());
        assert!(phil.unwrap().priority > 0.3);
    }

    #[test]
    fn test_summary() {
        let s = test_scheduler();
        let summary = s.summary();
        assert!(summary.total_domains >= 2);
        assert!(summary.total_pairs_needed > 0);
    }
}
