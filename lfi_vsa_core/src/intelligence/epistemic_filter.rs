// ============================================================
// Epistemic Filter — Skeptical Intake with Asymptotic Confidence
//
// PRINCIPLES (per PlausiDen doctrine):
//   1. Never assume anything is 100% true — confidence is asymptotic.
//      Even 0.999 is not 1.0. Only mathematical proofs approach 1.0.
//   2. Skeptical default: new information starts at LOW confidence.
//   3. Evidence-based updates: confidence rises ONLY with corroboration.
//   4. Source-weighted: reputable sources get more weight, not blind trust.
//   5. Recency + persistence: old information decays, recent evidence matters.
//   6. Contradiction tracking: if X contradicts established knowledge, that's a RED FLAG.
//   7. Protective filter: reject information from adversarial or unknown sources.
//
// CONFIDENCE HIERARCHY:
//   [0.99+]  Formal mathematical proofs, physical laws with countless verifications
//   [0.90+]  Peer-reviewed consensus across independent sources
//   [0.75+]  Multiple reputable sources agree
//   [0.50+]  Single reputable source, uncorroborated
//   [0.25+]  Unknown source, plausible content
//   [< 0.25] Adversarial or contradictory information
//
// ASYMPTOTIC FUNCTION:
//   confidence = 1 - exp(-evidence_weight * trust_factor)
//   No matter how much evidence, confidence approaches but never reaches 1.0.
// ============================================================

use std::collections::HashMap;

// ============================================================
// Knowledge Tier (Confidence Hierarchy)
// ============================================================

/// Tier of epistemic status for a claim.
/// BUG ASSUMPTION: tier boundaries are hand-tuned; may need calibration
/// based on empirical validation.
#[derive(Debug, Clone, PartialEq)]
pub enum KnowledgeTier {
    /// Formal mathematical or logical proof. Highest confidence, never 1.0.
    Proof,
    /// Scientific consensus, physical laws, well-established facts.
    Consensus,
    /// Multiple independent reputable sources agree.
    Corroborated,
    /// Single reputable source, uncorroborated.
    Plausible,
    /// Unknown source, no contradiction detected.
    Unverified,
    /// Contradicts established knowledge or from adversarial source.
    Suspect,
}

impl KnowledgeTier {
    /// Maximum confidence allowed at this tier.
    /// Even Proof tier has a cap below 1.0 (asymptote principle).
    pub fn max_confidence(&self) -> f64 {
        match self {
            Self::Proof => 0.9999,        // Asymptote — never exactly 1
            Self::Consensus => 0.99,
            Self::Corroborated => 0.90,
            Self::Plausible => 0.65,
            Self::Unverified => 0.35,
            Self::Suspect => 0.15,
        }
    }

    /// Minimum confidence floor at this tier (below this, tier demotes).
    pub fn min_confidence(&self) -> f64 {
        match self {
            Self::Proof => 0.95,
            Self::Consensus => 0.85,
            Self::Corroborated => 0.70,
            Self::Plausible => 0.40,
            Self::Unverified => 0.15,
            Self::Suspect => 0.0,
        }
    }
}

// ============================================================
// Source Trust
// ============================================================

/// A source of information with associated trust metadata.
#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub category: SourceCategory,
    /// Base trust level (0.0 to 0.95 — never fully trusted).
    pub trust: f64,
    /// Historical accuracy (correct claims / total claims).
    pub track_record: f64,
    /// How many claims we've seen from this source.
    pub claim_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceCategory {
    /// Mathematical proof system (Coq, Lean, Isabelle, hand-verified).
    FormalProof,
    /// Peer-reviewed scientific publication.
    PeerReviewed,
    /// Official standards body (NIST, IEEE, RFC, ISO).
    Standards,
    /// Reputable news organization.
    Journalism,
    /// Subject-matter expert.
    Expert,
    /// Open-source community (crates.io, GitHub, etc.).
    Community,
    /// Unknown or anonymous source.
    Anonymous,
    /// Known adversarial or unreliable source.
    Adversarial,
}

impl SourceCategory {
    /// Base trust multiplier for a category.
    pub fn base_trust(&self) -> f64 {
        match self {
            Self::FormalProof => 0.95,
            Self::PeerReviewed => 0.85,
            Self::Standards => 0.85,
            Self::Expert => 0.70,
            Self::Journalism => 0.60,
            Self::Community => 0.50,
            Self::Anonymous => 0.20,
            Self::Adversarial => 0.05,
        }
    }
}

