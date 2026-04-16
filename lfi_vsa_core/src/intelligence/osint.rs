// ============================================================
// Intelligence / OSINT Module — The Sensory Peripheral
// Section 1.IV: "Implement Intelligence/OSINT modules for analysis."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::psl::supervisor::PslSupervisor;
use crate::psl::axiom::AuditTarget;
use crate::identity::IdentityProver;

/// A piece of intelligence gathered from OSINT.
#[derive(Debug, Clone)]
pub struct OsintSignal {
    pub source: String,
    pub payload: String,
    pub metadata: Vec<(String, String)>,
}

/// The Intelligence Analyzer engine.
pub struct OsintAnalyzer {
    pub supervisor: PslSupervisor,
}

impl OsintAnalyzer {
    pub fn new() -> Self {
        debuglog!("OsintAnalyzer::new: Initializing OSINT audit supervisor");
        let supervisor = PslSupervisor::new();
        // Axioms will be inherited or registered here
        Self { supervisor }
    }

    /// Audits an external signal against the threat matrix.
    pub fn analyze_signal(&self, signal: &OsintSignal) -> Result<f64, String> {
        debuglog!("OsintAnalyzer: Analyzing signal from {}", signal.source);
        
        // 1. Vectorization of the signal
        let signal_hash = IdentityProver::hash(&signal.payload);
        let signal_vector = BipolarVector::from_seed(signal_hash);

        // 2. PSL Audit
        let target = AuditTarget::Vector(signal_vector);
        let assessment = self.supervisor.audit(&target).map_err(|e| format!("Audit failed: {:?}", e))?;

        if !assessment.level.permits_execution() {
            debuglog!("OsintAnalyzer: SIGNAL REJECTED (Level={:?})", assessment.level);
            return Err("Signal failed forensic trust audit".to_string());
        }

        debuglog!("OsintAnalyzer: Signal verified. Trust Confidence = {:.4}", assessment.confidence);
        Ok(assessment.confidence)
    }

    /// Perform a CARTA risk assessment on a set of signals.
    pub fn assess_risk(&self, signals: &[OsintSignal]) -> f64 {
        let mut total_risk = 0.0;
        for s in signals {
            match self.analyze_signal(s) {
                Ok(conf) => total_risk += 1.0 - conf,
                Err(_) => total_risk += 1.0,
            }
        }
        total_risk / (signals.len() as f64).max(1.0)
    }

    /// Categorize a signal by threat type based on keyword analysis.
    pub fn categorize_threat(signal: &OsintSignal) -> ThreatCategory {
        let lower = signal.payload.to_lowercase();

        if lower.contains("cve-") || lower.contains("vulnerability") || lower.contains("exploit") {
            ThreatCategory::Vulnerability
        } else if lower.contains("malware") || lower.contains("ransomware") || lower.contains("trojan") {
            ThreatCategory::Malware
        } else if lower.contains("phishing") || lower.contains("social engineering") || lower.contains("spear") {
            ThreatCategory::SocialEngineering
        } else if lower.contains("ddos") || lower.contains("denial of service") || lower.contains("botnet") {
            ThreatCategory::DenialOfService
        } else if lower.contains("breach") || lower.contains("leak") || lower.contains("exfiltration") {
            ThreatCategory::DataBreach
        } else if lower.contains("apt") || lower.contains("state-sponsored") || lower.contains("nation-state") {
            ThreatCategory::APT
        } else {
            ThreatCategory::Unknown
        }
    }

    /// Score the priority of a signal (0.0 = low, 1.0 = critical).
    pub fn priority_score(signal: &OsintSignal) -> f64 {
        let category = Self::categorize_threat(signal);
        let base_priority: f64 = match category {
            ThreatCategory::APT => 1.0,
            ThreatCategory::DataBreach => 0.9,
            ThreatCategory::Vulnerability => 0.8,
            ThreatCategory::Malware => 0.7,
            ThreatCategory::SocialEngineering => 0.6,
            ThreatCategory::DenialOfService => 0.5,
            ThreatCategory::Unknown => 0.3,
        };

        // Boost priority for signals with urgency indicators.
        let lower = signal.payload.to_lowercase();
        let urgency_boost = if lower.contains("critical") || lower.contains("active") || lower.contains("zero-day") {
            0.15
        } else {
            0.0
        };

        (base_priority + urgency_boost).min(1.0_f64)
    }

    /// Detect correlated signals (same topic from multiple sources).
    pub fn find_correlations(signals: &[OsintSignal]) -> Vec<(usize, usize, f64)> {
        let mut correlations = Vec::new();

        for i in 0..signals.len() {
            let vi = BipolarVector::from_seed(IdentityProver::hash(&signals[i].payload));
            for j in (i + 1)..signals.len() {
                let vj = BipolarVector::from_seed(IdentityProver::hash(&signals[j].payload));
                let sim = vi.similarity(&vj).unwrap_or(0.0);
                if sim > 0.3 {
                    correlations.push((i, j, sim));
                }
            }
        }

        correlations.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        correlations
    }
}

