// NODE 016: Swarm Consensus Integration Test
// STATUS: ALPHA - Algorithmic Cybernetics Verified
// PROTOCOL: Federated Voting Audit

use lfi_vsa_core::hmas::{MicroSupervisor, AgentRole, Proposal};
use lfi_vsa_core::memory_bus::{HyperMemory, DIM_PROLETARIAT};

#[test]
fn test_swarm_mathematical_consensus() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize the Swarm
    let architect = MicroSupervisor::new(AgentRole::Architect);
    let worker = MicroSupervisor::new(AgentRole::Worker);
    let auditor = MicroSupervisor::new(AgentRole::Auditor);
    let historian = MicroSupervisor::new(AgentRole::Historian);

    // 2. Define a Proposal (e.g., "Implement VSA Binding in logic loop")
    let proposal_text = "IMPLEMENT_VSA_LOGIC_LOOP";
    let proposal = Proposal {
        id: "PROP_001".into(),
        payload_hv: HyperMemory::from_string(proposal_text, DIM_PROLETARIAT),
        timestamp: 1774724000,
    };

    // 3. Cast Votes
    // Architect, Worker, and Historian agree (Verified=true)
    // Auditor DISAGREES (Verified=false) due to a logic flaw detection
    let v1 = architect.vote(&proposal, true)?;
    let v2 = worker.vote(&proposal, true)?;
    let v3 = historian.vote(&proposal, true)?;
    let v4 = auditor.vote(&proposal, false)?;

    // 4. Resolve Consensus (Aggregating the bundled signal)
    let votes = vec![v1, v2, v3, v4];
    let agreement = MicroSupervisor::resolve_consensus(&votes, &proposal, &architect.agree_anchor)?;

    // 5. Audit the Quorum
    // Since 3 out of 4 agreed, the agreement similarity should be high (> 0.5), 
    // but the 1 disagreement creates a forensic trace in the VSA space.
    println!("Swarm Agreement Level: {:.4}", agreement);
    assert!(agreement > architect.consensus_threshold - 0.2); // Consensus reached but imperfect
    
    Ok(())
}