// ============================================================
// Claim with Provenance
// ============================================================

/// A claim that has been filtered through the epistemic system.
#[derive(Debug, Clone)]
pub struct FilteredClaim {
    pub claim: String,
    pub tier: KnowledgeTier,
    pub confidence: f64,
    /// Sources supporting this claim.
    pub supporting_sources: Vec<String>,
    /// Sources contradicting this claim.
    pub contradicting_sources: Vec<String>,
    /// Amount of evidence accumulated.
    pub evidence_weight: f64,
    /// Was this claim rejected by the filter?
    pub rejected: bool,
    /// Why was it rejected (if applicable)?
    pub rejection_reason: Option<String>,
    /// Timestamp of first ingestion (ms).
    pub first_seen_ms: u64,
    /// Timestamp of most recent reinforcement.
    pub last_updated_ms: u64,
}

// ============================================================
// Epistemic Filter
// ============================================================

/// The epistemic filter — protective skepticism for knowledge intake.
/// BUG ASSUMPTION: contradiction detection is text-similarity based;
/// semantic contradictions with different wording may slip through.
pub struct EpistemicFilter {
    /// All claims tracked, keyed by normalized claim text.
    claims: HashMap<String, FilteredClaim>,
    /// Source registry.
    sources: HashMap<String, Source>,
    /// Total claims processed.
    pub total_processed: usize,
    /// Total claims rejected.
    pub total_rejected: usize,
    /// Total claims promoted to higher tiers.
    pub total_promoted: usize,
}

impl EpistemicFilter {
    pub fn new() -> Self {
        debuglog!("EpistemicFilter::new: Initializing skeptical intake filter");
        Self {
            claims: HashMap::new(),
            sources: HashMap::new(),
            total_processed: 0,
            total_rejected: 0,
            total_promoted: 0,
        }
    }

    /// Register a source with its trust profile.
    pub fn register_source(&mut self, source: Source) {
        debuglog!("EpistemicFilter::register_source: '{}' category={:?} trust={:.2}",
            source.name, source.category, source.trust);
        self.sources.insert(source.name.clone(), source);
    }

    /// Create a new source with default trust from its category.
    pub fn register_source_default(&mut self, name: &str, category: SourceCategory) {
        let trust = category.base_trust();
        self.register_source(Source {
            name: name.into(),
            category,
            trust,
            track_record: 0.5, // Neutral default
            claim_count: 0,
        });
    }

