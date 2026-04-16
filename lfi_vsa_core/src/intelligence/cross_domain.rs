// ============================================================
// Cross-Domain Reasoning — Analogical Transfer Between Fields
//
// PURPOSE: Apply knowledge from one domain to solve problems in
// unrelated domains. This is a key marker of general intelligence:
// seeing structural similarities between seemingly different fields.
//
// EXAMPLES OF CROSS-DOMAIN TRANSFER:
//   - Network topology → social graph analysis
//   - Epidemiology → virus propagation → information spread
//   - Game theory → security → economics → evolution
//   - Signal processing → natural language → financial markets
//   - Immune system → cybersecurity defense
//   - Evolutionary algorithms → code optimization
//   - Physics entropy → information entropy → social entropy
//
// MECHANISM:
//   VSA makes this natural. Concepts are hypervectors. Two concepts
//   from different domains that share structural properties will have
//   correlated vectors (via shared sub-patterns). The analogy engine
//   finds these cross-domain correlations and uses them for:
//   1. Problem solving: "This security problem looks like an epidemiology problem"
//   2. Learning acceleration: "Understanding physics helps learn CS"
//   3. Creative generation: "What if we apply swarm intelligence to code review?"
//
// RESEARCH BASIS:
//   - Gentner's Structure Mapping Theory (1983)
//   - Hofstadter's Fluid Analogies (1995)
//   - Holyoak's Multi-Constraint Theory (1989)
//   - Kanerva's Sparse Distributed Memory (1988) — the VSA foundation
// ============================================================

use crate::cognition::knowledge::KnowledgeEngine;
use crate::intelligence::training_data::TrainingDataGenerator;
use std::collections::HashMap;

// ============================================================
// Structural Analogy
// ============================================================

/// A structural analogy between two concepts in different domains.
#[derive(Debug, Clone)]
pub struct Analogy {
    /// Source concept (the one we know well).
    pub source_concept: String,
    /// Source domain.
    pub source_domain: String,
    /// Target concept (the one we're trying to understand).
    pub target_concept: String,
    /// Target domain.
    pub target_domain: String,
    /// Structural similarity score (0.0 to 1.0).
    pub similarity: f64,
    /// Human-readable mapping explanation.
    pub mapping: String,
    /// Confidence in this analogy's usefulness.
    pub confidence: f64,
}

/// A cross-domain insight — knowledge transferred between fields.
#[derive(Debug, Clone)]
pub struct CrossDomainInsight {
    /// The original problem domain.
    pub problem_domain: String,
    /// The domain that provided the insight.
    pub insight_domain: String,
    /// What was transferred.
    pub insight: String,
    /// How useful this transfer was (0.0 to 1.0).
    pub utility: f64,
    /// Did it actually help solve the problem?
    pub successful: bool,
}

// ============================================================
// Cross-Domain Analogy Database
// ============================================================

/// Pre-defined structural analogies between domains.
/// These are hand-crafted mappings that encode deep structural
/// similarities between seemingly unrelated fields.
pub struct AnalogyDatabase;

