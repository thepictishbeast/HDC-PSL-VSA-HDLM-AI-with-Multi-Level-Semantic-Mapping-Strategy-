// NODE 023: VSA Semantic Router (Mobile Optimized)
// STATUS: ALPHA - Material Gating Active
// PROTOCOL: Kinetic-Insight / Subspace-Routing
//
// ARCHITECTURE:
//   Routes input to the appropriate intelligence tier based on
//   VSA similarity to semantic anchors. The goal is to use the
//   smallest model that can handle the task — saving compute
//   for truly complex problems.
//
//   Tiers:
//     Pulse    → BitNet b1.58 (fast detection, < 1ms)
//     Bridge   → LFM local model (triage, ~10ms)
//     BigBrain → MoE / external model (deep resolution, ~1s)
//
//   Routing is resource-aware: if the system is under load,
//   it can downgrade tiers to maintain responsiveness.

use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use tracing::{info, debug};

/// The intelligence tier determines which model/engine handles a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelligenceTier {
    /// BitNet b1.58 — fastest, handles detection and simple classification.
    Pulse,
    /// Local Foundation Model — handles triage, moderate reasoning.
    Bridge,
    /// Mixture of Experts / external — handles deep analysis, complex reasoning.
    BigBrain,
}

impl IntelligenceTier {
    /// Relative computational cost (arbitrary units).
    pub fn cost(&self) -> f64 {
        match self {
            IntelligenceTier::Pulse => 1.0,
            IntelligenceTier::Bridge => 10.0,
            IntelligenceTier::BigBrain => 100.0,
        }
    }

    /// Human-readable description.
    pub fn description(&self) -> &str {
        match self {
            IntelligenceTier::Pulse => "BitNet fast-path (detection)",
            IntelligenceTier::Bridge => "Local Foundation Model (triage)",
            IntelligenceTier::BigBrain => "MoE deep resolution (complex reasoning)",
        }
    }
}

/// Explains why a routing decision was made.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// The selected tier.
    pub tier: IntelligenceTier,
    /// Similarity to the strategic anchor.
    pub strategic_similarity: f64,
    /// Similarity to the tactical anchor.
    pub tactical_similarity: f64,
    /// Whether the tier was downgraded due to resource constraints.
    pub downgraded: bool,
    /// Human-readable explanation.
    pub explanation: String,
}

/// Configuration for the semantic router.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Similarity threshold for BigBrain escalation.
    pub strategic_threshold: f64,
    /// Similarity threshold for Bridge activation.
    pub tactical_threshold: f64,
    /// Maximum tier allowed (for resource-constrained environments).
    /// Set to Pulse to force everything through the fast path.
    pub max_tier: IntelligenceTier,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            strategic_threshold: 0.85,
            tactical_threshold: 0.60,
            max_tier: IntelligenceTier::BigBrain,
        }
    }
}

/// The VSA Semantic Router.
pub struct SemanticRouter {
    /// Anchor for routine conversational/tactical tasks.
    pub tactical_anchor: HyperMemory,
    /// Anchor for critical structural vulnerabilities and strategic analysis.
    pub strategic_anchor: HyperMemory,
    /// Routing configuration.
    pub config: RouterConfig,
    /// Total routing decisions made.
    decisions_count: u64,
    /// Count per tier (for load analysis).
    tier_counts: [u64; 3],
}

impl SemanticRouter {
    pub fn new() -> Self {
        debuglog!("SemanticRouter::new: Initializing VSA semantic router");
        Self {
            tactical_anchor: HyperMemory::from_string("TACTICAL_CLI_EXECUTION_ROUTINE_TASK", DIM_PROLETARIAT),
            strategic_anchor: HyperMemory::from_string("STRATEGIC_DOMINANCE_STRUCTURAL_VULNERABILITY_LEVERAGE", DIM_PROLETARIAT),
            config: RouterConfig::default(),
            decisions_count: 0,
            tier_counts: [0; 3],
        }
    }

    pub fn with_config(config: RouterConfig) -> Self {
        debuglog!("SemanticRouter::with_config: Custom thresholds loaded");
        let mut router = Self::new();
        router.config = config;
        router
    }

    /// ROUTE: Determine the appropriate intelligence tier for an input.
    ///
    /// Returns just the tier (backward compatible).
    pub fn route_intent(&mut self, input_vector: &HyperMemory) -> IntelligenceTier {
        self.route_explained(input_vector).tier
    }

