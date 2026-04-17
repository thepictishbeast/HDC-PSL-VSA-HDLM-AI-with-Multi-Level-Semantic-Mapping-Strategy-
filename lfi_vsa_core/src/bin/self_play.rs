// NODE 021: Adversarial Self-Play Forge
// STATUS: ALPHA - Strategic Dialectic Active
// PROTOCOL: Thesis-Antithesis-Synthesis Loop
//
// PROVENANCE: Every synthesis records a TracedDerivation chain covering
// MCTS deliberation → PSL axiom evaluation → synthesis binding.
// Arenas are written to ~/.lfi/provenance/self_play_gen_<N>.json so
// strategy evolution is analyzable post-hoc.

use lfi_vsa_core::agent::LfiAgent;
use lfi_vsa_core::cognition::mcts::MctsEngine;
use lfi_vsa_core::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use lfi_vsa_core::psl::axiom::AuditTarget;
use lfi_vsa_core::hdc::vector::BipolarVector;
use lfi_vsa_core::reasoning_provenance::{InferenceSource, TraceArena};
use std::path::PathBuf;
use tracing::{info, warn, debug};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("// AUDIT: Initiating Adversarial Self-Play. Forging Strategic Kernel...");

    let mut agent = LfiAgent::new()?;
    agent.supervisor.material_trust_threshold = 0.90;

    // Resolve provenance output directory.
    let provenance_root: PathBuf = std::env::var("LFI_PROVENANCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."));
            p.push(".lfi");
            p.push("provenance");
            p
        });
    let _ = std::fs::create_dir_all(&provenance_root);

    let generations = 1000000;
    let mut synthesis_count = 0;

    for i in 0..generations {
        debug!("// DEBUG: Starting Generation {}", i);

        // 1. THE THESIS: Strategist proposes a move via MCTS — with provenance.
        let goal_text = format!("STRATEGIC_MOVE_GEN_{}", i);
        let root_state = HyperMemory::from_string(&goal_text, DIM_PROLETARIAT);
        let mut mcts = MctsEngine::new_exploratory(root_state);
        mcts.enable_provenance();

        let thesis_hv = mcts.deliberate(20, &agent.supervisor)?;

        // 2. THE ANTITHESIS: Auditor audits the move against PSL — with provenance.
        let bit_data: Vec<bool> = thesis_hv.vector.iter().map(|&x| x > 0).collect();
        let target_vec = BipolarVector { data: bitvec::prelude::BitVec::from_iter(bit_data) };
        let target = AuditTarget::Vector(target_vec);

        // Take the MCTS arena so we can extend it with PSL + synthesis traces.
        let mut arena: TraceArena = mcts.take_provenance().unwrap_or_default();
        // Link PSL audit to the most recent MCTS trace (last inserted).
        let last_mcts = if arena.len() > 0 { Some(arena.len() - 1) } else { None };
        let (audit_result, _psl_trace_ids) = agent.supervisor
            .audit_with_provenance(&target, &mut arena, last_mcts)?;

        let architect_ok = agent.reasoner.planner().plan(&goal_text).is_ok();
        let auditor_ok = audit_result.level.permits_execution();

        if architect_ok && auditor_ok {
            // 3. THE SYNTHESIS: Record the outcome as a traced derivation.
            info!("// AUDIT: Generation {} - SYNTHESIS ACHIEVED. Agreement: {:.4}",
                i, audit_result.confidence);

            let synthesis_cid = i as u64;
            let parent = if arena.len() > 0 { Some(arena.len() - 1) } else { None };
            arena.record_step(
                parent,
                InferenceSource::SelfPlayEpisode { generation: i },
                vec![format!("synthesis_gen_{}", i)],
                audit_result.confidence,
                Some(synthesis_cid),
                format!("Self-play synthesis @ gen {}: conf={:.4}, architect_ok={}",
                    i, audit_result.confidence, architect_ok),
                0,
            );

            // Persist the full thesis → antithesis → synthesis arena.
            let out = provenance_root.join(format!("self_play_gen_{}.json", i));
            if let Err(e) = arena.save_to_path(&out) {
                warn!("// AUDIT: failed to persist provenance for gen {}: {}", i, e);
            }

            let _ = agent.memory.commit_real(&BipolarVector::from_seed(i as u64));
            synthesis_count += 1;
        } else {
            warn!("// AUDIT: Generation {} - REJECTED. Forensic trace identified flaw.", i);
        }
    }

    info!("// AUDIT: Self-Play Complete. {} hardened strategies forged.", synthesis_count);
    Ok(())
}
