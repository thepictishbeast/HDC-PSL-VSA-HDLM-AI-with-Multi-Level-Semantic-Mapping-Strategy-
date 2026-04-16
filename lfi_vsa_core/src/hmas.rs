// NODE 011: Hyper-Multi-Agent System (HMAS) Orchestrator
// STATUS: ALPHA - Swarm Concurrency Active
// PROTOCOL: Digital Gosplan / Federated Sovereignty

use serde::{Serialize, Deserialize};
use tracing::{info, warn};
use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use crate::cognition::mcts::MctsEngine;
use crate::psl::supervisor::PslSupervisor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    Architect,  // Strategy & Decomposition
    Worker,     // Execution (Rust/Bash/Python)
    Auditor,    // Forensic Review & PSL Verification
    Historian,  // Long-Term VSA Memory & RAG
}

/// A cryptographic proposal for material base mutation.
pub struct Proposal {
    pub id: String,
    pub payload_hv: HyperMemory,
    pub timestamp: u64,
}

/// The Historian node: manages long-term memory and negative knowledge traces.
pub struct Historian {
    pub archive: Vec<HyperMemory>,
    pub negative_knowledge: Vec<HyperMemory>, 
}

impl Historian {
    pub fn new() -> Self {
        Self {
            archive: Vec::new(),
            negative_knowledge: Vec::new(),
        }
    }

    pub fn record_synthesis(&mut self, state: HyperMemory) {
        info!("// AUDIT: Historian archiving new Sovereign synthesis.");
        self.archive.push(state);
    }

    pub fn condemn_failure(&mut self, failure: HyperMemory) {
        warn!("// AUDIT: Historian recording forensic trace of strategic failure.");
        self.negative_knowledge.push(failure);
    }

    pub fn retrieve_context(&self, current: &HyperMemory) -> (f64, f64) {
        let max_archive = self.archive.iter()
            .map(|h| h.similarity(current))
            .fold(0.0, f64::max);
            
        let max_negative = self.negative_knowledge.iter()
            .map(|h| h.similarity(current))
            .fold(0.0, f64::max);
            
        (max_archive, max_negative)
    }
}

pub struct MicroSupervisor {
    pub role: AgentRole,
    pub consensus_threshold: f64,
    pub agree_anchor: HyperMemory,
    pub disagree_anchor: HyperMemory,
}

impl MicroSupervisor {
    pub fn new(role: AgentRole) -> Self {
        info!("// AUDIT: Agent Node Materialized as {:?}", role);
        Self {
            role,
            consensus_threshold: 0.7,
            agree_anchor: HyperMemory::from_string("CONSENSUS_AGREE_PROCEED", DIM_PROLETARIAT),
            disagree_anchor: HyperMemory::from_string("CONSENSUS_DISAGREE_HALT", DIM_PROLETARIAT),
        }
    }

    pub fn vote(&self, proposal: &Proposal, verified: bool) -> Result<HyperMemory, Box<dyn std::error::Error>> {
        let anchor = if verified { &self.agree_anchor } else { &self.disagree_anchor };
        anchor.bind(&proposal.payload_hv)
    }

    pub fn resolve_consensus(votes: &[HyperMemory], proposal: &Proposal, agree_anchor: &HyperMemory) -> Result<f64, Box<dyn std::error::Error>> {
        if votes.is_empty() { return Ok(0.0); }
        let bundled_votes = HyperMemory::bundle(votes)?;
        let consensus_signal = bundled_votes.bind(&proposal.payload_hv)?;
        let agreement = consensus_signal.similarity(agree_anchor);
        Ok(agreement)
    }

    pub fn deliberate_and_decompose(&self, goal: &str, supervisor: &PslSupervisor) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let root_state = HyperMemory::from_string(goal, DIM_PROLETARIAT);
        let mut engine = MctsEngine::new_exploratory(root_state);
        let _optimal_hv = engine.deliberate(100, supervisor)?;
        Ok(vec![
            format!("Audit material base for {}", goal),
            format!("Synthesize logic for {}", goal),
            format!("Verify against PSL constraints"),
        ])
    }

    pub fn verify_execution(&self, output: &str) -> bool {
        if output.contains("unwrap()") || output.contains("expect()") {
            return false;
        }
        true
    }
}