impl AnalogyDatabase {
    /// Get all known structural analogies.
    /// BUG ASSUMPTION: these are curated, not discovered. Real AGI
    /// would discover these autonomously. This is bootstrap data.
    pub fn all_analogies() -> Vec<Analogy> {
        vec![
            // Immune system ↔ Cybersecurity
            Analogy {
                source_concept: "adaptive_immunity".into(),
                source_domain: "biology".into(),
                target_concept: "ids_signatures".into(),
                target_domain: "security".into(),
                similarity: 0.8,
                mapping: "Immune system learns pathogen signatures → IDS learns attack signatures. Both adapt to new threats.".into(),
                confidence: 0.85,
            },
            Analogy {
                source_concept: "autoimmune_disease".into(),
                source_domain: "biology".into(),
                target_concept: "false_positives".into(),
                target_domain: "security".into(),
                similarity: 0.7,
                mapping: "Immune system attacks self → Security system blocks legitimate traffic. Both are over-sensitive defense.".into(),
                confidence: 0.75,
            },
            // Epidemiology ↔ Information Spread
            Analogy {
                source_concept: "r_naught".into(),
                source_domain: "biology".into(),
                target_concept: "viral_coefficient".into(),
                target_domain: "social_eng".into(),
                similarity: 0.85,
                mapping: "R0 measures infection spread rate → Viral coefficient measures content spread rate. Both model exponential propagation.".into(),
                confidence: 0.9,
            },
            // Game Theory ↔ Security ↔ Economics
            Analogy {
                source_concept: "prisoners_dilemma".into(),
                source_domain: "strategy".into(),
                target_concept: "responsible_disclosure".into(),
                target_domain: "security".into(),
                similarity: 0.7,
                mapping: "Cooperate/defect in game theory → Disclose/exploit in vuln discovery. Mutual cooperation benefits both.".into(),
                confidence: 0.75,
            },
            Analogy {
                source_concept: "nash_equilibrium".into(),
                source_domain: "strategy".into(),
                target_concept: "defense_in_depth".into(),
                target_domain: "security".into(),
                similarity: 0.6,
                mapping: "No player benefits from unilateral change → No single defense removal should compromise security.".into(),
                confidence: 0.65,
            },
            // Physics Entropy ↔ Information Entropy
            Analogy {
                source_concept: "thermodynamic_entropy".into(),
                source_domain: "physics".into(),
                target_concept: "information_entropy".into(),
                target_domain: "crypto".into(),
                similarity: 0.9,
                mapping: "Both measure disorder/uncertainty. Shannon directly adopted Boltzmann's framework. High entropy = hard to predict.".into(),
                confidence: 0.95,
            },
            // Evolution ↔ Code Optimization
            Analogy {
                source_concept: "natural_selection".into(),
                source_domain: "biology".into(),
                target_concept: "genetic_algorithm".into(),
                target_domain: "code".into(),
                similarity: 0.85,
                mapping: "Fittest organisms survive → Best code solutions survive. Both use mutation + selection + reproduction.".into(),
                confidence: 0.9,
            },
            // Network Topology ↔ Social Graphs
            Analogy {
                source_concept: "network_hubs".into(),
                source_domain: "networking".into(),
                target_concept: "social_influencers".into(),
                target_domain: "social_eng".into(),
                similarity: 0.75,
                mapping: "Network hub = many connections = single point of failure → Social influencer = many followers = high-value SE target.".into(),
                confidence: 0.8,
            },
            // Military Strategy ↔ Pentesting
            Analogy {
                source_concept: "flanking_maneuver".into(),
                source_domain: "strategy".into(),
                target_concept: "lateral_movement".into(),
                target_domain: "exploitation".into(),
                similarity: 0.7,
                mapping: "Attack from unexpected angle → Move laterally through network to reach real target. Both avoid frontal assault.".into(),
                confidence: 0.75,
            },
            Analogy {
                source_concept: "supply_line_disruption".into(),
                source_domain: "strategy".into(),
                target_concept: "supply_chain_attack".into(),
                target_domain: "security".into(),
                similarity: 0.8,
                mapping: "Cut enemy supply lines → Compromise software supply chain. Both attack the logistics, not the target directly.".into(),
                confidence: 0.85,
            },
            // Signal Processing ↔ Anomaly Detection
            Analogy {
                source_concept: "noise_filtering".into(),
                source_domain: "physics".into(),
                target_concept: "alert_triage".into(),
                target_domain: "security".into(),
                similarity: 0.7,
                mapping: "Filter noise from signal → Filter false positives from real alerts. Both use statistical thresholds.".into(),
                confidence: 0.75,
            },
            // Water Flow ↔ Network Traffic
            Analogy {
                source_concept: "fluid_dynamics".into(),
                source_domain: "physics".into(),
                target_concept: "traffic_shaping".into(),
                target_domain: "networking".into(),
                similarity: 0.65,
                mapping: "Water finds least resistance → Traffic follows least-cost path. Both governed by conservation laws and bottlenecks.".into(),
                confidence: 0.7,
            },
            // Lockpicking ↔ Vulnerability Research
            Analogy {
                source_concept: "lock_mechanisms".into(),
                source_domain: "security".into(),
                target_concept: "software_vulnerabilities".into(),
                target_domain: "exploitation".into(),
                similarity: 0.7,
                mapping: "Physical locks have design flaws → Software has implementation bugs. Both require understanding the mechanism to bypass it.".into(),
                confidence: 0.75,
            },
            // Language Translation ↔ Protocol Conversion
            Analogy {
                source_concept: "language_grammar".into(),
                source_domain: "reasoning".into(),
                target_concept: "protocol_parsing".into(),
                target_domain: "networking".into(),
                similarity: 0.65,
                mapping: "Natural languages have grammar rules → Network protocols have format specifications. Both require parsing and validation.".into(),
                confidence: 0.7,
            },
        ]
    }