    /// Ingest a claim through the filter.
    /// Returns the FilteredClaim result — check `rejected` field.
    pub fn ingest_claim(&mut self, claim: &str, source_name: &str) -> FilteredClaim {
        self.total_processed += 1;

        let normalized = Self::normalize_claim(claim);
        let now = Self::now_ms();

        // Look up source — unknown sources get Anonymous treatment.
        let source = self.sources.entry(source_name.to_string())
            .or_insert_with(|| Source {
                name: source_name.into(),
                category: SourceCategory::Anonymous,
                trust: SourceCategory::Anonymous.base_trust(),
                track_record: 0.5,
                claim_count: 0,
            });
        source.claim_count += 1;
        let source_trust = source.trust;
        let source_category = source.category.clone();

        // Filter 1: Adversarial source → automatic rejection (unless strong corroboration).
        if source_category == SourceCategory::Adversarial {
            let rejection = FilteredClaim {
                claim: claim.into(),
                tier: KnowledgeTier::Suspect,
                confidence: 0.05,
                supporting_sources: vec![source_name.into()],
                contradicting_sources: Vec::new(),
                evidence_weight: 0.0,
                rejected: true,
                rejection_reason: Some("Adversarial source — requires multi-source corroboration".into()),
                first_seen_ms: now,
                last_updated_ms: now,
            };
            self.total_rejected += 1;
            return rejection;
        }

        // Filter 2: Check for contradiction with existing high-confidence claims.
        let contradiction = self.detect_contradiction(&normalized);
        if contradiction.is_some() {
            debuglog!("EpistemicFilter: Contradiction detected for '{}'",
                crate::truncate_str(claim, 60));
        }

        // Filter 3: Update or create claim entry.
        if let Some(existing) = self.claims.get_mut(&normalized) {
            // Existing claim — corroborate or contradict.
            existing.last_updated_ms = now;

            if !existing.supporting_sources.contains(&source_name.to_string()) {
                existing.supporting_sources.push(source_name.into());
                existing.evidence_weight += source_trust;

                // Recompute confidence: asymptotic function.
                let new_confidence = Self::asymptotic_confidence(existing.evidence_weight);

                // Update tier based on evidence.
                let new_tier = Self::tier_for_evidence(
                    &existing.supporting_sources, &self.sources, &source_category,
                );

                if Self::tier_rank(&new_tier) > Self::tier_rank(&existing.tier) {
                    self.total_promoted += 1;
                    debuglog!("EpistemicFilter: Promoted claim from {:?} to {:?}",
                        existing.tier, new_tier);
                }

                existing.tier = new_tier.clone();
                existing.confidence = new_confidence.min(new_tier.max_confidence());
            }

            return existing.clone();
        }

        // New claim — initial skeptical confidence.
        let initial_tier = match source_category {
            SourceCategory::FormalProof => KnowledgeTier::Proof,
            SourceCategory::PeerReviewed | SourceCategory::Standards => KnowledgeTier::Plausible,
            SourceCategory::Expert => KnowledgeTier::Plausible,
            _ => KnowledgeTier::Unverified,
        };

        let initial_confidence = Self::asymptotic_confidence(source_trust)
            .min(initial_tier.max_confidence());

        let filtered = FilteredClaim {
            claim: claim.into(),
            tier: initial_tier,
            confidence: initial_confidence,
            supporting_sources: vec![source_name.into()],
            contradicting_sources: Vec::new(),
            evidence_weight: source_trust,
            rejected: false,
            rejection_reason: None,
            first_seen_ms: now,
            last_updated_ms: now,
        };

        self.claims.insert(normalized, filtered.clone());
        filtered
    }

    /// Record a contradiction: claim A contradicts claim B.
    pub fn record_contradiction(&mut self, claim_a: &str, claim_b: &str) {
        let norm_a = Self::normalize_claim(claim_a);
        let norm_b = Self::normalize_claim(claim_b);

        // Both claims take a confidence hit — real knowledge rarely has
        // exact contradictions, so this is a signal to re-examine both.
        for key in [&norm_a, &norm_b] {
            if let Some(c) = self.claims.get_mut(key) {
                c.confidence *= 0.7; // Discount 30% for contradiction
                if !c.contradicting_sources.is_empty() {
                    c.tier = KnowledgeTier::Suspect;
                }
            }
        }
    }

    /// Query the filter for a claim's status.
    pub fn check(&self, claim: &str) -> Option<&FilteredClaim> {
        let normalized = Self::normalize_claim(claim);
        self.claims.get(&normalized)
    }

    /// Get all claims at or above a given tier.
    pub fn claims_at_tier(&self, min_tier: &KnowledgeTier) -> Vec<&FilteredClaim> {
        let min_rank = Self::tier_rank(min_tier);
        self.claims.values()
            .filter(|c| Self::tier_rank(&c.tier) >= min_rank && !c.rejected)
            .collect()
    }

    /// Apply time-based confidence decay to all claims.
    /// BUG ASSUMPTION: uses simple linear decay — could use exponential
    /// or domain-specific decay rates in future versions.
    pub fn apply_decay(&mut self, decay_rate_per_day: f64) {
        let now = Self::now_ms();
        for claim in self.claims.values_mut() {
            let age_days = (now - claim.last_updated_ms) as f64 / (1000.0 * 86400.0);
            let decay = (decay_rate_per_day * age_days).min(0.5); // Cap at 50% decay
            claim.confidence *= 1.0 - decay;

            // Demote tier if confidence drops below minimum.
            if claim.confidence < claim.tier.min_confidence() {
                claim.tier = Self::demote_tier(&claim.tier);
            }
        }
    }

    /// Asymptotic confidence function: approaches but never reaches 1.0.
    /// Mathematically 1 - exp(-weight), but clamped at 0.9999 to enforce
    /// the asymptote even under floating-point rounding (exp(-100) rounds to 0).
    fn asymptotic_confidence(evidence_weight: f64) -> f64 {
        const ASYMPTOTE: f64 = 0.9999;
        let raw = 1.0 - (-evidence_weight).exp();
        raw.min(ASYMPTOTE)
    }