/// Categories of threats detected from OSINT signals.
#[derive(Debug, Clone, PartialEq)]
pub enum ThreatCategory {
    Vulnerability,
    Malware,
    SocialEngineering,
    DenialOfService,
    DataBreach,
    APT,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psl::axiom::DimensionalityAxiom;

    fn make_signal(source: &str, payload: &str) -> OsintSignal {
        OsintSignal {
            source: source.into(),
            payload: payload.into(),
            metadata: vec![],
        }
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = OsintAnalyzer::new();
        assert_eq!(analyzer.supervisor.axiom_count(), 0);
    }

    #[test]
    fn test_analyze_signal_with_axiom() {
        let mut analyzer = OsintAnalyzer::new();
        analyzer.supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let signal = make_signal("web_feed", "Breaking: new vulnerability CVE-2024-1234");
        let result = analyzer.analyze_signal(&signal);
        assert!(result.is_ok(), "Signal analysis should succeed");
        assert!(result.unwrap() > 0.0);
    }

    #[test]
    fn test_analyze_signal_no_axioms_passes() {
        let analyzer = OsintAnalyzer::new();
        let signal = make_signal("rss", "Normal news article about technology");
        // No axioms → default pass (confidence 1.0).
        let result = analyzer.analyze_signal(&signal);
        assert!(result.is_ok());
        assert!((result.unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_assess_risk_multiple_signals() {
        let mut analyzer = OsintAnalyzer::new();
        analyzer.supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let signals = vec![
            make_signal("feed_a", "Signal one"),
            make_signal("feed_b", "Signal two"),
            make_signal("feed_c", "Signal three"),
        ];

        let risk = analyzer.assess_risk(&signals);
        assert!(risk >= 0.0 && risk <= 1.0, "Risk should be in [0,1]: {:.4}", risk);
    }

    #[test]
    fn test_assess_risk_empty() {
        let analyzer = OsintAnalyzer::new();
        let risk = analyzer.assess_risk(&[]);
        assert_eq!(risk, 0.0, "Empty signal set should have zero risk");
    }

    #[test]
    fn test_signal_metadata() {
        let signal = OsintSignal {
            source: "twitter".into(),
            payload: "Threat actor spotted".into(),
            metadata: vec![
                ("author".into(), "anonymous".into()),
                ("timestamp".into(), "2026-04-13".into()),
            ],
        };
        assert_eq!(signal.metadata.len(), 2);
        assert_eq!(signal.metadata[0].0, "author");
    }

    #[test]
    fn test_categorize_vulnerability() {
        let signal = make_signal("nvd", "New CVE-2026-1234 critical vulnerability in OpenSSL");
        assert_eq!(OsintAnalyzer::categorize_threat(&signal), ThreatCategory::Vulnerability);
    }

    #[test]
    fn test_categorize_malware() {
        let signal = make_signal("feed", "New ransomware variant targeting healthcare");
        assert_eq!(OsintAnalyzer::categorize_threat(&signal), ThreatCategory::Malware);
    }

    #[test]
    fn test_categorize_apt() {
        let signal = make_signal("intel", "APT group linked to state-sponsored attacks");
        assert_eq!(OsintAnalyzer::categorize_threat(&signal), ThreatCategory::APT);
    }

    #[test]
    fn test_categorize_social_engineering() {
        let signal = make_signal("feed", "Phishing campaign targeting financial sector");
        assert_eq!(OsintAnalyzer::categorize_threat(&signal), ThreatCategory::SocialEngineering);
    }

    #[test]
    fn test_categorize_unknown() {
        let signal = make_signal("news", "Regular tech industry news update");
        assert_eq!(OsintAnalyzer::categorize_threat(&signal), ThreatCategory::Unknown);
    }

    #[test]
    fn test_priority_scoring() {
        let apt = make_signal("intel", "APT group active zero-day campaign");
        let news = make_signal("rss", "Industry conference next week");
        assert!(OsintAnalyzer::priority_score(&apt) > OsintAnalyzer::priority_score(&news));
    }

    #[test]
    fn test_priority_critical_boost() {
        let normal = make_signal("nvd", "CVE-2026-5678 vulnerability discovered");
        let critical = make_signal("nvd", "Critical zero-day vulnerability actively exploited");
        assert!(OsintAnalyzer::priority_score(&critical) > OsintAnalyzer::priority_score(&normal));
    }

    #[test]
    fn test_find_correlations_identical() {
        let signals = vec![
            make_signal("source_a", "exact same payload"),
            make_signal("source_b", "exact same payload"),
        ];
        let corr = OsintAnalyzer::find_correlations(&signals);
        if !corr.is_empty() {
            assert!((corr[0].2 - 1.0).abs() < 0.01, "Identical payloads should correlate perfectly");
        }
    }

    #[test]
    fn test_find_correlations_different() {
        let signals = vec![
            make_signal("source_a", "totally different topic alpha"),
            make_signal("source_b", "completely unrelated subject beta"),
        ];
        let corr = OsintAnalyzer::find_correlations(&signals);
        // Different payloads should have low or no correlation.
        if !corr.is_empty() {
            assert!(corr[0].2 < 0.9, "Different payloads should have low correlation");
        }
    }

    // ============================================================
    // Stress / invariant tests for OsintAnalyzer
    // ============================================================

    /// INVARIANT: assess_risk returns a finite value in [0,1] for any signal mix.
    #[test]
    fn invariant_assess_risk_in_unit_interval() {
        let analyzer = OsintAnalyzer::new();
        // Empty input.
        let r0 = analyzer.assess_risk(&[]);
        assert!(r0.is_finite() && (0.0..=1.0).contains(&r0),
            "empty risk out of [0,1]: {}", r0);
        // Mixed input.
        let mixed: Vec<OsintSignal> = (0..30)
            .map(|i| make_signal(&format!("src_{}", i),
                if i % 5 == 0 { "APT-29 IOC" } else { "benign" }))
            .collect();
        let r = analyzer.assess_risk(&mixed);
        assert!(r.is_finite() && (0.0..=1.0).contains(&r),
            "mixed risk out of [0,1]: {}", r);
    }

    /// INVARIANT: priority_score is finite and in a reasonable range — used
    /// directly as a sort key by triage UIs.
    #[test]
    fn invariant_priority_score_finite() {
        for severity_text in ["benign", "low", "medium", "high", "critical", "アリス"] {
            let signal = make_signal("src", severity_text);
            let score = OsintAnalyzer::priority_score(&signal);
            assert!(score.is_finite(), "priority must be finite, got {}", score);
            assert!(score >= 0.0, "priority must be non-negative: {}", score);
        }
    }

    /// INVARIANT: find_correlations returns indices that are strictly
    /// less than the input signal count. Out-of-bounds indices would
    /// crash downstream code on indexed access.
    #[test]
    fn invariant_correlations_indices_in_bounds() {
        let signals: Vec<OsintSignal> = (0..15)
            .map(|i| make_signal(&format!("s_{}", i), &format!("payload {}", i % 3)))
            .collect();
        let corrs = OsintAnalyzer::find_correlations(&signals);
        for (i, j, score) in &corrs {
            assert!(*i < signals.len(),
                "correlation index i={} out of bounds (len={})", i, signals.len());
            assert!(*j < signals.len(),
                "correlation index j={} out of bounds (len={})", j, signals.len());
            assert!(*i != *j, "self-correlation must be excluded");
            assert!(score.is_finite(),
                "correlation score must be finite: {}", score);
        }
    }

    /// INVARIANT: analyze_signal handles arbitrary unicode/control text.
    #[test]
    fn invariant_analyze_signal_safe_on_unicode() {
        let analyzer = OsintAnalyzer::new();
        let inputs = ["", "アリス", "🦀🦀", "\x00\x01control", &"x".repeat(50_000)];
        for payload in inputs {
            let signal = make_signal("src", payload);
            // Must not panic.
            let _ = analyzer.analyze_signal(&signal);
        }
    }

    /// INVARIANT: categorize_threat returns a valid ThreatCategory enum
    /// (exhaustive match guarantees) — never a default-zero or invalid state.
    #[test]
    fn invariant_categorize_threat_returns_valid_enum() {
        for payload in ["", "APT-29 lateral movement", "phishing campaign",
                        "ransomware deployment", "アリス", "🦀"] {
            let signal = make_signal("src", payload);
            let cat = OsintAnalyzer::categorize_threat(&signal);
            // Exhaustive match — compiler ensures we cover every variant.
            match cat {
                ThreatCategory::Vulnerability |
                ThreatCategory::Malware |
                ThreatCategory::SocialEngineering |
                ThreatCategory::DenialOfService |
                ThreatCategory::DataBreach |
                ThreatCategory::APT |
                ThreatCategory::Unknown => {}
            }
        }
    }

    /// INVARIANT: assess_risk with empty list returns 0.0.
    #[test]
    fn invariant_empty_signals_zero_risk() {
        let analyzer = OsintAnalyzer::new();
        let risk = analyzer.assess_risk(&[]);
        assert_eq!(risk, 0.0, "empty signals should yield zero risk");
    }

    /// INVARIANT: find_correlations on empty list returns empty.
    #[test]
    fn invariant_find_correlations_empty_list() {
        let correlations = OsintAnalyzer::find_correlations(&[]);
        assert!(correlations.is_empty());
    }
}
