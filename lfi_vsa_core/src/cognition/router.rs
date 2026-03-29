// NODE 023: VSA Semantic Router (Mobile Optimized)
// STATUS: ALPHA - Material Gating Active
// PROTOCOL: Kinetic-Insight / Subspace-Routing

use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use tracing::{info, debug};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelligenceTier {
    Pulse,    // BitNet b1.58 (Detection)
    Bridge,   // LFM (Triage)
    BigBrain, // MoE (Resolution)
}

pub struct SemanticRouter {
    /// Anchor for routine conversational/tactical tasks.
    pub tactical_anchor: HyperMemory,
    /// Anchor for critical structural vulnerabilities and dominance plays.
    pub strategic_anchor: HyperMemory,
    pub escalation_threshold: f64,
}

impl SemanticRouter {
    pub fn new() -> Self {
        Self {
            tactical_anchor: HyperMemory::from_string("TACTICAL_CLI_EXECUTION_ROUTINE_TASK", DIM_PROLETARIAT),
            strategic_anchor: HyperMemory::from_string("STRATEGIC_DOMINANCE_STRUCTURAL_VULNERABILITY_LEVERAGE", DIM_PROLETARIAT),
            escalation_threshold: 0.85,
        }
    }

    /// ROUTE: Performs a physical similarity check to gate BigBrain activation.
    pub fn route_intent(&self, input_vector: &HyperMemory) -> IntelligenceTier {
        let strategic_sim = input_vector.similarity(&self.strategic_anchor);
        let tactical_sim = input_vector.similarity(&self.tactical_anchor);
        
        debug!("// DEBUG: VSA Routing - Strategic: {:.4}, Tactical: {:.4}", strategic_sim, tactical_sim);

        if strategic_sim >= self.escalation_threshold {
            info!("// AUDIT: KINETIC INSIGHT. Strategic subspace aligned. Escalating to BigBrain.");
            IntelligenceTier::BigBrain
        } else if tactical_sim >= 0.6 {
            IntelligenceTier::Bridge
        } else {
            IntelligenceTier::Pulse
        }
    }
}
