// ============================================================
// Continuous Intelligence Gatherer — Always-On Information Intake
//
// PURPOSE: Keep LFI constantly up-to-date with the latest info in
// science, tech, AI, security. Every piece of information goes through
// the epistemic filter before being accepted.
//
// SOURCE TYPES:
//   - Security: CVE feeds (NVD, MITRE), security advisories
//   - Research: arXiv new papers, conference proceedings
//   - News: tech journalism (via RSS feeds)
//   - Community: GitHub trending, Hacker News, Lobsters
//   - Standards: IETF drafts, NIST publications, W3C updates
//
// PIPELINE:
//   1. Source poller fetches new items
//   2. Content summarizer extracts key claims (via LLM if available)
//   3. Each claim passes through EpistemicFilter
//   4. Filtered claims feed into KnowledgeEngine
//   5. Cross-domain engine identifies related existing knowledge
//   6. Generalization tester checks for rote risk
//
// SAFETY:
//   - All sources are categorized (PeerReviewed, Journalism, Community, Anonymous)
//   - No source gets 100% trust — even reputable ones have bounded confidence
//   - Contradictions with existing high-confidence claims flag for review
//   - Rate-limited fetching to avoid abusing source APIs
// ============================================================

use crate::intelligence::epistemic_filter::{EpistemicFilter, Source, SourceCategory, KnowledgeTier};
use crate::cognition::knowledge::KnowledgeEngine;
use std::collections::HashMap;

// ============================================================
// Source Poller Configuration
// ============================================================

/// A configured intelligence source.
#[derive(Debug, Clone)]
pub struct IntelSource {
    pub name: String,
    pub url: String,
    pub category: SourceCategory,
    /// How often to poll (seconds).
    pub poll_interval_sec: u64,
    /// Last successful poll time (ms).
    pub last_poll_ms: u64,
    /// Number of successful polls.
    pub poll_count: u64,
    /// Number of claims ingested from this source.
    pub claims_ingested: u64,
}

impl IntelSource {
    pub fn new(name: &str, url: &str, category: SourceCategory, poll_interval_sec: u64) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            category,
            poll_interval_sec,
            last_poll_ms: 0,
            poll_count: 0,
            claims_ingested: 0,
        }
    }

    /// Should this source be polled now?
    pub fn is_due(&self, now_ms: u64) -> bool {
        let interval_ms = self.poll_interval_sec * 1000;
        now_ms.saturating_sub(self.last_poll_ms) >= interval_ms
    }
}

// ============================================================
// Built-in Intelligence Sources
// ============================================================

/// Curated list of reputable intelligence sources.
/// Users can add/remove sources via register_source().
pub fn default_sources() -> Vec<IntelSource> {
    vec![
        // Security feeds (hourly)
        IntelSource::new(
            "NIST NVD",
            "https://services.nvd.nist.gov/rest/json/cves/2.0",
            SourceCategory::Standards,
            3600, // 1 hour
        ),
        IntelSource::new(
            "MITRE CVE",
            "https://cve.mitre.org/data/downloads/",
            SourceCategory::Standards,
            3600,
        ),
        IntelSource::new(
            "US-CERT Alerts",
            "https://www.cisa.gov/uscert/ncas/alerts",
            SourceCategory::Standards,
            1800, // 30 min
        ),

        // Research feeds (daily)
        IntelSource::new(
            "arXiv CS.CR",
            "http://export.arxiv.org/rss/cs.CR",
            SourceCategory::PeerReviewed,
            86400, // 24 hours
        ),
        IntelSource::new(
            "arXiv CS.AI",
            "http://export.arxiv.org/rss/cs.AI",
            SourceCategory::PeerReviewed,
            86400,
        ),
        IntelSource::new(
            "arXiv CS.LG",
            "http://export.arxiv.org/rss/cs.LG",
            SourceCategory::PeerReviewed,
            86400,
        ),

        // Community (every 4 hours)
        IntelSource::new(
            "Hacker News",
            "https://hnrss.org/frontpage",
            SourceCategory::Community,
            14400,
        ),
        IntelSource::new(
            "Lobsters",
            "https://lobste.rs/rss",
            SourceCategory::Community,
            14400,
        ),

        // Standards bodies (daily)
        IntelSource::new(
            "IETF Drafts",
            "https://datatracker.ietf.org/doc/recent/",
            SourceCategory::Standards,
            86400,
        ),
    ]
}

// ============================================================
// Continuous Intelligence Engine
// ============================================================

/// The continuous intelligence gatherer.
/// BUG ASSUMPTION: actual HTTP fetching is stubbed — in production,
/// would use reqwest or ureq. For now, this is the architecture scaffold.
pub struct ContinuousIntelligence {
    pub sources: Vec<IntelSource>,
    pub filter: EpistemicFilter,
    /// Total poll attempts.
    pub poll_count: u64,
    /// Total claims ingested.
    pub claims_ingested: u64,
    /// Per-source success rate.
    pub source_health: HashMap<String, (u64, u64)>, // (successes, attempts)
}