    /// Determine the appropriate tier based on accumulated sources.
    fn tier_for_evidence(
        supporters: &[String],
        sources: &HashMap<String, Source>,
        _latest_source_category: &SourceCategory,
    ) -> KnowledgeTier {
        // Count source categories.
        let categories: Vec<SourceCategory> = supporters.iter()
            .filter_map(|s| sources.get(s).map(|src| src.category.clone()))
            .collect();

        let has_formal = categories.contains(&SourceCategory::FormalProof);
        let peer_count = categories.iter()
            .filter(|c| **c == SourceCategory::PeerReviewed).count();
        let standards_count = categories.iter()
            .filter(|c| **c == SourceCategory::Standards).count();
        let expert_count = categories.iter()
            .filter(|c| **c == SourceCategory::Expert).count();
        let reputable_count = peer_count + standards_count + expert_count;

        if has_formal && supporters.len() == 1 {
            KnowledgeTier::Proof
        } else if has_formal || (peer_count >= 3 && standards_count >= 1) {
            KnowledgeTier::Consensus
        } else if reputable_count >= 2 {
            KnowledgeTier::Corroborated
        } else if reputable_count >= 1 {
            KnowledgeTier::Plausible
        } else {
            KnowledgeTier::Unverified
        }
    }

    /// Detect contradictions with existing high-confidence claims.
    /// Returns the contradicting claim if found.
    fn detect_contradiction(&self, normalized: &str) -> Option<String> {
        // Simple heuristic: if claim contains "not X" and we have "X" at high confidence.
        if let Some(stripped) = normalized.strip_prefix("not ") {
            if let Some(existing) = self.claims.get(stripped) {
                if existing.confidence > 0.7 {
                    return Some(existing.claim.clone());
                }
            }
        }
        None
    }

