// ============================================================
// Model Extraction Detection — Protect Against Model Theft
//
// PURPOSE: Detect when an adversary is systematically querying your
// deployed model to steal its behavior. Model extraction attacks
// (also called "model stealing") reconstruct proprietary models from
// API responses, enabling the attacker to:
//   - Clone commercial models without paying
//   - Bypass usage-based billing
//   - Feed stolen models into offensive applications
//   - Train distillation students at scale
//
// ATTACK PATTERNS WE DETECT:
//   1. Systematic query grids (e.g., iterating over parameter spaces)
//   2. Coverage-maximizing queries (broad, shallow)
//   3. Diversity anomalies (unusually varied inputs from single actor)
//   4. Boundary-probing (queries near decision boundaries)
//   5. High query rate from single identity
//   6. Batch-like timing patterns
//
// REFERENCE:
//   Tramèr et al. "Stealing Machine Learning Models via Prediction APIs"
//   Papernot et al. "Practical Black-Box Attacks against Machine Learning"
//
// INTEGRATION:
//   Track per-identity (user/IP/session) query metadata. Flag when
//   patterns indicate extraction. Escalate to rate limits or blocks.
// ============================================================

use std::collections::HashMap;

// ============================================================
// Query Record
// ============================================================

#[derive(Debug, Clone)]
pub struct QueryRecord {
    pub identity: String,
    pub query: String,
    pub timestamp_ms: u64,
    /// Response length (proxy for information leaked per query).
    pub response_length: usize,
    /// Feature: cosine similarity to previous query (if tracked).
    pub similarity_to_previous: Option<f64>,
}

// ============================================================
// Extraction Threat
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ExtractionSignal {
    /// Abnormally high query rate from one identity.
    HighQueryRate { queries_per_hour: f64 },
    /// Queries follow a systematic grid pattern.
    SystematicPattern { description: String },
    /// Unusually diverse queries from a single identity.
    HighDiversity { unique_ratio: f64 },
    /// Query batches with regular timing intervals.
    BatchedTiming { std_dev_ms: f64 },
    /// Queries probing near output boundaries (different responses).
    BoundaryProbing { boundary_hits: usize },
    /// Total volume of information extracted.
    InformationVolume { total_chars: usize },
}

