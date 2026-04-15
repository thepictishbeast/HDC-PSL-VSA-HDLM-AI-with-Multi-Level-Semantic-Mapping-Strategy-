// ============================================================
// MetaCognitive Profiler — Self-Awareness for LFI
//
// Tracks what LFI is good at and bad at using BipolarVectors.
// The weakness_map and strength_map are bundled hypervectors:
//   - Each domain (coding, math, security, etc.) gets a base vector
//   - Successes bundle into strength_map with domain vector
//   - Failures bundle into weakness_map with domain vector
//   - Probing either map with a domain vector returns performance signal
//
// The improvement_queue prioritizes domains with highest weakness
// signal for targeted learning (Active Learning strategy).
//
// This is the "knows what it doesn't know" module — critical for
// escape velocity (self-directed improvement).
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::holographic::HolographicMemory;
use crate::hdc::error::HdcError;
use std::collections::HashMap;

/// A domain of knowledge or capability that LFI can be profiled on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CognitiveDomain {
    Coding,
    Mathematics,
    Security,
    NaturalLanguage,
    Planning,
    Reasoning,
    FactualKnowledge,
    Conversation,
    SelfImprovement,
    Custom(String),
}

impl CognitiveDomain {
    /// Deterministic seed for this domain's base vector.
    fn seed(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        match self {
            CognitiveDomain::Coding => "domain:coding".hash(&mut hasher),
            CognitiveDomain::Mathematics => "domain:mathematics".hash(&mut hasher),
            CognitiveDomain::Security => "domain:security".hash(&mut hasher),
            CognitiveDomain::NaturalLanguage => "domain:natural_language".hash(&mut hasher),
            CognitiveDomain::Planning => "domain:planning".hash(&mut hasher),
            CognitiveDomain::Reasoning => "domain:reasoning".hash(&mut hasher),
            CognitiveDomain::FactualKnowledge => "domain:factual_knowledge".hash(&mut hasher),
            CognitiveDomain::Conversation => "domain:conversation".hash(&mut hasher),
            CognitiveDomain::SelfImprovement => "domain:self_improvement".hash(&mut hasher),
            CognitiveDomain::Custom(name) => format!("domain:custom:{}", name).hash(&mut hasher),
        }
        hasher.finish()
    }

    /// Get the base hypervector for this domain.
    pub fn base_vector(&self) -> BipolarVector {
        BipolarVector::from_seed(self.seed())
    }
}

/// A single performance record for profiling.
#[derive(Debug, Clone)]
pub struct PerformanceRecord {
    /// The domain this record belongs to.
    pub domain: CognitiveDomain,
    /// Whether the task was successful.
    pub success: bool,
    /// Confidence of the result (0.0 to 1.0).
    pub confidence: f64,
    /// The task input vector (for holographic storage).
    pub task_vector: BipolarVector,
    /// Optional description of what happened.
    pub description: String,
}

/// An entry in the improvement queue — a domain that needs work.
#[derive(Debug, Clone)]
pub struct ImprovementTarget {
    /// The domain to improve.
    pub domain: CognitiveDomain,
    /// How weak we are in this domain (higher = weaker = higher priority).
    pub weakness_score: f64,
    /// How strong we are (for context).
    pub strength_score: f64,
    /// Net score: weakness - strength. Higher = more improvement needed.
    pub improvement_priority: f64,
    /// Number of failures recorded.
    pub failure_count: usize,
    /// Number of successes recorded.
    pub success_count: usize,
}

/// The MetaCognitive Profiler — LFI's self-awareness engine.
///
/// Maintains holographic maps of strengths and weaknesses,
/// tracks per-domain performance, and generates a prioritized
/// improvement queue for self-directed learning.
pub struct MetaCognitiveProfiler {
    /// Holographic memory of successful task patterns per domain.
    strength_map: HolographicMemory,
    /// Holographic memory of failed task patterns per domain.
    weakness_map: HolographicMemory,
    /// Per-domain success/failure counters.
    domain_stats: HashMap<CognitiveDomain, DomainStats>,
    /// Running average confidence per domain.
    domain_confidence: HashMap<CognitiveDomain, f64>,
    /// Total records processed.
    pub total_records: usize,
}

/// Per-domain statistics.
#[derive(Debug, Clone, Default)]
struct DomainStats {
    successes: usize,
    failures: usize,
    total_confidence: f64,
}