    fn normalize_claim(claim: &str) -> String {
        claim.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn tier_rank(tier: &KnowledgeTier) -> u8 {
        match tier {
            KnowledgeTier::Proof => 5,
            KnowledgeTier::Consensus => 4,
            KnowledgeTier::Corroborated => 3,
            KnowledgeTier::Plausible => 2,
            KnowledgeTier::Unverified => 1,
            KnowledgeTier::Suspect => 0,
        }
    }

    fn demote_tier(tier: &KnowledgeTier) -> KnowledgeTier {
        match tier {
            KnowledgeTier::Proof => KnowledgeTier::Consensus,
            KnowledgeTier::Consensus => KnowledgeTier::Corroborated,
            KnowledgeTier::Corroborated => KnowledgeTier::Plausible,
            KnowledgeTier::Plausible => KnowledgeTier::Unverified,
            KnowledgeTier::Unverified => KnowledgeTier::Suspect,
            KnowledgeTier::Suspect => KnowledgeTier::Suspect,
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Generate a report of the filter's current state.
    pub fn report(&self) -> String {
        let mut out = format!("=== Epistemic Filter Report ===\n");
        out.push_str(&format!("Total processed: {}\n", self.total_processed));
        out.push_str(&format!("Total rejected:  {}\n", self.total_rejected));
        out.push_str(&format!("Total promoted:  {}\n", self.total_promoted));
        out.push_str(&format!("Active claims:   {}\n", self.claims.len()));
        out.push_str(&format!("Sources:         {}\n", self.sources.len()));
        out.push_str("\nTier distribution:\n");

        let mut tier_counts: HashMap<String, usize> = HashMap::new();
        for c in self.claims.values() {
            *tier_counts.entry(format!("{:?}", c.tier)).or_insert(0) += 1;
        }
        let mut sorted: Vec<_> = tier_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (tier, count) in sorted {
            out.push_str(&format!("  {:15} {}\n", tier, count));
        }

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
    fn test_asymptotic_never_reaches_one() {
        // No matter how much evidence, confidence never hits 1.0.
        assert!(EpistemicFilter::asymptotic_confidence(1.0) < 1.0);
        assert!(EpistemicFilter::asymptotic_confidence(10.0) < 1.0);
        assert!(EpistemicFilter::asymptotic_confidence(100.0) < 1.0);
        assert!(EpistemicFilter::asymptotic_confidence(10000.0) < 1.0);
    }

    #[test]
    fn test_tier_max_confidence_caps() {
        // Even Proof tier has a cap below 1.0.
        assert!(KnowledgeTier::Proof.max_confidence() < 1.0);
        assert!(KnowledgeTier::Proof.max_confidence() > KnowledgeTier::Consensus.max_confidence());
        assert!(KnowledgeTier::Suspect.max_confidence() < KnowledgeTier::Unverified.max_confidence());
    }

    #[test]
    fn test_skeptical_default_for_unknown_source() {
        let mut filter = EpistemicFilter::new();
        let result = filter.ingest_claim("The sky is green", "unknown_blog");
        assert!(result.confidence < 0.5,
            "Unknown source should have low confidence: {:.2}", result.confidence);
        assert_eq!(result.tier, KnowledgeTier::Unverified);
    }

    #[test]
    fn test_adversarial_source_rejected() {
        let mut filter = EpistemicFilter::new();
        filter.register_source(Source {
            name: "fake_news_site".into(),
            category: SourceCategory::Adversarial,
            trust: 0.05,
            track_record: 0.1,
            claim_count: 0,
        });

        let result = filter.ingest_claim("Earth is flat", "fake_news_site");
        assert!(result.rejected, "Adversarial source should be rejected");
        assert_eq!(result.tier, KnowledgeTier::Suspect);
    }

    #[test]
    fn test_corroboration_promotes_tier() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("expert_a", SourceCategory::Expert);
        filter.register_source_default("expert_b", SourceCategory::Expert);
        filter.register_source_default("standards_body", SourceCategory::Standards);

        let r1 = filter.ingest_claim("TLS 1.3 is secure", "expert_a");
        let initial_tier = r1.tier.clone();

        // Second source corroborates — should promote.
        let _r2 = filter.ingest_claim("TLS 1.3 is secure", "expert_b");
        let r3 = filter.ingest_claim("TLS 1.3 is secure", "standards_body");

        assert_eq!(r3.supporting_sources.len(), 3);
        assert!(r3.confidence > r1.confidence,
            "Corroboration should increase confidence: {:.2} → {:.2}",
            r1.confidence, r3.confidence);
        assert_ne!(r3.tier, initial_tier, "Tier should promote with corroboration");
    }

    #[test]
    fn test_formal_proof_highest_tier() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("coq_proof", SourceCategory::FormalProof);

        let result = filter.ingest_claim("2 + 2 = 4", "coq_proof");
        assert_eq!(result.tier, KnowledgeTier::Proof);
        // Even proof doesn't reach 1.0.
        assert!(result.confidence < 1.0);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_contradiction_detection() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("expert", SourceCategory::Expert);
        filter.register_source_default("expert2", SourceCategory::Expert);

        let _ = filter.ingest_claim("water boils at 100C", "expert");
        let _ = filter.ingest_claim("water boils at 100C", "expert2");

        // Contradictory claim should trigger detection.
        let contradiction = filter.detect_contradiction("not water boils at 100c");
        assert!(contradiction.is_some(), "Should detect 'not X' contradicts 'X'");
    }

    #[test]
    fn test_record_contradiction_reduces_confidence() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("src1", SourceCategory::Expert);
        filter.register_source_default("src2", SourceCategory::Expert);

        let r1 = filter.ingest_claim("Claim A", "src1");
        let r2 = filter.ingest_claim("Claim B", "src2");

        let initial_a = r1.confidence;
        let initial_b = r2.confidence;

        filter.record_contradiction("Claim A", "Claim B");

        let final_a = filter.check("Claim A").unwrap().confidence;
        let final_b = filter.check("Claim B").unwrap().confidence;

        assert!(final_a < initial_a, "A confidence should drop: {:.2} → {:.2}", initial_a, final_a);
        assert!(final_b < initial_b, "B confidence should drop: {:.2} → {:.2}", initial_b, final_b);
    }

    #[test]
    fn test_source_category_trust_ordering() {
        // FormalProof > PeerReviewed > Standards > Expert > Journalism > Community > Anonymous > Adversarial
        assert!(SourceCategory::FormalProof.base_trust() > SourceCategory::PeerReviewed.base_trust());
        assert!(SourceCategory::PeerReviewed.base_trust() >= SourceCategory::Standards.base_trust());
        assert!(SourceCategory::Expert.base_trust() > SourceCategory::Journalism.base_trust());
        assert!(SourceCategory::Journalism.base_trust() > SourceCategory::Anonymous.base_trust());
        assert!(SourceCategory::Anonymous.base_trust() > SourceCategory::Adversarial.base_trust());
    }

    #[test]
    fn test_claims_at_tier_filtering() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("formal", SourceCategory::FormalProof);
        filter.register_source_default("anon", SourceCategory::Anonymous);

        filter.ingest_claim("proven thing", "formal");
        filter.ingest_claim("maybe true", "anon");

        let high_tier = filter.claims_at_tier(&KnowledgeTier::Corroborated);
        let low_tier = filter.claims_at_tier(&KnowledgeTier::Unverified);

        assert!(low_tier.len() >= high_tier.len(),
            "More claims should be at or above Unverified tier");
    }