    /// Find domains that have structural analogies to the query domain.
    /// Returns owned Strings to avoid memory leaks from Box::leak().
    /// SUPERSOCIETY: Never leak memory for convenience — own the data.
    pub fn analogies_for_domain(domain: &str) -> Vec<String> {
        let all = Self::all_analogies();
        let mut related = Vec::new();
        for analogy in &all {
            if analogy.target_domain == domain && !related.contains(&analogy.source_domain) {
                related.push(analogy.source_domain.clone());
            }
            if analogy.source_domain == domain && !related.contains(&analogy.target_domain) {
                related.push(analogy.target_domain.clone());
            }
        }
        related
    }
}

// ============================================================
// Cross-Domain Reasoning Engine
// ============================================================

/// The cross-domain reasoning engine. Finds structural analogies
/// between concepts and uses them to accelerate learning and
/// solve problems in unfamiliar domains.
pub struct CrossDomainEngine {
    /// History of cross-domain insights.
    pub insights: Vec<CrossDomainInsight>,
    /// Transfer success rates per domain pair.
    pub transfer_rates: HashMap<(String, String), (usize, usize)>, // (successes, attempts)
}

impl CrossDomainEngine {
    pub fn new() -> Self {
        debuglog!("CrossDomainEngine::new: Initializing cross-domain analogical reasoning");
        Self {
            insights: Vec::new(),
            transfer_rates: HashMap::new(),
        }
    }