    /// ROUTE with full explanation of the decision.
    pub fn route_explained(&mut self, input_vector: &HyperMemory) -> RoutingDecision {
        let strategic_sim = input_vector.similarity(&self.strategic_anchor);
        let tactical_sim = input_vector.similarity(&self.tactical_anchor);

        debug!("// DEBUG: VSA Routing - Strategic: {:.4}, Tactical: {:.4}", strategic_sim, tactical_sim);

        let raw_tier = if strategic_sim >= self.config.strategic_threshold {
            info!("// AUDIT: KINETIC INSIGHT. Strategic subspace aligned. Escalating to BigBrain.");
            IntelligenceTier::BigBrain
        } else if tactical_sim >= self.config.tactical_threshold {
            IntelligenceTier::Bridge
        } else {
            IntelligenceTier::Pulse
        };

        // Apply resource cap.
        let (tier, downgraded) = self.cap_tier(raw_tier);

        let explanation = if downgraded {
            format!("Routed to {} (downgraded from {} due to resource cap). Strategic={:.4}, Tactical={:.4}",
                tier.description(), raw_tier.description(), strategic_sim, tactical_sim)
        } else {
            format!("Routed to {}. Strategic={:.4}, Tactical={:.4}",
                tier.description(), strategic_sim, tactical_sim)
        };

        self.decisions_count += 1;
        let tier_idx = match tier {
            IntelligenceTier::Pulse => 0,
            IntelligenceTier::Bridge => 1,
            IntelligenceTier::BigBrain => 2,
        };
        self.tier_counts[tier_idx] += 1;

        debuglog!("SemanticRouter::route: {} (decision #{})", explanation, self.decisions_count);

        RoutingDecision {
            tier,
            strategic_similarity: strategic_sim,
            tactical_similarity: tactical_sim,
            downgraded,
            explanation,
        }
    }

    /// Cap a tier based on the max_tier configuration.
    fn cap_tier(&self, tier: IntelligenceTier) -> (IntelligenceTier, bool) {
        let max_cost = self.config.max_tier.cost();
        if tier.cost() > max_cost {
            (self.config.max_tier, true)
        } else {
            (tier, false)
        }
    }

    /// Set the maximum allowed tier (for resource adaptation).
    pub fn set_max_tier(&mut self, max: IntelligenceTier) {
        debuglog!("SemanticRouter::set_max_tier: {:?}", max);
        self.config.max_tier = max;
    }

    /// Get routing statistics.
    pub fn stats(&self) -> RouterStats {
        RouterStats {
            total_decisions: self.decisions_count,
            pulse_count: self.tier_counts[0],
            bridge_count: self.tier_counts[1],
            bigbrain_count: self.tier_counts[2],
        }
    }

    /// Total routing decisions made.
    pub fn decision_count(&self) -> u64 {
        self.decisions_count
    }

    // Backward compat: immutable route for callers that don't need mut.
    #[allow(dead_code)]
    fn route_intent_readonly(&self, input_vector: &HyperMemory) -> IntelligenceTier {
        let strategic_sim = input_vector.similarity(&self.strategic_anchor);
        let tactical_sim = input_vector.similarity(&self.tactical_anchor);

        if strategic_sim >= self.config.strategic_threshold {
            IntelligenceTier::BigBrain
        } else if tactical_sim >= self.config.tactical_threshold {
            IntelligenceTier::Bridge
        } else {
            IntelligenceTier::Pulse
        }
    }
}

/// Routing statistics for monitoring.
#[derive(Debug, Clone)]
pub struct RouterStats {
    pub total_decisions: u64,
    pub pulse_count: u64,
    pub bridge_count: u64,
    pub bigbrain_count: u64,
}