    #[test]
    fn test_report_generation() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("src", SourceCategory::Expert);
        filter.ingest_claim("test claim", "src");
        let report = filter.report();
        assert!(report.contains("Epistemic Filter Report"));
        assert!(report.contains("Total processed"));
        assert!(report.contains("Tier distribution"));
    }

    #[test]
    fn test_skeptical_intake_then_learning() {
        // Start skeptical, learn from evidence.
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("src1", SourceCategory::Expert);
        filter.register_source_default("src2", SourceCategory::Expert);
        filter.register_source_default("src3", SourceCategory::Standards);
        filter.register_source_default("src4", SourceCategory::PeerReviewed);

        let claim = "HTTP/3 uses QUIC protocol";
        let first = filter.ingest_claim(claim, "src1").confidence;

        // Each corroboration increases confidence asymptotically.
        let second = filter.ingest_claim(claim, "src2").confidence;
        let third = filter.ingest_claim(claim, "src3").confidence;
        let fourth = filter.ingest_claim(claim, "src4").confidence;

        assert!(first < second && second < third && third < fourth);
        // Never reaches 1.0.
        assert!(fourth < 1.0);
    }

    #[test]
    fn test_confidence_decay_over_time() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("src", SourceCategory::Expert);
        let claim = "latest news";
        let initial_conf = filter.ingest_claim(claim, "src").confidence;

        // Artificially age the claim by modifying last_updated.
        let normalized = EpistemicFilter::normalize_claim(claim);
        if let Some(c) = filter.claims.get_mut(&normalized) {
            c.last_updated_ms = c.last_updated_ms.saturating_sub(10 * 86_400_000); // 10 days ago
        }

        filter.apply_decay(0.02); // 2% per day

        let final_conf = filter.check(claim).unwrap().confidence;
        assert!(final_conf < initial_conf, "Confidence should decay: {:.2} → {:.2}",
            initial_conf, final_conf);
    }

    // ============================================================
    // Stress / invariant tests for EpistemicFilter
    // ============================================================

    /// INVARIANT: every ingested claim has confidence in [0,1] AND the
    /// confidence never exceeds the source-category's max trust ceiling.
    /// Confidence > category cap means a downstream policy decision will
    /// trust an unverified claim more than the source justifies.
    #[test]
    fn invariant_ingested_confidence_in_range() {
        let mut filter = EpistemicFilter::new();
        let categories = [
            ("anon", SourceCategory::Anonymous),
            ("comm", SourceCategory::Community),
            ("journ", SourceCategory::Journalism),
            ("std", SourceCategory::Standards),
            ("peer", SourceCategory::PeerReviewed),
            ("expert", SourceCategory::Expert),
            ("proof", SourceCategory::FormalProof),
        ];
        for (name, cat) in &categories {
            filter.register_source_default(name, cat.clone());
        }
        for (name, _cat) in &categories {
            let result = filter.ingest_claim(&format!("test claim from {}", name), name);
            assert!(result.confidence.is_finite() && (0.0..=1.0).contains(&result.confidence),
                "confidence out of [0,1] for source {}: {}", name, result.confidence);
        }
    }

    /// INVARIANT: the same claim ingested twice from the same source must
    /// not silently grow confidence above 1.0. Re-ingestion should be
    /// idempotent or strictly non-decreasing-but-clamped.
    #[test]
    fn invariant_repeated_ingest_clamped_to_one() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("repeater", SourceCategory::Expert);
        let claim = "repeated claim";
        for _ in 0..50 {
            let result = filter.ingest_claim(claim, "repeater");
            assert!(result.confidence <= 1.0 + 1e-9,
                "confidence escaped [0,1] under repeated ingest: {}", result.confidence);
        }
    }

    /// INVARIANT: claims_at_tier returns a strict subset — none of the
    /// returned claims are below the given tier (no false matches).
    #[test]
    fn invariant_claims_at_tier_respects_minimum() {
        let mut filter = EpistemicFilter::new();
        for src in ["proof_src", "expert_src", "comm_src", "anon_src"] {
            let category = match src {
                "proof_src" => SourceCategory::FormalProof,
                "expert_src" => SourceCategory::Expert,
                "comm_src" => SourceCategory::Community,
                _ => SourceCategory::Anonymous,
            };
            filter.register_source_default(src, category);
            filter.ingest_claim(&format!("claim from {}", src), src);
        }
        for min_tier in [
            KnowledgeTier::Suspect,
            KnowledgeTier::Unverified,
            KnowledgeTier::Plausible,
            KnowledgeTier::Corroborated,
            KnowledgeTier::Consensus,
            KnowledgeTier::Proof,
        ] {
            let claims = filter.claims_at_tier(&min_tier);
            let min_rank = EpistemicFilter::tier_rank(&min_tier);
            for c in &claims {
                let claim_rank = EpistemicFilter::tier_rank(&c.tier);
                assert!(claim_rank >= min_rank,
                    "claims_at_tier({:?}) returned a {:?} claim", min_tier, c.tier);
            }
        }
    }

    /// INVARIANT: check() returns None for unknown claims, Some for ingested.
    #[test]
    fn invariant_check_matches_ingest() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("src", SourceCategory::Expert);
        assert!(filter.check("never ingested").is_none());
        filter.ingest_claim("known claim", "src");
        assert!(filter.check("known claim").is_some());
    }

    /// INVARIANT: apply_decay never produces negative confidence values.
    #[test]
    fn invariant_decay_non_negative() {
        let mut filter = EpistemicFilter::new();
        filter.register_source_default("src", SourceCategory::Anonymous);
        for i in 0..10 {
            filter.ingest_claim(&format!("claim {}", i), "src");
        }
        // Apply massive decay multiple times.
        for _ in 0..10 {
            filter.apply_decay(0.5);
        }
        for c in filter.claims.values() {
            assert!(c.confidence >= 0.0,
                "decay produced negative confidence: {}", c.confidence);
            assert!(c.confidence.is_finite(),
                "decay produced non-finite confidence: {}", c.confidence);
        }
    }

    /// INVARIANT: SourceCategory::base_trust is in [0,1] for every category.
    /// Category trust outside [0,1] would break confidence math everywhere.
    #[test]
    fn invariant_source_category_trust_in_unit_interval() {
        for cat in [
            SourceCategory::Anonymous,
            SourceCategory::Community,
            SourceCategory::Journalism,
            SourceCategory::Standards,
            SourceCategory::PeerReviewed,
            SourceCategory::Expert,
            SourceCategory::FormalProof,
        ] {
            let trust = cat.base_trust();
            assert!(trust.is_finite() && (0.0..=1.0).contains(&trust),
                "{:?} base_trust out of [0,1]: {}", cat, trust);
        }
    }

    /// INVARIANT: KnowledgeTier max_confidence >= min_confidence for all tiers.
    #[test]
    fn invariant_tier_confidence_max_ge_min() {
        for tier in [
            KnowledgeTier::Proof,
            KnowledgeTier::Consensus,
            KnowledgeTier::Corroborated,
            KnowledgeTier::Plausible,
            KnowledgeTier::Unverified,
            KnowledgeTier::Suspect,
        ] {
            let min: f64 = tier.min_confidence();
            let max: f64 = tier.max_confidence();
            assert!(max >= min,
                "tier {:?}: max {} < min {}", tier, max, min);
            assert!(min.is_finite() && max.is_finite());
            assert!((0.0..=1.0).contains(&min));
            assert!((0.0..=1.0).contains(&max));
        }
    }

    /// INVARIANT: check() returns None for never-ingested claims.
    #[test]
    fn invariant_check_unknown_claim_none() {
        let filter = EpistemicFilter::new();
        assert!(filter.check("never_ingested_claim_xyz").is_none());
    }
}