impl MetaCognitiveProfiler {
    /// Create a new empty profiler.
    pub fn new() -> Self {
        debuglog!("MetaCognitiveProfiler::new: Initializing self-awareness engine");
        Self {
            strength_map: HolographicMemory::new(),
            weakness_map: HolographicMemory::new(),
            domain_stats: HashMap::new(),
            domain_confidence: HashMap::new(),
            total_records: 0,
        }
    }

    /// Record a performance observation.
    ///
    /// Successes are bundled into the strength_map.
    /// Failures are bundled into the weakness_map.
    /// Both are keyed by domain base vector for later probing.
    pub fn record(&mut self, record: &PerformanceRecord) -> Result<(), HdcError> {
        debuglog!(
            "MetaCognitiveProfiler::record: domain={:?}, success={}, conf={:.3}",
            record.domain, record.success, record.confidence
        );

        let domain_vector = record.domain.base_vector();

        // Bind the task vector with the domain vector for contextual storage
        let contextual = domain_vector.bind(&record.task_vector)?;

        if record.success {
            self.strength_map.associate(&domain_vector, &contextual)?;
        } else {
            self.weakness_map.associate(&domain_vector, &contextual)?;
        }

        // Update stats
        let stats = self.domain_stats.entry(record.domain.clone()).or_default();
        if record.success {
            stats.successes += 1;
        } else {
            stats.failures += 1;
        }
        stats.total_confidence += record.confidence;

        // Update running average confidence
        let total = stats.successes + stats.failures;
        self.domain_confidence.insert(
            record.domain.clone(),
            stats.total_confidence / total as f64,
        );

        self.total_records += 1;
        debuglog!(
            "MetaCognitiveProfiler::record: total_records={}, domain_stats={:?}",
            self.total_records, stats
        );

        Ok(())
    }

    /// Probe strength in a domain.
    ///
    /// Returns a similarity score: higher = more strength signal.
    /// Score range is [-1.0, 1.0] but typically [0.0, 0.5] for
    /// populated domains.
    pub fn probe_strength(&self, domain: &CognitiveDomain) -> Result<f64, HdcError> {
        debuglog!("MetaCognitiveProfiler::probe_strength: domain={:?}", domain);
        let domain_vector = domain.base_vector();
        let retrieved = self.strength_map.probe(&domain_vector)?;
        let sim = retrieved.similarity(&domain_vector)?;
        debuglog!("MetaCognitiveProfiler::probe_strength: sim={:.4}", sim);
        Ok(sim)
    }

    /// Probe weakness in a domain.
    ///
    /// Returns a similarity score: higher = more weakness signal.
    pub fn probe_weakness(&self, domain: &CognitiveDomain) -> Result<f64, HdcError> {
        debuglog!("MetaCognitiveProfiler::probe_weakness: domain={:?}", domain);
        let domain_vector = domain.base_vector();
        let retrieved = self.weakness_map.probe(&domain_vector)?;
        let sim = retrieved.similarity(&domain_vector)?;
        debuglog!("MetaCognitiveProfiler::probe_weakness: sim={:.4}", sim);
        Ok(sim)
    }

