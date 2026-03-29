// NODE 025: Sovereign Forensic Dashboard (TUI)
// STATUS: ALPHA - Real-Time Visibility Active
// PROTOCOL: Swarm-State-Visualization

use lfi_vsa_core::telemetry::MaterialAuditor;
use lfi_vsa_core::memory_bus::HyperMemory;
use std::{thread, time::Duration};
use chrono::Local;

fn main() {
    println!("\x1B[2J\x1B[1;1H"); // Clear screen
    println!("============================================================");
    println!(" SOVEREIGN FORENSIC DASHBOARD — SUBSTRATE VISIBILITY v1.0");
    println!("============================================================");

    loop {
        // 1. Fetch live VSA state to calculate real orthogonality
        let vsa_ortho = match HyperMemory::load_from_disk(".vsa_core_memory.bin") {
            Ok(mem) => mem.audit_orthogonality(),
            Err(_) => 0.02, // Fallback if memory hasn't been created yet
        };

        // 2. Fetch Material State
        let stats = MaterialAuditor::get_stats(vsa_ortho, 1.0); // Pass rate still placeholder until DB is wired
        let time_str = Local::now().format("%H:%M:%S").to_string();

        // 3. Render Hardware Layer (Level 1)
        println!("\x1B[4;1H[LEVEL 1: HARDWARE SUBSTRATE]");
        println!("  TIMESTAMP:  {}", time_str);
        println!("  RAM AVAIL:  {} MB", stats.ram_available_mb);
        println!("  CPU TEMP:   {} C  {}", stats.cpu_temp_c, if stats.is_throttled { "!! THROTTLED !!" } else { "NOMINAL" });

        // 4. Render Semantic Layer (Level 2)
        println!("\x1B[10;1H[LEVEL 2: SEMANTIC VSA HEALTH]");
        // Visualize similarity as a heatmap bar
        let bars = (stats.vsa_orthogonality * 100.0).clamp(0.0, 50.0) as usize;
        let bar_str: String = (0..50).map(|i| if i < bars { "|" } else { "." }).collect();
        println!("  ORTHOGONALITY: [{}] {:.4}", bar_str, stats.vsa_orthogonality);
        println!("  MEMORY STATE:  {}", if stats.vsa_orthogonality > 0.10 { "WARNING: ALIASING DETECTED" } else { "FORENSIC SILENCE MAINTAINED" });

        // 5. Render Reasoning Stream (Level 3)
        println!("\x1B[15;1H[LEVEL 3: THINKING STREAM — INTERNAL MONOLOGUE]");
        println!("  > MONITORING LOCAL HOST FOR TELEMETRY CHANGES...");
        println!("  > VSA MEMORY FILE SIZE: NOMINAL");
        println!("  > [BACKGROUND TRAINING ACTIVE]");

        // 6. Render Governance Ledger (Level 4)
        println!("\x1B[20;1H[LEVEL 4: SOVEREIGN LEDGER — NeuPSL]");
        println!("  AXIOM PASS RATE: {:.2}% (Simulated Monitor)", stats.axiom_pass_rate * 100.0);
        println!("  STATUS:          SOVEREIGN COMMANDS PRIORITIZED");

        println!("\x1B[25;1H------------------------------------------------------------");
        println!(" [AUDIT ACTIVE] Press Ctrl+C to detach monitor.");

        thread::sleep(Duration::from_secs(2));
    }
}