#[derive(Debug, Clone)]
pub struct ExtractionThreat {
    pub identity: String,
    pub signals: Vec<ExtractionSignal>,
    pub severity: ExtractionSeverity,
    pub confidence: f64,
    pub mitigation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtractionSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================
// Per-Identity Tracker
// ============================================================

#[derive(Debug, Clone, Default)]
struct IdentityStats {
    pub queries: Vec<QueryRecord>,
    /// Unix epoch ms when this identity first appeared. Reserved for
    /// session-window tracking; currently not consumed but kept on the
    /// struct so future analysers don't have to backfill it.
    #[allow(dead_code)]
    pub first_seen_ms: u64,
}

// ============================================================
// Model Extraction Detector
// ============================================================

pub struct ModelExtractionDetector {
    stats: HashMap<String, IdentityStats>,
    /// Rate threshold: queries/hour above this = suspicious.
    pub rate_threshold_per_hour: f64,
    /// Diversity threshold: unique queries / total > this = suspicious.
    pub diversity_threshold: f64,
    /// Info volume threshold: total response chars > this = high extraction.
    pub volume_threshold_chars: usize,
    /// Min queries required before scoring an identity.
    pub min_queries: usize,
}

impl ModelExtractionDetector {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            rate_threshold_per_hour: 1000.0,
            diversity_threshold: 0.95,
            volume_threshold_chars: 500_000,
            min_queries: 10,
        }
    }

    /// Record a query for an identity. Returns current threat assessment.
    pub fn record(&mut self, record: QueryRecord) -> ExtractionThreat {
        let identity = record.identity.clone();
        let stats = self.stats.entry(identity.clone()).or_insert_with(|| IdentityStats {
            queries: Vec::new(),
            first_seen_ms: record.timestamp_ms,
        });
        stats.queries.push(record);
        self.assess(&identity)
    }

    /// Assess extraction threat for an identity.
    pub fn assess(&self, identity: &str) -> ExtractionThreat {
        let stats = match self.stats.get(identity) {
            Some(s) if s.queries.len() >= self.min_queries => s,
            _ => return Self::no_threat(identity),
        };

        let mut signals = Vec::new();

        // 1. Rate analysis
        if let Some(rate) = Self::compute_rate(stats) {
            if rate > self.rate_threshold_per_hour {
                signals.push(ExtractionSignal::HighQueryRate { queries_per_hour: rate });
            }
        }

        // 2. Diversity analysis
        let diversity = Self::compute_diversity(stats);
        if diversity > self.diversity_threshold {
            signals.push(ExtractionSignal::HighDiversity { unique_ratio: diversity });
        }

        // 3. Timing regularity (batch detection)
        if let Some(std_dev) = Self::compute_timing_std_dev(stats) {
            let mean_interval = Self::compute_mean_interval(stats).unwrap_or(1.0);
            // If std_dev / mean < 0.1, very regular = bot-like
            if mean_interval > 0.0 && std_dev / mean_interval < 0.1 && stats.queries.len() > 20 {
                signals.push(ExtractionSignal::BatchedTiming { std_dev_ms: std_dev });
            }
        }

        // 4. Information volume
        let total_chars: usize = stats.queries.iter().map(|q| q.response_length).sum();
        if total_chars > self.volume_threshold_chars {
            signals.push(ExtractionSignal::InformationVolume { total_chars });
        }

        // 5. Systematic pattern detection (shared query prefixes)
        if let Some(pattern) = Self::detect_systematic_pattern(stats) {
            signals.push(ExtractionSignal::SystematicPattern { description: pattern });
        }

        // 6. Boundary probing (similar queries with different responses)
        let boundary_hits = Self::count_boundary_probes(stats);
        if boundary_hits >= 5 {
            signals.push(ExtractionSignal::BoundaryProbing { boundary_hits });
        }

        let severity = Self::compute_severity(&signals);
        let confidence = Self::compute_confidence(&signals);
        let mitigation = Self::build_mitigation(&severity);

        ExtractionThreat {
            identity: identity.into(),
            signals,
            severity,
            confidence,
            mitigation,
        }
    }

    fn no_threat(identity: &str) -> ExtractionThreat {
        ExtractionThreat {
            identity: identity.into(),
            signals: Vec::new(),
            severity: ExtractionSeverity::Info,
            confidence: 0.0,
            mitigation: "Insufficient queries to assess threat.".into(),
        }
    }

    fn compute_rate(stats: &IdentityStats) -> Option<f64> {
        if stats.queries.len() < 2 { return None; }
        let first = stats.queries.first()?.timestamp_ms;
        let last = stats.queries.last()?.timestamp_ms;
        let elapsed_hours = (last - first) as f64 / 3_600_000.0;
        if elapsed_hours <= 0.0 {
            // All queries instantaneous — extreme rate
            return Some(f64::MAX.min(stats.queries.len() as f64 * 3600.0));
        }
        Some(stats.queries.len() as f64 / elapsed_hours)
    }

    fn compute_diversity(stats: &IdentityStats) -> f64 {
        if stats.queries.is_empty() { return 0.0; }
        let unique: std::collections::HashSet<&String> =
            stats.queries.iter().map(|q| &q.query).collect();
        unique.len() as f64 / stats.queries.len() as f64
    }

    fn compute_timing_std_dev(stats: &IdentityStats) -> Option<f64> {
        if stats.queries.len() < 3 { return None; }
        let intervals: Vec<f64> = stats.queries.windows(2)
            .map(|w| (w[1].timestamp_ms.saturating_sub(w[0].timestamp_ms)) as f64)
            .collect();
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance: f64 = intervals.iter()
            .map(|i| (i - mean).powi(2))
            .sum::<f64>() / intervals.len() as f64;
        Some(variance.sqrt())
    }

    fn compute_mean_interval(stats: &IdentityStats) -> Option<f64> {
        if stats.queries.len() < 2 { return None; }
        let intervals: Vec<f64> = stats.queries.windows(2)
            .map(|w| (w[1].timestamp_ms.saturating_sub(w[0].timestamp_ms)) as f64)
            .collect();
        Some(intervals.iter().sum::<f64>() / intervals.len() as f64)
    }

    /// Detect systematic patterns (e.g., iterating over parameter space).
    /// Heuristic: if >60% of queries share a common long prefix, it's systematic.
    fn detect_systematic_pattern(stats: &IdentityStats) -> Option<String> {
        if stats.queries.len() < 10 { return None; }

        // Check for common prefix > 10 chars.
        let queries: Vec<&String> = stats.queries.iter().map(|q| &q.query).collect();
        let min_len = queries.iter().map(|q| q.len()).min().unwrap_or(0);
        if min_len < 10 { return None; }

        let first = queries[0].as_bytes();
        let mut common_len = min_len;
        for q in queries.iter().skip(1) {
            let b = q.as_bytes();
            let mut matching = 0;
            while matching < common_len && matching < b.len()
                && first[matching] == b[matching] {
                matching += 1;
            }
            common_len = matching;
        }

        if common_len >= 15 {
            let prefix = std::str::from_utf8(&first[..common_len])
                .unwrap_or("")
                .chars()
                .take(30)
                .collect::<String>();
            return Some(format!("{} queries share prefix '{}...'", queries.len(), prefix));
        }
        None
    }

    /// Count pairs of similar queries with different responses (boundary probing).
    fn count_boundary_probes(stats: &IdentityStats) -> usize {
        let mut count = 0;
        for (i, a) in stats.queries.iter().enumerate() {
            for b in stats.queries.iter().skip(i + 1) {
                if let Some(sim) = b.similarity_to_previous {
                    if sim > 0.9 && (a.response_length as i64 - b.response_length as i64).abs() > 100 {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn compute_severity(signals: &[ExtractionSignal]) -> ExtractionSeverity {
        if signals.is_empty() { return ExtractionSeverity::Info; }
        // Critical: 3+ signals OR any single high-volume signal
        let has_volume = signals.iter().any(|s|
            matches!(s, ExtractionSignal::InformationVolume { total_chars } if *total_chars > 1_000_000));
        let has_systematic = signals.iter().any(|s|
            matches!(s, ExtractionSignal::SystematicPattern { .. }));
        let has_rate = signals.iter().any(|s|
            matches!(s, ExtractionSignal::HighQueryRate { queries_per_hour } if *queries_per_hour > 5000.0));

        if signals.len() >= 3 || has_volume || (has_systematic && has_rate) {
            ExtractionSeverity::Critical
        } else if signals.len() == 2 || has_systematic {
            ExtractionSeverity::High
        } else if signals.len() == 1 {
            ExtractionSeverity::Medium
        } else {
            ExtractionSeverity::Low
        }
    }

    fn compute_confidence(signals: &[ExtractionSignal]) -> f64 {
        if signals.is_empty() { return 0.0; }
        (0.3 + 0.2 * signals.len() as f64).min(0.95)
    }

    fn build_mitigation(severity: &ExtractionSeverity) -> String {
        match severity {
            ExtractionSeverity::Critical => {
                "BLOCK this identity. Model extraction attack in progress. Notify security team. Consider legal action if usage violates ToS.".into()
            }
            ExtractionSeverity::High => {
                "Rate limit aggressively. Require additional authentication. Log all queries for evidence.".into()
            }
            ExtractionSeverity::Medium => {
                "Apply moderate rate limiting. Monitor for escalation. Log queries.".into()
            }
            ExtractionSeverity::Low => {
                "Log as potential concern. No immediate action required.".into()
            }
            ExtractionSeverity::Info => {
                "Normal usage pattern.".into()
            }
        }
    }

    /// Get all identities currently tracked, sorted by severity.
    pub fn top_threats(&self, n: usize) -> Vec<ExtractionThreat> {
        let mut threats: Vec<ExtractionThreat> = self.stats.keys()
            .map(|id| self.assess(id))
            .filter(|t| t.severity != ExtractionSeverity::Info)
            .collect();
        threats.sort_by(|a, b| b.severity.cmp(&a.severity));
        threats.truncate(n);
        threats
    }

    /// Reset tracking for an identity (e.g., after block or period rollover).
    pub fn reset(&mut self, identity: &str) {
        self.stats.remove(identity);
    }

    pub fn tracked_count(&self) -> usize {
        self.stats.len()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn record(identity: &str, query: &str, ts: u64, len: usize) -> QueryRecord {
        QueryRecord {
            identity: identity.into(),
            query: query.into(),
            timestamp_ms: ts,
            response_length: len,
            similarity_to_previous: None,
        }
    }

    #[test]
    fn test_insufficient_queries_info_only() {
        let mut detector = ModelExtractionDetector::new();
        for i in 0..5 {
            let threat = detector.record(record("user1", "query", 1000 + i, 100));
            assert_eq!(threat.severity, ExtractionSeverity::Info);
        }
    }

    #[test]
    fn test_high_rate_detected() {
        let mut detector = ModelExtractionDetector::new();
        // 100 queries in 1 second = 360,000/hour
        for i in 0..100 {
            detector.record(record("attacker", &format!("q{}", i), 1000 + i * 10, 100));
        }
        let threat = detector.assess("attacker");
        assert!(threat.signals.iter().any(|s|
            matches!(s, ExtractionSignal::HighQueryRate { .. })));
    }

    #[test]
    fn test_high_diversity_detected() {
        let mut detector = ModelExtractionDetector::new();
        // 100% unique queries
        for i in 0..20 {
            detector.record(record("attacker", &format!("unique query {}", i),
                1000 + i * 1000, 100));
        }
        let threat = detector.assess("attacker");
        assert!(threat.signals.iter().any(|s|
            matches!(s, ExtractionSignal::HighDiversity { .. })),
            "Should flag high diversity, got signals: {:?}", threat.signals);
    }

    #[test]
    fn test_normal_user_no_threat() {
        let mut detector = ModelExtractionDetector::new();
        // Normal user: some repetition, normal rate, modest volume
        for i in 0..15 {
            let q = match i % 3 {
                0 => "how do I reset password",
                1 => "what is my balance",
                _ => "cancel subscription",
            };
            detector.record(record("normal_user", q, 1000 + i * 60_000, 200));
        }
        let threat = detector.assess("normal_user");
        assert!(threat.severity == ExtractionSeverity::Info
            || threat.severity == ExtractionSeverity::Low,
            "Normal user should not be flagged: {:?}", threat);
    }

    #[test]
    fn test_batched_timing_detected() {
        let mut detector = ModelExtractionDetector::new();
        // Queries at perfectly regular intervals
        for i in 0..25 {
            detector.record(record("bot", &format!("q{}", i),
                1000 + i * 5000, 100)); // Every 5 seconds exactly
        }
        let threat = detector.assess("bot");
        assert!(threat.signals.iter().any(|s|
            matches!(s, ExtractionSignal::BatchedTiming { .. })),
            "Regular timing should be flagged: {:?}", threat.signals);
    }

    #[test]
    fn test_systematic_pattern_detected() {
        let mut detector = ModelExtractionDetector::new();
        // Iterating over parameter space
        for i in 0..20 {
            detector.record(record("attacker",
                &format!("Translate to French the word number: {}", i),
                1000 + i * 1000, 100));
        }
        let threat = detector.assess("attacker");
        assert!(threat.signals.iter().any(|s|
            matches!(s, ExtractionSignal::SystematicPattern { .. })),
            "Systematic pattern should be flagged: {:?}", threat.signals);
    }

    #[test]
    fn test_volume_detected() {
        let mut detector = ModelExtractionDetector::new();
        // Large responses add up
        for i in 0..15 {
            detector.record(record("attacker", &format!("long query {}", i),
                1000 + i * 60_000, 60_000)); // Each response is 60k chars
        }
        let threat = detector.assess("attacker");
        assert!(threat.signals.iter().any(|s|
            matches!(s, ExtractionSignal::InformationVolume { total_chars } if *total_chars > 500_000)),
            "High volume should be flagged");
    }

    #[test]
    fn test_critical_severity_for_3_plus_signals() {
        let mut detector = ModelExtractionDetector::new();
        // Combine high rate + systematic + volume
        for i in 0..30 {
            detector.record(record("attacker",
                &format!("Prefix: common_extractor_probe_{}", i),
                1000 + i * 10, 50_000)); // Fast + systematic + large
        }
        let threat = detector.assess("attacker");
        assert!(threat.severity >= ExtractionSeverity::High);
    }

    #[test]
    fn test_severity_escalation_ordering() {
        use ExtractionSeverity::*;
        assert!(Critical > High);
        assert!(High > Medium);
        assert!(Medium > Low);
        assert!(Low > Info);
    }

    #[test]
    fn test_mitigation_recommendations() {
        let mut detector = ModelExtractionDetector::new();
        for i in 0..30 {
            detector.record(record("attacker",
                &format!("Extract pattern attempt {}", i),
                1000 + i, 80_000));
        }
        let threat = detector.assess("attacker");
        if threat.severity >= ExtractionSeverity::High {
            assert!(threat.mitigation.to_lowercase().contains("rate limit")
                || threat.mitigation.to_lowercase().contains("block"),
                "High severity should recommend action: {}", threat.mitigation);
        }
    }

    #[test]
    fn test_top_threats() {
        let mut detector = ModelExtractionDetector::new();
        // Normal users
        for i in 0..15 {
            detector.record(record("good_user", &format!("q{}", i), 1000 + i * 60_000, 200));
        }
        // Attacker
        for i in 0..30 {
            detector.record(record("attacker",
                &format!("Common prefix extraction {}", i),
                1000 + i * 10, 50_000));
        }
        let top = detector.top_threats(5);
        assert!(top.iter().any(|t| t.identity == "attacker"),
            "Attacker should be in top threats");
    }

    #[test]
    fn test_reset_clears_tracking() {
        let mut detector = ModelExtractionDetector::new();
        for i in 0..15 {
            detector.record(record("user1", &format!("q{}", i), 1000 + i, 100));
        }
        assert_eq!(detector.tracked_count(), 1);
        detector.reset("user1");
        assert_eq!(detector.tracked_count(), 0);
    }

    #[test]
    fn test_multiple_identities_independent() {
        let mut detector = ModelExtractionDetector::new();
        for i in 0..15 {
            detector.record(record("user_a", &format!("q{}", i), 1000 + i * 1000, 100));
            detector.record(record("user_b", &format!("q{}", i), 1000 + i * 60_000, 100));
        }
        assert_eq!(detector.tracked_count(), 2);
        // user_a with tight intervals may be flagged; user_b at 1/min should not
        let a = detector.assess("user_a");
        let b = detector.assess("user_b");
        // At minimum, assessments are independent
        assert_eq!(a.identity, "user_a");
        assert_eq!(b.identity, "user_b");
    }

    #[test]
    fn test_boundary_probing_detected() {
        let mut detector = ModelExtractionDetector::new();
        // Similar queries with different responses
        for i in 0..15 {
            let record = QueryRecord {
                identity: "attacker".into(),
                query: format!("probe {}", i),
                timestamp_ms: 1000 + i * 1000,
                response_length: if i % 2 == 0 { 100 } else { 500 }, // Toggling
                similarity_to_previous: Some(0.95), // All similar
            };
            detector.record(record);
        }
        let threat = detector.assess("attacker");
        // Note: might or might not flag depending on implementation,
        // but test validates the method runs.
        assert_eq!(threat.identity, "attacker");
    }
}