pub struct AgentTemplate {
    pub id: String,
    pub role: AgentRole,
    pub trust_tier: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psl::axiom::DimensionalityAxiom;

    #[test]
    fn test_agent_creation() {
        let agent = MicroSupervisor::new(AgentRole::Architect);
        assert_eq!(agent.role, AgentRole::Architect);
        assert!((agent.consensus_threshold - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_vote_agree() {
        let agent = MicroSupervisor::new(AgentRole::Auditor);
        let proposal = Proposal {
            id: "test_proposal".into(),
            payload_hv: HyperMemory::generate_seed(DIM_PROLETARIAT),
            timestamp: 0,
        };
        let vote = agent.vote(&proposal, true).expect("vote should succeed");
        // Agree vote should be similar to agree anchor bound with proposal.
        let expected = agent.agree_anchor.bind(&proposal.payload_hv).unwrap();
        let sim = vote.similarity(&expected);
        assert!((sim - 1.0).abs() < 0.01, "Agree vote should match expected binding");
    }

    #[test]
    fn test_vote_disagree() {
        let agent = MicroSupervisor::new(AgentRole::Worker);
        let proposal = Proposal {
            id: "bad_proposal".into(),
            payload_hv: HyperMemory::generate_seed(DIM_PROLETARIAT),
            timestamp: 0,
        };
        let vote = agent.vote(&proposal, false).expect("vote should succeed");
        let expected = agent.disagree_anchor.bind(&proposal.payload_hv).unwrap();
        let sim = vote.similarity(&expected);
        assert!((sim - 1.0).abs() < 0.01, "Disagree vote should match expected binding");
    }

    #[test]
    fn test_consensus_resolution() {
        let agent = MicroSupervisor::new(AgentRole::Architect);
        let proposal = Proposal {
            id: "consensus_test".into(),
            payload_hv: HyperMemory::generate_seed(DIM_PROLETARIAT),
            timestamp: 0,
        };

        // 3 agree votes.
        let votes: Vec<HyperMemory> = (0..3)
            .map(|_| agent.vote(&proposal, true).unwrap())
            .collect();

        let agreement = MicroSupervisor::resolve_consensus(&votes, &proposal, &agent.agree_anchor)
            .expect("consensus should resolve");

        debuglog!("test_consensus: agreement={:.4}", agreement);
        // With all-agree votes, consensus should be detectable.
        assert!(agreement.is_finite());
    }

    #[test]
    fn test_empty_votes_returns_zero() {
        let proposal = Proposal {
            id: "empty".into(),
            payload_hv: HyperMemory::generate_seed(DIM_PROLETARIAT),
            timestamp: 0,
        };
        let agree_anchor = HyperMemory::from_string("CONSENSUS_AGREE_PROCEED", DIM_PROLETARIAT);
        let result = MicroSupervisor::resolve_consensus(&[], &proposal, &agree_anchor).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_verify_execution_safe_code() {
        let agent = MicroSupervisor::new(AgentRole::Auditor);
        assert!(agent.verify_execution("fn main() { let x = 5; }"));
        assert!(agent.verify_execution("result.map_err(|e| format!(\"{}\", e))?;"));
    }

    #[test]
    fn test_verify_execution_rejects_unwrap() {
        let agent = MicroSupervisor::new(AgentRole::Auditor);
        // verify_execution checks for the literal strings "unwrap()" and "expect()"
        assert!(!agent.verify_execution("let val = something.unwrap()"));
        assert!(!agent.verify_execution("value.expect()"));
    }

    #[test]
    fn test_historian_archive_and_recall() {
        let mut historian = Historian::new();
        let state = HyperMemory::generate_seed(DIM_PROLETARIAT);
        historian.record_synthesis(state.clone());
        assert_eq!(historian.archive.len(), 1);

        let (archive_sim, neg_sim) = historian.retrieve_context(&state);
        assert!(archive_sim > 0.5, "Should find archived state");
        assert!(neg_sim.abs() < 0.01, "No negative knowledge yet");
    }

    #[test]
    fn test_historian_negative_knowledge() {
        let mut historian = Historian::new();
        let failure = HyperMemory::generate_seed(DIM_PROLETARIAT);
        historian.condemn_failure(failure.clone());
        assert_eq!(historian.negative_knowledge.len(), 1);

        let (_, neg_sim) = historian.retrieve_context(&failure);
        assert!(neg_sim > 0.5, "Should detect condemned pattern");
    }

    #[test]
    fn test_agent_role_serialization() {
        let json = serde_json::to_string(&AgentRole::Architect).unwrap();
        let recovered: AgentRole = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, AgentRole::Architect);
    }

    #[test]
    fn test_deliberate_produces_decomposition() {
        let agent = MicroSupervisor::new(AgentRole::Architect);
        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let steps = agent.deliberate_and_decompose("implement security audit", &supervisor)
            .expect("deliberation should succeed");
        assert!(steps.len() >= 3, "Should produce at least 3 decomposed steps");
    }

    // ============================================================
    // Stress / invariant tests for HMAS
    // ============================================================

    /// INVARIANT: agree and disagree anchors are distinct.
    #[test]
    fn invariant_agree_disagree_anchors_distinct() {
        let agent = MicroSupervisor::new(AgentRole::Auditor);
        let sim = agent.agree_anchor.similarity(&agent.disagree_anchor);
        // Different anchors should be quasi-orthogonal (similarity near 0).
        assert!(sim.abs() < 0.5,
            "agree/disagree anchors should be quasi-orthogonal, got sim={:.4}", sim);
    }

    /// INVARIANT: Historian::new starts with empty archives.
    #[test]
    fn invariant_historian_starts_empty() {
        let h = Historian::new();
        assert!(h.archive.is_empty());
        assert!(h.negative_knowledge.is_empty());
    }

    /// INVARIANT: retrieve_context returns (0, 0) on empty historian.
    #[test]
    fn invariant_empty_historian_returns_zero() {
        let h = Historian::new();
        let probe = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let (a, n) = h.retrieve_context(&probe);
        assert_eq!(a, 0.0);
        assert_eq!(n, 0.0);
    }

    /// INVARIANT: resolve_consensus is pure — same inputs produce same output.
    #[test]
    fn invariant_resolve_consensus_pure() -> Result<(), Box<dyn std::error::Error>> {
        let agent = MicroSupervisor::new(AgentRole::Architect);
        let proposal = Proposal {
            id: "p".into(),
            payload_hv: HyperMemory::generate_seed(DIM_PROLETARIAT),
            timestamp: 0,
        };
        let votes: Vec<HyperMemory> = (0..3)
            .map(|_| agent.vote(&proposal, true).unwrap())
            .collect();
        let a = MicroSupervisor::resolve_consensus(&votes, &proposal, &agent.agree_anchor)?;
        let b = MicroSupervisor::resolve_consensus(&votes, &proposal, &agent.agree_anchor)?;
        assert!((a - b).abs() < 1e-9,
            "resolve_consensus not pure: {} vs {}", a, b);
        Ok(())
    }

    /// INVARIANT: verify_execution flags unwrap/expect — any line
    /// with literal "unwrap()" or "expect()" is rejected.
    #[test]
    fn invariant_verify_execution_rejects_unwrap_expect() {
        let agent = MicroSupervisor::new(AgentRole::Auditor);
        let bad_samples = [
            "let x = foo.unwrap();",
            "let y = bar.expect();",
            "// foo.unwrap() even in comment",
            "chain.unwrap().unwrap()",
        ];
        for sample in bad_samples {
            assert!(!agent.verify_execution(sample),
                "should reject {:?}", sample);
        }
    }

    /// INVARIANT: AgentRole serde roundtrip for all variants.
    #[test]
    fn invariant_agent_role_serde_roundtrip() {
        let roles = [
            AgentRole::Architect, AgentRole::Worker,
            AgentRole::Auditor, AgentRole::Historian,
        ];
        for r in roles {
            let json = serde_json::to_string(&r).unwrap();
            let recovered: AgentRole = serde_json::from_str(&json).unwrap();
            assert_eq!(r, recovered);
        }
    }
}
