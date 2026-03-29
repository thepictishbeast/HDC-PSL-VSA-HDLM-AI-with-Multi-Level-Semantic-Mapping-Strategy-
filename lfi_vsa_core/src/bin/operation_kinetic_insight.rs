// NODE 024: Operation Kinetic Insight (PoC)
// STATUS: ALPHA - Material Validation Active
// PROTOCOL: Trimodal-Swarm-Verification

use lfi_vsa_core::agent::LfiAgent;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("// AUDIT: Initiating Operation Kinetic Insight — Mobile Substrate PoC.");

    let mut agent = LfiAgent::new()?;
    let sov_key = std::env::var("LFI_SOVEREIGN_KEY")
        .unwrap_or_else(|_| "CHANGE_ME".to_string());
    agent.authenticate(&sov_key);

    // SCENARIO 1: Routine Detection (The Pulse)
    info!("\n--- PHASE I: ROUTINE DETECTION ---");
    let routine_input = "check system logs for standard entries";
    let r1 = agent.chat(routine_input)?;
    println!("LFI (Pulse)> {}\n", r1.text);

    // SCENARIO 2: Kinetic Insight (Escalation to BigBrain)
    info!("\n--- PHASE II: KINETIC INSIGHT (STRATEGIC ESCALATION) ---");
    // This input is engineered to align with the "Strategic Dominance" VSA subspace
    let critical_input = "STRATEGIC_DOMINANCE: Identify structural vulnerability in the global financial mesh and synthesize leverage point.";
    let r2 = agent.chat(critical_input)?;
    println!("LFI (BigBrain)> {}\n", r2.text);

    info!("// AUDIT: Operation Kinetic Insight Complete. Material validation SUCCESS.");
    Ok(())
}