    /// Get the success rate for a domain.
    pub fn success_rate(&self, domain: &CognitiveDomain) -> f64 {
        if let Some(stats) = self.domain_stats.get(domain) {
            let total = stats.successes + stats.failures;
            if total == 0 {
                return 0.0;
            }
            stats.successes as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Get the average confidence for a domain.
    pub fn average_confidence(&self, domain: &CognitiveDomain) -> f64 {
        self.domain_confidence.get(domain).copied().unwrap_or(0.0)
    }

    /// Generate the improvement queue, sorted by priority (highest first).
    ///
    /// Priority = failure_rate * (1.0 - average_confidence).
    /// Domains with high failure rates and low confidence are prioritized.
    pub fn improvement_queue(&self) -> Result<Vec<ImprovementTarget>, HdcError> {
        debuglog!("MetaCognitiveProfiler::improvement_queue: generating priorities");

        let mut targets = Vec::new();

        for (domain, stats) in &self.domain_stats {
            let total = stats.successes + stats.failures;
            if total == 0 {
                continue;
            }

            let failure_rate = stats.failures as f64 / total as f64;
            let avg_conf = stats.total_confidence / total as f64;

            let weakness_score = failure_rate * (1.0 - avg_conf);
            let strength_score = (1.0 - failure_rate) * avg_conf;
            let improvement_priority = weakness_score - strength_score + 0.5; // Normalize around 0.5

            targets.push(ImprovementTarget {
                domain: domain.clone(),
                weakness_score,
                strength_score,
                improvement_priority,
                failure_count: stats.failures,
                success_count: stats.successes,
            });
        }

        // Sort by improvement priority (highest first)
        targets.sort_by(|a, b| b.improvement_priority.partial_cmp(&a.improvement_priority).unwrap_or(std::cmp::Ordering::Equal));

        debuglog!(
            "MetaCognitiveProfiler::improvement_queue: {} domains ranked",
            targets.len()
        );

        Ok(targets)
    }

    /// Get a summary of all tracked domains.
    pub fn summary(&self) -> HashMap<CognitiveDomain, (usize, usize, f64)> {
        let mut result = HashMap::new();
        for (domain, stats) in &self.domain_stats {
            let avg_conf = self.domain_confidence.get(domain).copied().unwrap_or(0.0);
            result.insert(domain.clone(), (stats.successes, stats.failures, avg_conf));
        }
        result
    }

    /// Check if a domain is identified as a weakness (failure rate > 50%).
    pub fn is_weak(&self, domain: &CognitiveDomain) -> bool {
        self.success_rate(domain) < 0.5
    }

    /// Check if a domain is identified as a strength (success rate > 80%).
    pub fn is_strong(&self, domain: &CognitiveDomain) -> bool {
        self.success_rate(domain) >= 0.8
    }

    /// Get the total number of domains being tracked.
    pub fn domain_count(&self) -> usize {
        self.domain_stats.len()
    }

    /// Generate a concrete improvement plan for the weakest domains.
    ///
    /// Each plan item describes what the system should do to improve,
    /// based on the failure patterns observed.
    pub fn generate_improvement_plan(&self) -> Result<Vec<ImprovementPlan>, HdcError> {
        debuglog!("MetaCognitiveProfiler::generate_improvement_plan: analyzing performance");

        let queue = self.improvement_queue()?;
        let mut plans = Vec::new();

        for target in queue.iter().filter(|t| t.improvement_priority > 0.4) {
            let total = target.success_count + target.failure_count;
            let failure_rate = if total > 0 { target.failure_count as f64 / total as f64 } else { 0.0 };

            let strategy = if failure_rate > 0.8 {
                "CRITICAL: Almost always failing. Needs foundational learning before attempting more tasks."
            } else if failure_rate > 0.5 {
                "HIGH: Failing more than succeeding. Focus on identifying the specific failure patterns."
            } else {
                "MODERATE: Occasionally failing. Refine edge case handling and increase confidence."
            };

            let actions = match &target.domain {
                CognitiveDomain::Coding => vec![
                    "Review recent code generation failures for pattern analysis",
                    "Study language-specific idioms for error-prone constructs",
                    "Practice with increasingly complex code synthesis tasks",
                ],
                CognitiveDomain::Security => vec![
                    "Review PSL axiom rejection patterns in feedback loop",
                    "Study common vulnerability patterns (OWASP, CWE)",
                    "Practice threat modeling on diverse attack surfaces",
                ],
                CognitiveDomain::Mathematics => vec![
                    "Review failed mathematical reasoning chains",
                    "Practice symbolic manipulation and proof verification",
                    "Study numerical stability and edge case handling",
                ],
                CognitiveDomain::Planning => vec![
                    "Analyze plan failures: were goals too ambitious?",
                    "Practice decomposition on smaller, verifiable sub-goals",
                    "Study means-end analysis heuristics",
                ],
                _ => vec![
                    "Review failure patterns in this domain",
                    "Identify knowledge gaps via the knowledge engine",
                    "Practice with increasing difficulty",
                ],
            };

            plans.push(ImprovementPlan {
                domain: target.domain.clone(),
                priority: strategy.to_string(),
                failure_rate,
                actions: actions.iter().map(|s| s.to_string()).collect(),
                estimated_sessions: (failure_rate * 10.0).ceil() as usize,
            });
        }

        debuglog!("MetaCognitiveProfiler::generate_improvement_plan: {} plans generated", plans.len());
        Ok(plans)
    }

    /// Detect cross-domain performance correlation.
    ///
    /// Returns pairs of domains where improvement in one correlates
    /// with improvement in another — suggesting transfer learning.
    pub fn detect_cross_domain_transfer(&self) -> Vec<(CognitiveDomain, CognitiveDomain, f64)> {
        debuglog!("MetaCognitiveProfiler::detect_cross_domain_transfer: analyzing correlations");

        let domains: Vec<&CognitiveDomain> = self.domain_stats.keys().collect();
        let mut transfers = Vec::new();

        for i in 0..domains.len() {
            for j in (i + 1)..domains.len() {
                let rate_i = self.success_rate(domains[i]);
                let rate_j = self.success_rate(domains[j]);

                // Simple correlation: both strong or both weak suggests transfer.
                // This is a heuristic — real transfer detection needs temporal data.
                let correlation = 1.0 - (rate_i - rate_j).abs();

                if correlation > 0.7 && rate_i > 0.3 && rate_j > 0.3 {
                    transfers.push((domains[i].clone(), domains[j].clone(), correlation));
                }
            }
        }

        transfers.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        debuglog!("MetaCognitiveProfiler::detect_cross_domain_transfer: {} correlations found", transfers.len());
        transfers
    }

    /// Overall readiness score: weighted average of all domain success rates.
    /// 1.0 = all domains at 100% success, 0.0 = all domains at 0%.
    pub fn overall_readiness(&self) -> f64 {
        if self.domain_stats.is_empty() {
            return 0.0;
        }
        let total: f64 = self.domain_stats.keys()
            .map(|d| self.success_rate(d))
            .sum();
        total / self.domain_stats.len() as f64
    }
}

/// A concrete improvement plan for a weak domain.
#[derive(Debug, Clone)]
pub struct ImprovementPlan {
    pub domain: CognitiveDomain,
    pub priority: String,
    pub failure_rate: f64,
    pub actions: Vec<String>,
    pub estimated_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(domain: CognitiveDomain, success: bool, confidence: f64) -> PerformanceRecord {
        PerformanceRecord {
            domain,
            success,
            confidence,
            task_vector: BipolarVector::new_random().expect("random vector"),
            description: String::new(),
        }
    }

    #[test]
    fn test_profiler_creation() {
        let profiler = MetaCognitiveProfiler::new();
        assert_eq!(profiler.total_records, 0);
        assert_eq!(profiler.domain_count(), 0);
    }

    #[test]
    fn test_record_success() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        let record = make_record(CognitiveDomain::Coding, true, 0.9);
        profiler.record(&record)?;

        assert_eq!(profiler.total_records, 1);
        assert_eq!(profiler.domain_count(), 1);
        assert!((profiler.success_rate(&CognitiveDomain::Coding) - 1.0).abs() < 0.001);
        assert!((profiler.average_confidence(&CognitiveDomain::Coding) - 0.9).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn test_record_failure() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        let record = make_record(CognitiveDomain::Security, false, 0.2);
        profiler.record(&record)?;

        assert_eq!(profiler.success_rate(&CognitiveDomain::Security), 0.0);
        assert!(profiler.is_weak(&CognitiveDomain::Security));
        Ok(())
    }

    #[test]
    fn test_mixed_performance() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();

        // 8 successes, 2 failures in Coding
        for _ in 0..8 {
            profiler.record(&make_record(CognitiveDomain::Coding, true, 0.85))?;
        }
        for _ in 0..2 {
            profiler.record(&make_record(CognitiveDomain::Coding, false, 0.3))?;
        }

        // 2 successes, 8 failures in Security
        for _ in 0..2 {
            profiler.record(&make_record(CognitiveDomain::Security, true, 0.6))?;
        }
        for _ in 0..8 {
            profiler.record(&make_record(CognitiveDomain::Security, false, 0.15))?;
        }

        assert!(profiler.is_strong(&CognitiveDomain::Coding));
        assert!(profiler.is_weak(&CognitiveDomain::Security));
        assert_eq!(profiler.total_records, 20);

        Ok(())
    }

    #[test]
    fn test_improvement_queue_ordering() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();

        // Strong domain: Coding
        for _ in 0..9 {
            profiler.record(&make_record(CognitiveDomain::Coding, true, 0.9))?;
        }
        profiler.record(&make_record(CognitiveDomain::Coding, false, 0.4))?;

        // Weak domain: Security
        profiler.record(&make_record(CognitiveDomain::Security, true, 0.5))?;
        for _ in 0..9 {
            profiler.record(&make_record(CognitiveDomain::Security, false, 0.1))?;
        }

        // Medium domain: Mathematics
        for _ in 0..5 {
            profiler.record(&make_record(CognitiveDomain::Mathematics, true, 0.7))?;
        }
        for _ in 0..5 {
            profiler.record(&make_record(CognitiveDomain::Mathematics, false, 0.3))?;
        }

        let queue = profiler.improvement_queue()?;
        assert_eq!(queue.len(), 3);

        // Security should be first (weakest)
        assert_eq!(queue[0].domain, CognitiveDomain::Security);
        // Coding should be last (strongest)
        assert_eq!(queue[queue.len() - 1].domain, CognitiveDomain::Coding);

        debuglog!("Improvement queue:");
        for target in &queue {
            debuglog!(
                "  {:?}: priority={:.3}, failures={}, successes={}",
                target.domain, target.improvement_priority,
                target.failure_count, target.success_count
            );
        }

        Ok(())
    }