impl RouterStats {
    /// Fraction of decisions routed to each tier.
    pub fn tier_distribution(&self) -> [f64; 3] {
        let t = self.total_decisions.max(1) as f64;
        [
            self.pulse_count as f64 / t,
            self.bridge_count as f64 / t,
            self.bigbrain_count as f64 / t,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_default_routes_to_pulse() {
        let mut router = SemanticRouter::new();
        // A random input should route to Pulse (lowest tier).
        let random = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let decision = router.route_explained(&random);
        // Most random inputs are distant from both anchors → Pulse.
        assert!(
            decision.tier == IntelligenceTier::Pulse || decision.tier == IntelligenceTier::Bridge,
            "Random input should route to Pulse or Bridge, got {:?}", decision.tier
        );
    }

    #[test]
    fn test_router_strategic_escalation() {
        let mut router = SemanticRouter::new();
        // Feed the strategic anchor itself — should escalate to BigBrain.
        let decision = router.route_explained(&router.strategic_anchor.clone());
        assert_eq!(decision.tier, IntelligenceTier::BigBrain);
        assert!(!decision.downgraded);
    }

    #[test]
    fn test_router_tactical_routing() {
        let mut router = SemanticRouter::new();
        let decision = router.route_explained(&router.tactical_anchor.clone());
        assert!(
            decision.tier == IntelligenceTier::Bridge || decision.tier == IntelligenceTier::BigBrain,
            "Tactical anchor should route to Bridge or BigBrain"
        );
    }

    #[test]
    fn test_resource_cap_downgrades() {
        let config = RouterConfig {
            max_tier: IntelligenceTier::Pulse, // Force everything to Pulse.
            ..Default::default()
        };
        let mut router = SemanticRouter::with_config(config);

        let strategic_input = router.strategic_anchor.clone();
        let decision = router.route_explained(&strategic_input);

        // Even strategic input should be capped to Pulse.
        assert_eq!(decision.tier, IntelligenceTier::Pulse);
        assert!(decision.downgraded, "Should be marked as downgraded");
    }

    #[test]
    fn test_routing_stats() {
        let mut router = SemanticRouter::new();

        for _ in 0..5 {
            let v = HyperMemory::generate_seed(DIM_PROLETARIAT);
            router.route_intent(&v);
        }

        let stats = router.stats();
        assert_eq!(stats.total_decisions, 5);
        assert_eq!(
            stats.pulse_count + stats.bridge_count + stats.bigbrain_count,
            5,
            "Tier counts should sum to total"
        );

        let dist = stats.tier_distribution();
        let sum: f64 = dist.iter().sum();
        assert!((sum - 1.0).abs() < 0.01, "Distribution should sum to 1.0");
    }

    #[test]
    fn test_tier_costs() {
        assert!(IntelligenceTier::Pulse.cost() < IntelligenceTier::Bridge.cost());
        assert!(IntelligenceTier::Bridge.cost() < IntelligenceTier::BigBrain.cost());
    }

    #[test]
    fn test_custom_thresholds() {
        let config = RouterConfig {
            strategic_threshold: 0.99, // Very high — almost nothing escalates
            tactical_threshold: 0.99,
            max_tier: IntelligenceTier::BigBrain,
        };
        let mut router = SemanticRouter::with_config(config);
        let random = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let decision = router.route_explained(&random);
        assert_eq!(decision.tier, IntelligenceTier::Pulse, "High thresholds should route everything to Pulse");
    }

    // ============================================================
    // Stress / invariant tests for SemanticRouter
    // ============================================================

    /// INVARIANT: every tier has a finite positive cost in increasing order.
    #[test]
    fn invariant_tier_costs_strictly_ordered() {
        let pulse = IntelligenceTier::Pulse.cost();
        let bridge = IntelligenceTier::Bridge.cost();
        let big = IntelligenceTier::BigBrain.cost();
        assert!(pulse.is_finite() && pulse > 0.0, "Pulse cost must be finite+positive: {}", pulse);
        assert!(bridge.is_finite() && bridge > 0.0, "Bridge cost must be finite+positive: {}", bridge);
        assert!(big.is_finite() && big > 0.0, "BigBrain cost must be finite+positive: {}", big);
        assert!(pulse < bridge && bridge < big,
            "tiers must have strictly increasing cost: {} < {} < {}", pulse, bridge, big);
    }

    /// INVARIANT: every tier has a non-empty description.
    #[test]
    fn invariant_tier_descriptions_non_empty() {
        for tier in [
            IntelligenceTier::Pulse,
            IntelligenceTier::Bridge,
            IntelligenceTier::BigBrain,
        ] {
            assert!(!tier.description().is_empty(),
                "{:?} description must not be empty", tier);
        }
    }

    /// INVARIANT: max_tier caps routing — no decision should exceed it
    /// regardless of input.
    #[test]
    fn invariant_max_tier_caps_routing() {
        for cap in [IntelligenceTier::Pulse, IntelligenceTier::Bridge, IntelligenceTier::BigBrain] {
            let config = RouterConfig {
                strategic_threshold: 0.0,  // Encourage escalation
                tactical_threshold: 0.0,
                max_tier: cap,
            };
            let mut router = SemanticRouter::with_config(config);
            for seed_offset in 0..10 {
                let vec = HyperMemory::generate_seed(DIM_PROLETARIAT + seed_offset);
                let decision = router.route_explained(&vec);
                assert!(decision.tier.cost() <= cap.cost(),
                    "routed to {:?} which exceeds cap {:?}", decision.tier, cap);
            }
        }
    }

    /// INVARIANT: RoutingDecision carries similarity scores in [-1,1] — the
    /// cosine-similarity range. Downstream uses these for confidence display.
    #[test]
    fn invariant_routing_decision_similarities_in_cosine_range() {
        let mut router = SemanticRouter::new();
        for _ in 0..20 {
            let vec = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let decision = router.route_explained(&vec);
            assert!(decision.strategic_similarity.is_finite()
                && decision.strategic_similarity >= -1.0 - 1e-6
                && decision.strategic_similarity <= 1.0 + 1e-6,
                "strategic_similarity out of [-1,1]: {}", decision.strategic_similarity);
            assert!(decision.tactical_similarity.is_finite()
                && decision.tactical_similarity >= -1.0 - 1e-6
                && decision.tactical_similarity <= 1.0 + 1e-6,
                "tactical_similarity out of [-1,1]: {}", decision.tactical_similarity);
        }
    }

    /// INVARIANT: RoutingDecision.explanation is never empty — every
    /// decision must be explainable for audit.
    #[test]
    fn invariant_explanation_non_empty() {
        let mut router = SemanticRouter::new();
        for _ in 0..5 {
            let vec = HyperMemory::generate_seed(DIM_PROLETARIAT);
            let decision = router.route_explained(&vec);
            assert!(!decision.explanation.is_empty(),
                "decision must carry non-empty explanation");
        }
    }
}
