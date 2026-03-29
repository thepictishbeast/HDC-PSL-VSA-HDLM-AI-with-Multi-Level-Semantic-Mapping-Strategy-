// NODE 021: Adversarial Self-Play Forge
// STATUS: ALPHA - Strategic Dialectic Active
// PROTOCOL: Thesis-Antithesis-Synthesis Loop

use lfi_vsa_core::agent::LfiAgent;
use lfi_vsa_core::cognition::mcts::MctsEngine;
use lfi_vsa_core::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use lfi_vsa_core::psl::axiom::AuditTarget;
use lfi_vsa_core::hdc::vector::BipolarVector;
use tracing::{info, warn, debug};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("// AUDIT: Initiating Adversarial Self-Play. Forging Strategic Kernel...");

    let mut agent = LfiAgent::new()?;
    
    // Set Auditor to "Hostile" mode (high similarity threshold)
    agent.supervisor.material_trust_threshold = 0.90;
    
    let generations = 1000000; 
    let mut synthesis_count = 0;

    for i in 0..generations {
        debug!("// DEBUG: Starting Generation {}", i);

        // 1. THE THESIS: The Strategist proposes a move via MCTS
        let goal_text = format!("STRATEGIC_MOVE_GEN_{}", i);
        let root_state = HyperMemory::from_string(&goal_text, DIM_PROLETARIAT);
        let mut mcts = MctsEngine::new(root_state);
        
        let thesis_hv = mcts.deliberate(20, &agent.supervisor)?;

        // 2. THE ANTITHESIS: The Auditor audits the move against NeuPSL
        // Convert HyperMemory (ndarray) to BipolarVector (bitvec) for PSL audit
        let bit_data: Vec<bool> = thesis_hv.vector.iter().map(|&x| x > 0).collect();
        let target_vec = BipolarVector { data: bitvec::prelude::BitVec::from_iter(bit_data) };
        let target = AuditTarget::Vector(target_vec);

        let architect_ok = agent.reasoner.planner().plan(&goal_text).is_ok();
        let audit_result = agent.supervisor.audit(&target)?;
        let auditor_ok = audit_result.level.permits_execution();

        if architect_ok && auditor_ok {
            // 3. THE SYNTHESIS: Hardened strategy confirmed
            info!("// AUDIT: Generation {} - SYNTHESIS ACHIEVED. Agreement: {:.4}", i, audit_result.confidence);
            
            // Permanent VSA binding via SuperpositionStorage
            // In a production Supersociety loop, we use 'commit_real'
            let _ = agent.memory.commit_real(&BipolarVector::from_seed(i as u64));
            synthesis_count += 1;
        } else {
            warn!("// AUDIT: Generation {} - REJECTED. Forensic trace identified flaw.", i);
        }
    }

    info!("// AUDIT: Self-Play Complete. {} hardened strategies forged.", synthesis_count);
    Ok(())
}
