// ============================================================
// Domain Gap Analyzer — Find what PlausiDen doesn't know
//
// Scans the knowledge base to identify:
// 1. Domains with very few facts (sparse coverage)
// 2. Domains with low average quality (weak knowledge)
// 3. Expected domains that don't exist yet (blind spots)
// 4. Queries that found zero relevant facts (live gaps)
//
// Output feeds into the Magpie pipeline to auto-generate
// training data for gap domains.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use crate::persistence::BrainDb;

/// A domain's coverage profile.
#[derive(Debug, Clone)]
pub struct DomainProfile {
    pub domain: String,
    pub fact_count: i64,
    pub avg_quality: f64,
    pub coverage_tier: CoverageTier,
}

/// Coverage classification.
#[derive(Debug, Clone, PartialEq)]
pub enum CoverageTier {
    /// 100K+ facts — deep coverage
    Deep,
    /// 10K-100K — good coverage
    Good,
    /// 1K-10K — moderate coverage
    Moderate,
    /// 100-1K — sparse coverage, needs more data
    Sparse,
    /// <100 — critical gap
    Critical,
    /// Domain doesn't exist yet
    Missing,
}

impl CoverageTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deep => "deep",
            Self::Good => "good",
            Self::Moderate => "moderate",
            Self::Sparse => "sparse",
            Self::Critical => "critical",
            Self::Missing => "missing",
        }
    }

    fn from_count(count: i64) -> Self {
        match count {
            c if c >= 100_000 => Self::Deep,
            c if c >= 10_000 => Self::Good,
            c if c >= 1_000 => Self::Moderate,
            c if c >= 100 => Self::Sparse,
            _ => Self::Critical,
        }
    }
}

/// Full gap analysis report.
#[derive(Debug, Clone)]
pub struct GapReport {
    pub total_domains: usize,
    pub total_facts: i64,
    pub profiles: Vec<DomainProfile>,
    pub gaps: Vec<DomainProfile>,
    pub missing_domains: Vec<String>,
    pub weakest_quality: Vec<DomainProfile>,
    pub recommended_actions: Vec<String>,
}

/// Expected domains that a comprehensive AI should cover.
const EXPECTED_DOMAINS: &[&str] = &[
    "cybersecurity", "programming", "mathematics", "physics", "biology",
    "chemistry", "medicine", "history", "geography", "economics",
    "philosophy", "psychology", "sociology", "law", "politics",
    "literature", "music", "art", "architecture", "engineering",
    "astronomy", "geology", "oceanography", "meteorology", "ecology",
    "linguistics", "anthropology", "archaeology", "paleontology",
    "nutrition", "agriculture", "robotics", "quantum_computing",
    "blockchain", "cryptography", "game_theory", "statistics",
    "logic", "ethics", "religion", "mythology", "sports",
    "cooking", "travel", "finance", "business", "marketing",
    "education", "parenting", "relationships", "mental_health",
];

pub struct DomainGapAnalyzer {
    db: Arc<BrainDb>,
}

impl DomainGapAnalyzer {
    pub fn new(db: Arc<BrainDb>) -> Self {
        Self { db }
    }

    /// Run full gap analysis.
    pub fn analyze(&self) -> GapReport {
        let conn = self.db.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Get existing domain stats
        let mut stmt = conn.prepare(
            "SELECT domain, COUNT(*) as cnt, ROUND(AVG(COALESCE(quality_score, 0.5)), 3) as avg_q \
             FROM facts WHERE domain IS NOT NULL \
             GROUP BY domain ORDER BY cnt DESC"
        ).unwrap_or_else(|_| conn.prepare("SELECT 'none', 0, 0.0").unwrap());

        let profiles: Vec<DomainProfile> = stmt.query_map([], |row| {
            let domain: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            let quality: f64 = row.get(2)?;
            Ok(DomainProfile {
                coverage_tier: CoverageTier::from_count(count),
                domain,
                fact_count: count,
                avg_quality: quality,
            })
        }).unwrap_or_else(|_| panic!("query_map failed"))
          .filter_map(|r| r.ok())
          .collect();

        let total_facts: i64 = profiles.iter().map(|p| p.fact_count).sum();
        let existing_domains: std::collections::HashSet<String> = profiles.iter()
            .map(|p| p.domain.clone()).collect();

        // Find gaps (sparse or critical coverage)
        let gaps: Vec<DomainProfile> = profiles.iter()
            .filter(|p| matches!(p.coverage_tier, CoverageTier::Sparse | CoverageTier::Critical))
            .cloned()
            .collect();

        // Find missing domains
        let missing: Vec<String> = EXPECTED_DOMAINS.iter()
            .filter(|d| !existing_domains.contains(**d))
            .map(|d| d.to_string())
            .collect();

        // Weakest quality domains (avg < 0.6 with >100 facts)
        let mut weakest: Vec<DomainProfile> = profiles.iter()
            .filter(|p| p.avg_quality < 0.6 && p.fact_count > 100)
            .cloned()
            .collect();
        weakest.sort_by(|a, b| a.avg_quality.partial_cmp(&b.avg_quality).unwrap_or(std::cmp::Ordering::Equal));

        // Generate recommendations
        let mut recommendations = Vec::new();
        for d in &missing {
            recommendations.push(format!("MISSING: Generate training data for '{}' domain", d));
        }
        for p in gaps.iter().take(5) {
            recommendations.push(format!("SPARSE: '{}' has only {} facts — need 10x more", p.domain, p.fact_count));
        }
        for p in weakest.iter().take(5) {
            recommendations.push(format!("QUALITY: '{}' avg quality {:.2} — review and upgrade data sources", p.domain, p.avg_quality));
        }

        GapReport {
            total_domains: profiles.len(),
            total_facts,
            profiles,
            gaps,
            missing_domains: missing,
            weakest_quality: weakest,
            recommended_actions: recommendations,
        }
    }