impl ContinuousIntelligence {
    pub fn new() -> Self {
        debuglog!("ContinuousIntelligence::new: Initializing always-on intel gatherer");
        let mut engine = Self {
            sources: default_sources(),
            filter: EpistemicFilter::new(),
            poll_count: 0,
            claims_ingested: 0,
            source_health: HashMap::new(),
        };

        // Register all default sources with the filter.
        for src in engine.sources.clone() {
            engine.filter.register_source(Source {
                name: src.name.clone(),
                category: src.category.clone(),
                trust: src.category.base_trust(),
                track_record: 0.5,
                claim_count: 0,
            });
        }

        engine
    }

    /// Add a custom intelligence source.
    pub fn register_source(&mut self, source: IntelSource) {
        debuglog!("ContinuousIntelligence: Registering source '{}'", source.name);
        self.filter.register_source(Source {
            name: source.name.clone(),
            category: source.category.clone(),
            trust: source.category.base_trust(),
            track_record: 0.5,
            claim_count: 0,
        });
        self.sources.push(source);
    }

    /// Run one polling cycle. Returns number of new claims ingested.
    /// BUG ASSUMPTION: actual HTTP is stubbed here. In production, would
    /// use an async HTTP client and proper feed parsing.
    pub fn poll_cycle(&mut self, knowledge: &mut KnowledgeEngine) -> u64 {
        let now = Self::now_ms();
        let mut new_claims = 0u64;

        let sources_to_poll: Vec<usize> = self.sources.iter()
            .enumerate()
            .filter(|(_, s)| s.is_due(now))
            .map(|(i, _)| i)
            .collect();

        debuglog!("ContinuousIntelligence::poll_cycle: {} sources due", sources_to_poll.len());

        for idx in sources_to_poll {
            let source_name = self.sources[idx].name.clone();

            // Mark poll attempt.
            self.source_health.entry(source_name.clone())
                .or_insert((0, 0)).1 += 1;

            // Simulated fetch — in production, this would make HTTP calls.
            let source_snapshot = self.sources[idx].clone();
            match Self::simulated_fetch_static(&source_snapshot) {
                Ok(claims) => {
                    if let Some(e) = self.source_health.get_mut(&source_name) {
                        e.0 += 1;
                    }
                    self.sources[idx].last_poll_ms = now;
                    self.sources[idx].poll_count += 1;

                    for claim in claims {
                        let result = self.filter.ingest_claim(&claim, &source_name);
                        if !result.rejected {
                            // Add to knowledge engine if tier is at least Plausible.
                            if matches!(result.tier,
                                KnowledgeTier::Proof | KnowledgeTier::Consensus
                                | KnowledgeTier::Corroborated | KnowledgeTier::Plausible) {
                                let concept = format!("intel_{}", self.claims_ingested);
                                let _ = knowledge.learn_with_definition(
                                    &concept,
                                    &claim,
                                    &[&format!("source_{}", source_name)],
                                    result.confidence * 0.5, // Start modest
                                    true,
                                );
                                new_claims += 1;
                                self.sources[idx].claims_ingested += 1;
                                self.claims_ingested += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    debuglog!("ContinuousIntelligence: Poll failed for '{}': {}",
                        source_name, e);
                }
            }

            self.poll_count += 1;
        }

        new_claims
    }

    /// Stubbed fetch — in production, replace with real HTTP + feed parsing.
    /// Static to avoid borrow-checker conflicts with self.source_health.
    fn simulated_fetch_static(source: &IntelSource) -> Result<Vec<String>, String> {
        debuglog!("ContinuousIntelligence::simulated_fetch: '{}' (stub)", source.name);
        Ok(Vec::new())
    }

    /// Manually inject a claim (for testing or manual intelligence).
    pub fn inject_claim(
        &mut self,
        claim: &str,
        source_name: &str,
        knowledge: &mut KnowledgeEngine,
    ) -> bool {
        let result = self.filter.ingest_claim(claim, source_name);
        if !result.rejected && result.confidence > 0.3 {
            let concept = format!("intel_{}", self.claims_ingested);
            let _ = knowledge.learn_with_definition(
                &concept,
                claim,
                &[&format!("source_{}", source_name)],
                result.confidence * 0.5,
                true,
            );
            self.claims_ingested += 1;
            true
        } else {
            false
        }
    }

    /// Per-source reliability score.
    pub fn source_reliability(&self, source_name: &str) -> f64 {
        match self.source_health.get(source_name) {
            Some((s, a)) if *a > 0 => *s as f64 / *a as f64,
            _ => 0.5, // Unknown reliability
        }
    }

    /// Generate a report of intelligence gathering activity.
    pub fn report(&self) -> String {
        let mut out = "=== Continuous Intelligence Report ===\n".to_string();
        out.push_str(&format!("Sources:          {}\n", self.sources.len()));
        out.push_str(&format!("Total polls:      {}\n", self.poll_count));
        out.push_str(&format!("Claims ingested:  {}\n", self.claims_ingested));

        out.push_str("\nPer-source activity:\n");
        let mut sources_sorted: Vec<_> = self.sources.iter().collect();
        sources_sorted.sort_by(|a, b| b.claims_ingested.cmp(&a.claims_ingested));
        for src in sources_sorted.iter().take(10) {
            let reliability = self.source_reliability(&src.name);
            out.push_str(&format!(
                "  {:20} polls={:4} claims={:4} reliability={:.1}%\n",
                crate::truncate_str(&src.name, 20),
                src.poll_count,
                src.claims_ingested,
                reliability * 100.0,
            ));
        }

        out.push_str(&format!("\n{}", self.filter.report()));

        out
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_sources_exist() {
        let sources = default_sources();
        assert!(sources.len() >= 8, "Should have 8+ default sources");

        // Check category coverage
        let categories: Vec<SourceCategory> = sources.iter().map(|s| s.category.clone()).collect();
        assert!(categories.iter().any(|c| matches!(c, SourceCategory::Standards)));
        assert!(categories.iter().any(|c| matches!(c, SourceCategory::PeerReviewed)));
        assert!(categories.iter().any(|c| matches!(c, SourceCategory::Community)));
    }

    #[test]
    fn test_source_is_due() {
        let mut source = IntelSource::new("test", "http://example.com", SourceCategory::Community, 60);
        source.last_poll_ms = 1_000_000;

        // Not due yet
        assert!(!source.is_due(1_000_500));

        // Due after interval (60s = 60_000ms)
        assert!(source.is_due(1_061_000));
    }

    #[test]
    fn test_engine_creation() {
        let engine = ContinuousIntelligence::new();
        assert!(engine.sources.len() >= 8);
        assert_eq!(engine.poll_count, 0);
        assert_eq!(engine.claims_ingested, 0);
    }

    #[test]
    fn test_inject_claim_reputable() {
        let mut engine = ContinuousIntelligence::new();
        let mut knowledge = KnowledgeEngine::new();

        let initial = knowledge.concept_count();

        // Standards body injects a CVE claim.
        let accepted = engine.inject_claim(
            "CVE-2024-12345: Buffer overflow in libfoo",
            "NIST NVD",
            &mut knowledge,
        );

        assert!(accepted, "Reputable source claim should be accepted");
        assert!(knowledge.concept_count() > initial);
    }

    #[test]
    fn test_inject_claim_adversarial_rejected() {
        let mut engine = ContinuousIntelligence::new();
        let mut knowledge = KnowledgeEngine::new();

        // Register an adversarial source.
        engine.filter.register_source(Source {
            name: "fake_news".into(),
            category: SourceCategory::Adversarial,
            trust: 0.05,
            track_record: 0.1,
            claim_count: 0,
        });

        let accepted = engine.inject_claim(
            "Earth is flat",
            "fake_news",
            &mut knowledge,
        );

        assert!(!accepted, "Adversarial source claim should be rejected");
    }

    #[test]
    fn test_register_custom_source() {
        let mut engine = ContinuousIntelligence::new();
        let initial_count = engine.sources.len();

        engine.register_source(IntelSource::new(
            "Custom Expert Blog",
            "https://example.com/feed",
            SourceCategory::Expert,
            3600,
        ));

        assert_eq!(engine.sources.len(), initial_count + 1);
        // Should also be registered in the filter.
        assert!(engine.filter.check("test claim").is_none()); // No claims yet
    }

    #[test]
    fn test_source_reliability_tracking() {
        let mut engine = ContinuousIntelligence::new();
        engine.source_health.insert("test_src".into(), (8, 10));
        assert!((engine.source_reliability("test_src") - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_report_generation() {
        let mut engine = ContinuousIntelligence::new();
        let mut knowledge = KnowledgeEngine::new();
        let _ = engine.inject_claim("TLS 1.3 is current standard", "NIST NVD", &mut knowledge);

        let report = engine.report();
        assert!(report.contains("Continuous Intelligence Report"));
        assert!(report.contains("Sources:"));
        assert!(report.contains("Per-source activity"));
    }

    #[test]
    fn test_poll_interval_categories() {
        let sources = default_sources();
        // All sources poll at least daily (within 24 hours).
        for src in &sources {
            assert!(src.poll_interval_sec <= 86400,
                "Source '{}' should poll at least daily (got {}s)",
                src.name, src.poll_interval_sec);
        }
    }
}