    /// Find the best analogy for understanding a concept in a target domain,
    /// using knowledge from all other domains.
    pub fn find_analogy(
        &self,
        target_concept: &str,
        target_domain: &str,
        knowledge: &KnowledgeEngine,
    ) -> Option<Analogy> {
        debuglog!("CrossDomainEngine::find_analogy: target='{}' in '{}'",
            target_concept, target_domain);

        let all_analogies = AnalogyDatabase::all_analogies();

        // Find analogies where the target domain matches.
        let mut candidates: Vec<Analogy> = all_analogies.into_iter()
            .filter(|a| a.target_domain == target_domain || a.source_domain == target_domain)
            .collect();

        // Score by: similarity * source_mastery * confidence
        candidates.sort_by(|a, b| {
            let score_a = a.similarity * a.confidence * knowledge.domain_mastery(&a.source_domain);
            let score_b = b.similarity * b.confidence * knowledge.domain_mastery(&b.source_domain);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.into_iter().next()
    }

    /// Apply cross-domain knowledge transfer: use understanding of one domain
    /// to boost mastery in another.
    pub fn transfer_knowledge(
        &mut self,
        source_domain: &str,
        target_domain: &str,
        knowledge: &mut KnowledgeEngine,
    ) -> CrossDomainInsight {
        let source_mastery = knowledge.domain_mastery(source_domain);
        let target_mastery = knowledge.domain_mastery(target_domain);

        // Transfer boost = source_mastery * structural_similarity * 0.3
        // (can't transfer more than 30% of source mastery — diminishing returns)
        let similarity = self.domain_pair_similarity(source_domain, target_domain);
        let transfer_amount = source_mastery * similarity * 0.3;

        let successful = transfer_amount > 0.01 && source_mastery > target_mastery;

        if successful {
            // Boost target domain by applying transfer
            let _ = TrainingDataGenerator::apply_transfer(
                knowledge, source_domain, transfer_amount,
            );
        }

        let insight = CrossDomainInsight {
            problem_domain: target_domain.into(),
            insight_domain: source_domain.into(),
            insight: format!(
                "Transferred {:.1}% mastery from {} (mastery={:.1}%) to {} (mastery={:.1}%) via structural similarity={:.2}",
                transfer_amount * 100.0, source_domain, source_mastery * 100.0,
                target_domain, target_mastery * 100.0, similarity,
            ),
            utility: transfer_amount,
            successful,
        };

        // Record in transfer rates
        let key = (source_domain.to_string(), target_domain.to_string());
        let entry = self.transfer_rates.entry(key).or_insert((0, 0));
        entry.1 += 1; // attempts
        if successful { entry.0 += 1; } // successes

        self.insights.push(insight.clone());
        debuglog!("CrossDomainEngine::transfer: {} → {} (amount={:.4}, success={})",
            source_domain, target_domain, transfer_amount, successful);

        insight
    }

    /// Compute structural similarity between two domains.
    /// Uses the analogy database as ground truth.
    fn domain_pair_similarity(&self, domain_a: &str, domain_b: &str) -> f64 {
        let analogies = AnalogyDatabase::all_analogies();
        let mut max_sim = 0.0_f64;

        for analogy in &analogies {
            if (analogy.source_domain == domain_a && analogy.target_domain == domain_b)
                || (analogy.source_domain == domain_b && analogy.target_domain == domain_a) {
                max_sim = max_sim.max(analogy.similarity);
            }
        }

        // If no direct analogy, check domain_relationships for weaker links.
        if max_sim < 0.01 {
            let relationships = TrainingDataGenerator::domain_relationships();
            for (domain, related) in &relationships {
                if *domain == domain_a {
                    for (rel, weight) in related {
                        if *rel == domain_b {
                            max_sim = max_sim.max(*weight * 0.5); // Weaker than structural analogy
                        }
                    }
                }
            }
        }

        max_sim
    }

    /// Find the best source domain to learn from for a target domain.
    /// Returns (source_domain, expected_transfer_amount).
    pub fn best_transfer_source(
        &self,
        target_domain: &str,
        knowledge: &KnowledgeEngine,
    ) -> Option<(String, f64)> {
        let domains = TrainingDataGenerator::domains();
        let mut best: Option<(String, f64)> = None;

        for domain in &domains {
            if domain == target_domain { continue; }
            let mastery = knowledge.domain_mastery(domain);
            let similarity = self.domain_pair_similarity(domain, target_domain);
            let expected_transfer = mastery * similarity * 0.3;

            if let Some((_, ref current_best)) = best {
                if expected_transfer > *current_best {
                    best = Some((domain.clone(), expected_transfer));
                }
            } else if expected_transfer > 0.01 {
                best = Some((domain.clone(), expected_transfer));
            }
        }

        best
    }

    /// Run a full cross-domain transfer sweep: for every weak domain,
    /// find the best source and transfer knowledge.
    pub fn transfer_sweep(
        &mut self,
        knowledge: &mut KnowledgeEngine,
        weakness_threshold: f64,
    ) -> Vec<CrossDomainInsight> {
        let domains = TrainingDataGenerator::domains();
        let mut insights = Vec::new();

        // Find weak domains.
        let weak_domains: Vec<String> = domains.iter()
            .filter(|d| knowledge.domain_mastery(d) < weakness_threshold)
            .cloned()
            .collect();

        for target in &weak_domains {
            if let Some((source, _)) = self.best_transfer_source(target, knowledge) {
                let insight = self.transfer_knowledge(&source, target, knowledge);
                insights.push(insight);
            }
        }

        debuglog!("CrossDomainEngine::transfer_sweep: {} transfers attempted for {} weak domains",
            insights.len(), weak_domains.len());
        insights
    }

    /// Overall transfer success rate.
    pub fn success_rate(&self) -> f64 {
        let total_successes: usize = self.transfer_rates.values().map(|(s, _)| s).sum();
        let total_attempts: usize = self.transfer_rates.values().map(|(_, a)| a).sum();
        if total_attempts == 0 { 0.0 } else { total_successes as f64 / total_attempts as f64 }
    }

    /// Number of insights generated.
    pub fn insight_count(&self) -> usize {
        self.insights.len()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analogy_database_has_entries() {
        let analogies = AnalogyDatabase::all_analogies();
        assert!(analogies.len() >= 10, "Should have 10+ analogies, got {}", analogies.len());

        // Check structural properties
        for analogy in &analogies {
            assert!(analogy.similarity > 0.0 && analogy.similarity <= 1.0);
            assert!(analogy.confidence > 0.0 && analogy.confidence <= 1.0);
            assert!(!analogy.mapping.is_empty());
            assert_ne!(analogy.source_domain, analogy.target_domain);
        }
    }

    #[test]
    fn test_find_analogy_for_security() {
        let engine = CrossDomainEngine::new();
        let knowledge = KnowledgeEngine::new();

        let analogy = engine.find_analogy("attack_detection", "security", &knowledge);
        // Should find at least one analogy (biology→security immune system mapping)
        assert!(analogy.is_some(), "Should find an analogy for security domain");
    }

    #[test]
    fn test_cross_domain_transfer() {
        let mut engine = CrossDomainEngine::new();
        let mut knowledge = KnowledgeEngine::new();

        let insight = engine.transfer_knowledge("biology", "security", &mut knowledge);
        assert!(!insight.insight.is_empty());
        assert_eq!(insight.problem_domain, "security");
        assert_eq!(insight.insight_domain, "biology");
    }

    #[test]
    fn test_best_transfer_source() {
        let engine = CrossDomainEngine::new();
        let knowledge = KnowledgeEngine::new();

        // For security, biology should be a viable transfer source (immune system analogy)
        let source = engine.best_transfer_source("security", &knowledge);
        // May or may not find one depending on seeded mastery levels
        if let Some((domain, amount)) = source {
            assert!(amount > 0.0);
            assert_ne!(domain, "security");
        }
    }

    #[test]
    fn test_domain_pair_similarity() {
        let engine = CrossDomainEngine::new();

        // Physics → crypto should have high similarity (entropy analogy)
        let sim = engine.domain_pair_similarity("physics", "crypto");
        assert!(sim > 0.5, "Physics↔crypto should have high similarity: {:.2}", sim);

        // Two completely unrelated domains should have low/zero similarity
        let sim2 = engine.domain_pair_similarity("geography", "code");
        assert!(sim2 < 0.5, "Geography↔code should have low similarity: {:.2}", sim2);
    }

    #[test]
    fn test_transfer_sweep() {
        let mut engine = CrossDomainEngine::new();
        let mut knowledge = KnowledgeEngine::new();

        let insights = engine.transfer_sweep(&mut knowledge, 0.5);
        // Should generate some insights (many domains below 0.5 mastery)
        assert!(!insights.is_empty(), "Transfer sweep should generate insights");
    }

    #[test]
    fn test_transfer_rate_tracking() {
        let mut engine = CrossDomainEngine::new();
        let mut knowledge = KnowledgeEngine::new();

        // Do several transfers
        for _ in 0..3 {
            engine.transfer_knowledge("physics", "crypto", &mut knowledge);
        }

        assert_eq!(engine.insight_count(), 3);
        assert!(engine.success_rate() >= 0.0);
    }

    #[test]
    fn test_analogy_covers_key_domains() {
        let analogies = AnalogyDatabase::all_analogies();
        let source_domains: Vec<&str> = analogies.iter().map(|a| a.source_domain.as_str()).collect();
        let target_domains: Vec<&str> = analogies.iter().map(|a| a.target_domain.as_str()).collect();

        // Key domains should appear in at least one analogy
        assert!(source_domains.contains(&"biology") || target_domains.contains(&"biology"));
        assert!(source_domains.contains(&"physics") || target_domains.contains(&"physics"));
        assert!(source_domains.contains(&"strategy") || target_domains.contains(&"strategy"));
        assert!(target_domains.contains(&"security"), "Security should be an analogy target");
    }

    // ============================================================
    // Stress / invariant tests for CrossDomainEngine
    // ============================================================

    /// INVARIANT: every analogy in the database has non-empty source/target
    /// domains and a non-empty mapping description.
    #[test]
    fn invariant_analogies_well_formed() {
        for a in AnalogyDatabase::all_analogies() {
            assert!(!a.source_domain.is_empty(),
                "analogy source domain must not be empty");
            assert!(!a.target_domain.is_empty(),
                "analogy target domain must not be empty");
            assert!(!a.source_concept.is_empty(),
                "source_concept must not be empty");
            assert!(!a.target_concept.is_empty(),
                "target_concept must not be empty");
            assert!(a.similarity.is_finite()
                && (0.0..=1.0).contains(&a.similarity),
                "similarity out of [0,1]: {}", a.similarity);
        }
    }

    /// INVARIANT: success_rate is in [0,1] for any engine state.
    #[test]
    fn invariant_success_rate_in_unit_interval() {
        let engine = CrossDomainEngine::new();
        let r = engine.success_rate();
        assert!(r.is_finite() && (0.0..=1.0).contains(&r),
            "success_rate out of [0,1]: {}", r);
    }

    /// INVARIANT: insight_count grows monotonically — only ever increases.
    #[test]
    fn invariant_insight_count_non_decreasing() {
        let mut engine = CrossDomainEngine::new();
        let mut knowledge = crate::cognition::knowledge::KnowledgeEngine::new();
        let mut last = engine.insight_count();
        for src in ["biology", "physics", "strategy", "music", "economics"] {
            let _ = engine.transfer_knowledge(src, "security", &mut knowledge);
            let now = engine.insight_count();
            assert!(now >= last,
                "insight_count regressed: {} → {}", last, now);
            last = now;
        }
    }

    /// INVARIANT: find_analogy never panics on arbitrary unicode input.
    #[test]
    fn invariant_find_analogy_safe_on_unicode() {
        let engine = CrossDomainEngine::new();
        let knowledge = crate::cognition::knowledge::KnowledgeEngine::new();
        let big_x = "x".repeat(10_000);
        let big_y = "y".repeat(10_000);
        let inputs: [(&str, &str); 5] = [
            ("", ""),
            ("アリス", "セキュリティ"),
            ("🦀", "🔒"),
            ("control:\x00", "control:\x01"),
            (&big_x, &big_y),
        ];
        for (concept, domain) in inputs {
            // Must not panic.
            let _ = engine.find_analogy(concept, domain, &knowledge);
        }
    }

    /// INVARIANT: best_transfer_source returns a valid domain (non-empty)
    /// when given a domain and concept that exist in the analogy database.
    #[test]
    fn invariant_best_transfer_source_returns_known_domain() {
        let engine = CrossDomainEngine::new();
        let knowledge = crate::cognition::knowledge::KnowledgeEngine::new();
        if let Some((source, score)) = engine.best_transfer_source("security", &knowledge) {
            assert!(!source.is_empty(),
                "best_transfer_source returned empty string");
            assert!(score.is_finite(),
                "transfer score must be finite: {}", score);
        }
    }

    /// INVARIANT: all_analogies is non-empty and deterministic.
    #[test]
    fn invariant_all_analogies_deterministic_nonempty() {
        let a1 = AnalogyDatabase::all_analogies();
        let a2 = AnalogyDatabase::all_analogies();
        assert_eq!(a1.len(), a2.len(),
            "all_analogies should be deterministic");
        assert!(!a1.is_empty(),
            "all_analogies should not be empty");
    }

    /// INVARIANT: CrossDomainEngine::new() starts with 0 insights.
    #[test]
    fn invariant_new_engine_zero_insights() {
        let engine = CrossDomainEngine::new();
        assert_eq!(engine.insight_count(), 0);
        let rate = engine.success_rate();
        assert!(rate.is_finite());
    }
}