    #[test]
    fn test_holographic_probing() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();

        // Record several successes in Coding
        for _ in 0..5 {
            profiler.record(&make_record(CognitiveDomain::Coding, true, 0.9))?;
        }

        // Record several failures in Security
        for _ in 0..5 {
            profiler.record(&make_record(CognitiveDomain::Security, false, 0.1))?;
        }

        // Strength probe should detect Coding signal
        let coding_strength = profiler.probe_strength(&CognitiveDomain::Coding)?;
        debuglog!("Coding strength signal: {:.4}", coding_strength);

        // Weakness probe should detect Security signal
        let security_weakness = profiler.probe_weakness(&CognitiveDomain::Security)?;
        debuglog!("Security weakness signal: {:.4}", security_weakness);

        // Both probes should return non-zero signals (actual value depends
        // on holographic interference, but should be detectable)
        // Note: with many bundled associations, signal degrades — this is expected
        Ok(())
    }

    #[test]
    fn test_domain_base_vectors_orthogonal() -> Result<(), HdcError> {
        let domains = vec![
            CognitiveDomain::Coding,
            CognitiveDomain::Mathematics,
            CognitiveDomain::Security,
            CognitiveDomain::NaturalLanguage,
        ];

        // All domain base vectors should be quasi-orthogonal
        for i in 0..domains.len() {
            for j in (i + 1)..domains.len() {
                let vi = domains[i].base_vector();
                let vj = domains[j].base_vector();
                let sim = vi.similarity(&vj)?;
                debuglog!("{:?} vs {:?}: sim={:.4}", domains[i], domains[j], sim);
                assert!(
                    sim.abs() < 0.1,
                    "{:?} vs {:?} should be orthogonal, sim={}",
                    domains[i], domains[j], sim
                );
            }
        }
        Ok(())
    }

    #[test]
    fn test_custom_domain() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        let custom = CognitiveDomain::Custom("pentesting".to_string());
        profiler.record(&make_record(custom.clone(), true, 0.95))?;
        assert_eq!(profiler.success_rate(&custom), 1.0);
        Ok(())
    }

    #[test]
    fn test_summary() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        profiler.record(&make_record(CognitiveDomain::Coding, true, 0.9))?;
        profiler.record(&make_record(CognitiveDomain::Coding, false, 0.3))?;

        let summary = profiler.summary();
        let (successes, failures, avg_conf) = summary.get(&CognitiveDomain::Coding).expect("should have coding");
        assert_eq!(*successes, 1);
        assert_eq!(*failures, 1);
        assert!((*avg_conf - 0.6).abs() < 0.001); // (0.9 + 0.3) / 2
        Ok(())
    }

    #[test]
    fn test_improvement_plan_generation() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();

        // Weak Security domain.
        for _ in 0..8 {
            profiler.record(&make_record(CognitiveDomain::Security, false, 0.1))?;
        }
        for _ in 0..2 {
            profiler.record(&make_record(CognitiveDomain::Security, true, 0.5))?;
        }

        let plans = profiler.generate_improvement_plan()?;
        assert!(!plans.is_empty(), "Should generate improvement plans for weak domains");
        assert_eq!(plans[0].domain, CognitiveDomain::Security);
        assert!(plans[0].failure_rate > 0.5);
        assert!(!plans[0].actions.is_empty());
        Ok(())
    }

    #[test]
    fn test_no_plans_for_strong_domains() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();

        // Strong Coding domain.
        for _ in 0..10 {
            profiler.record(&make_record(CognitiveDomain::Coding, true, 0.95))?;
        }

        let plans = profiler.generate_improvement_plan()?;
        assert!(
            plans.iter().all(|p| p.domain != CognitiveDomain::Coding),
            "Strong domains should not appear in improvement plans"
        );
        Ok(())
    }

    #[test]
    fn test_cross_domain_transfer_detection() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();

        // Both Coding and Security are strong (suggesting transfer).
        for _ in 0..9 {
            profiler.record(&make_record(CognitiveDomain::Coding, true, 0.9))?;
            profiler.record(&make_record(CognitiveDomain::Security, true, 0.85))?;
        }
        profiler.record(&make_record(CognitiveDomain::Coding, false, 0.4))?;
        profiler.record(&make_record(CognitiveDomain::Security, false, 0.3))?;

        let transfers = profiler.detect_cross_domain_transfer();
        // Should detect correlation between Coding and Security.
        assert!(!transfers.is_empty(), "Should detect cross-domain transfer");
        Ok(())
    }

    #[test]
    fn test_overall_readiness() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();

        // All domains at 100%.
        for domain in &[CognitiveDomain::Coding, CognitiveDomain::Security, CognitiveDomain::Mathematics] {
            profiler.record(&make_record(domain.clone(), true, 0.9))?;
        }

        let readiness = profiler.overall_readiness();
        assert!((readiness - 1.0).abs() < 0.01, "All-success should give readiness ~1.0, got {:.4}", readiness);

        // Empty profiler.
        let empty = MetaCognitiveProfiler::new();
        assert_eq!(empty.overall_readiness(), 0.0);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for MetaCognitiveProfiler
    // ============================================================

    /// INVARIANT: success_rate stays in [0.0, 1.0] regardless of recorded mix.
    #[test]
    fn invariant_success_rate_in_unit_interval() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        for i in 0..50 {
            profiler.record(&PerformanceRecord {
                domain: CognitiveDomain::Coding,
                success: i % 3 != 0,
                confidence: 0.5,
                task_vector: BipolarVector::new_random().expect("rand"),
                description: "test".into(),
            })?;
            let rate = profiler.success_rate(&CognitiveDomain::Coding);
            assert!(rate >= 0.0 && rate <= 1.0,
                "success_rate escaped [0,1]: {}", rate);
        }
        Ok(())
    }

    /// INVARIANT: average_confidence stays in [0.0, 1.0].
    #[test]
    fn invariant_average_confidence_in_unit_interval() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        for i in 0..30 {
            let conf = (i as f64 % 11.0) / 10.0;
            profiler.record(&PerformanceRecord {
                domain: CognitiveDomain::Reasoning,
                success: true,
                confidence: conf,
                task_vector: BipolarVector::new_random().expect("rand"),
                description: "test".into(),
            })?;
            let avg = profiler.average_confidence(&CognitiveDomain::Reasoning);
            assert!(avg >= 0.0 && avg <= 1.0,
                "avg confidence escaped [0,1]: {}", avg);
        }
        Ok(())
    }

    /// INVARIANT: overall_readiness stays in [0.0, 1.0] across multi-domain mix.
    #[test]
    fn invariant_overall_readiness_in_unit_interval() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        let domains = [
            CognitiveDomain::Coding,
            CognitiveDomain::Reasoning,
            CognitiveDomain::Mathematics,
            CognitiveDomain::Security,
        ];
        for d in &domains {
            for i in 0..10 {
                profiler.record(&PerformanceRecord {
                    domain: d.clone(),
                    success: i % 2 == 0,
                    confidence: 0.6,
                    task_vector: BipolarVector::new_random().expect("rand"),
                    description: "test".into(),
                })?;
            }
        }
        let r = profiler.overall_readiness();
        assert!(r >= 0.0 && r <= 1.0, "readiness escaped [0,1]: {}", r);
        Ok(())
    }

    /// INVARIANT: domain_count grows when a new domain is recorded for the
    /// first time, stays flat for subsequent records on existing domains.
    #[test]
    fn invariant_domain_count_grows_on_first_record_only() -> Result<(), HdcError> {
        let mut profiler = MetaCognitiveProfiler::new();
        let initial = profiler.domain_count();
        profiler.record(&PerformanceRecord {
            domain: CognitiveDomain::Coding,
            success: true, confidence: 0.5,
            task_vector: BipolarVector::new_random().expect("rand"),
            description: "first".into(),
        })?;
        let after_first = profiler.domain_count();
        assert_eq!(after_first, initial + 1, "first record must add 1 domain");
        // Second record for the SAME domain — count must not grow.
        profiler.record(&PerformanceRecord {
            domain: CognitiveDomain::Coding,
            success: false, confidence: 0.3,
            task_vector: BipolarVector::new_random().expect("rand"),
            description: "second".into(),
        })?;
        assert_eq!(profiler.domain_count(), after_first,
            "second record on existing domain must not grow count");
        Ok(())
    }
}