    /// Generate Magpie prompts for gap domains.
    pub fn generate_gap_prompts(&self, max_prompts: usize) -> Vec<(String, String)> {
        let report = self.analyze();
        let mut prompts = Vec::new();

        // Missing domains first
        for domain in &report.missing_domains {
            if prompts.len() >= max_prompts { break; }
            prompts.push((
                domain.clone(),
                format!("Ask a detailed question about {} and provide an expert answer.\nQ:", domain),
            ));
        }

        // Then sparse domains
        for gap in &report.gaps {
            if prompts.len() >= max_prompts { break; }
            prompts.push((
                gap.domain.clone(),
                format!("Ask a challenging question about {} and provide a thorough answer.\nQ:", gap.domain),
            ));
        }

        prompts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_analyzer() -> DomainGapAnalyzer {
        let id = std::process::id();
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = PathBuf::from(format!("/tmp/plausiden_test_gap_{}_{}.db", id, ts));
        let db = Arc::new(BrainDb::open(&path).unwrap());
        // Ensure domain column exists (migration may not have added it)
        {
            let conn = db.conn.lock().unwrap();
            conn.execute("ALTER TABLE facts ADD COLUMN domain TEXT", []).ok();
            conn.execute("ALTER TABLE facts ADD COLUMN quality_score REAL", []).ok();
        }
        // Seed with test facts in a few domains — use direct SQL to set domain
        {
            let conn = db.conn.lock().unwrap();
            for i in 0..50 {
                conn.execute(
                    "INSERT OR REPLACE INTO facts (key, value, source, confidence, domain, quality_score) VALUES (?, ?, 'test', 0.8, 'cybersecurity', 0.8)",
                    rusqlite::params![format!("cyber_{}", i), format!("Cybersecurity fact {}", i)]
                ).ok();
            }
            for i in 0..5 {
                conn.execute(
                    "INSERT OR REPLACE INTO facts (key, value, source, confidence, domain, quality_score) VALUES (?, ?, 'test', 0.6, 'philosophy', 0.6)",
                    rusqlite::params![format!("philo_{}", i), format!("Philosophy fact {}", i)]
                ).ok();
            }
        }
        DomainGapAnalyzer::new(db)
    }

    #[test]
    fn test_gap_analysis() {
        let a = test_analyzer();
        let report = a.analyze();
        assert!(report.total_domains >= 2);
        assert!(report.total_facts >= 55);
        // Philosophy with 5 facts should be critical
        let philo = report.profiles.iter().find(|p| p.domain == "philosophy");
        assert!(philo.is_some());
        assert_eq!(philo.unwrap().coverage_tier, CoverageTier::Critical);
    }

    #[test]
    fn test_missing_domains() {
        let a = test_analyzer();
        let report = a.analyze();
        // Many expected domains should be missing from our tiny test DB
        assert!(report.missing_domains.len() > 10);
        assert!(report.missing_domains.contains(&"medicine".to_string()));
    }

    #[test]
    fn test_gap_prompts() {
        let a = test_analyzer();
        let prompts = a.generate_gap_prompts(5);
        assert!(!prompts.is_empty());
        assert!(prompts.len() <= 5);
    }

    #[test]
    fn test_coverage_tier() {
        assert_eq!(CoverageTier::from_count(200_000), CoverageTier::Deep);
        assert_eq!(CoverageTier::from_count(50_000), CoverageTier::Good);
        assert_eq!(CoverageTier::from_count(5_000), CoverageTier::Moderate);
        assert_eq!(CoverageTier::from_count(500), CoverageTier::Sparse);
        assert_eq!(CoverageTier::from_count(50), CoverageTier::Critical);
    }
}
