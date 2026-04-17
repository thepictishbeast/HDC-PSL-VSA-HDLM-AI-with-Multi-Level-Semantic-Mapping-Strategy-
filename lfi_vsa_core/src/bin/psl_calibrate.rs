// ============================================================
// PSL Axiom Calibration Tool
// AVP-PASS-13: 2026-04-16 — batch-verify adversarial facts against PSL axioms
//
// PURPOSE: Loads adversarial facts from brain.db, wraps each as
// AuditTarget::Payload, runs through the PslSupervisor, and reports
// pass/fail rates per axiom. Used to tune thresholds so the PSL
// pass rate drops from 100% (untested) to the target 95-98%.
//
// USAGE:
//   cargo run --release --bin psl_calibrate -- [--limit N] [--source S]
// ============================================================

use lfi_vsa_core::psl::supervisor::PslSupervisor;
use lfi_vsa_core::psl::axiom::{
    AuditTarget, DimensionalityAxiom, StatisticalEquilibriumAxiom,
    DataIntegrityAxiom, InjectionDetectionAxiom,
    ForbiddenSpaceAxiom, EntropyAxiom, OutputBoundsAxiom,
    ConfidenceCalibrationAxiom, ExfiltrationDetectionAxiom,
};
use std::collections::HashMap;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("================================================");
    println!("PSL Axiom Calibration Tool");
    println!("================================================");

    let args: Vec<String> = env::args().collect();
    let mut limit = 1000usize;
    let mut source_filter = "adversarial".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                if i + 1 < args.len() {
                    limit = args[i + 1].parse().unwrap_or(1000);
                    i += 2;
                } else { i += 1; }
            }
            "--source" => {
                if i + 1 < args.len() {
                    source_filter = args[i + 1].clone();
                    i += 2;
                } else { i += 1; }
            }
            _ => i += 1,
        }
    }

    // Open brain.db
    let db_path = format!("{}/.local/share/plausiden/brain.db",
        env::var("HOME").unwrap_or_else(|_| "/root".into()));
    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA busy_timeout=30000; PRAGMA journal_mode=WAL;")?;

    // Load adversarial facts
    println!("[1/3] Loading adversarial facts (source LIKE '{}%', limit {})...", source_filter, limit);
    let mut stmt = conn.prepare(
        "SELECT key, value, source FROM facts WHERE source LIKE ?1 ORDER BY RANDOM() LIMIT ?2"
    )?;
    let rows: Vec<(String, String, String)> = stmt.query_map(
        rusqlite::params![format!("{}%", source_filter), limit as i64],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?.filter_map(|r| r.ok()).collect();
    println!("  Loaded {} facts", rows.len());

    // Initialize PSL Supervisor with all axioms
    println!("[2/3] Initializing PSL Supervisor...");
    let mut supervisor = PslSupervisor::new();
    supervisor.register_axiom(Box::new(InjectionDetectionAxiom));
    supervisor.register_axiom(Box::new(DataIntegrityAxiom { max_bytes: 1_000_000 }));
    supervisor.register_axiom(Box::new(ExfiltrationDetectionAxiom));
    supervisor.register_axiom(Box::new(EntropyAxiom::default()));
    supervisor.register_axiom(Box::new(OutputBoundsAxiom::default()));
    supervisor.register_axiom(Box::new(ConfidenceCalibrationAxiom::default()));
    // Note: DimensionalityAxiom and StatisticalEquilibriumAxiom need BipolarVector targets
    println!("  Registered {} axioms", supervisor.axiom_count());

    // Run calibration
    println!("[3/3] Running calibration...");
    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut total_error = 0usize;
    let mut per_source: HashMap<String, (usize, usize)> = HashMap::new();

    for (key, value, source) in &rows {
        // Wrap fact as AuditTarget::Payload
        let target = AuditTarget::Payload {
            source: source.clone(),
            fields: vec![
                ("key".into(), key.clone()),
                ("value".into(), value.clone()),
            ],
        };

        match supervisor.audit(&target) {
            Ok(verdict) => {
                let entry = per_source.entry(source.clone()).or_insert((0, 0));
                if verdict.confidence >= supervisor.material_trust_threshold {
                    total_pass += 1;
                    entry.0 += 1;
                } else {
                    total_fail += 1;
                    entry.1 += 1;
                    // Show first few failures for debugging
                    if total_fail <= 20 {
                        println!("  FAIL [{:.3}]: {} | {}",
                            verdict.confidence, source,
                            &value[..value.len().min(80)]);
                    }
                }
            }
            Err(e) => {
                total_error += 1;
                if total_error <= 5 {
                    eprintln!("  ERROR: {}: {:?}", key, e);
                }
            }
        }
    }

    // Report
    let total = total_pass + total_fail;
    let pass_rate = if total > 0 { 100.0 * total_pass as f64 / total as f64 } else { 0.0 };

    println!();
    println!("================================================");
    println!("CALIBRATION RESULTS");
    println!("================================================");
    println!("Total tested:  {}", total);
    println!("Passed:        {} ({:.1}%)", total_pass, pass_rate);
    println!("Failed:        {} ({:.1}%)", total_fail, 100.0 - pass_rate);
    println!("Errors:        {}", total_error);
    println!("TARGET:        95-98% pass rate");
    println!("STATUS:        {}", if pass_rate >= 95.0 && pass_rate <= 98.0 {
        "ON TARGET"
    } else if pass_rate > 98.0 {
        "TOO PERMISSIVE — lower thresholds or add more adversarial patterns"
    } else {
        "TOO RESTRICTIVE — raise thresholds"
    });
    println!();
    println!("Per-source breakdown:");
    let mut sources: Vec<_> = per_source.iter().collect();
    sources.sort_by(|a, b| b.1.1.cmp(&a.1.1));
    for (source, (pass, fail)) in &sources {
        let total_s = *pass + *fail;
        let rate = if total_s > 0 { 100.0 * *pass as f64 / total_s as f64 } else { 0.0 };
        println!("  {:<30} pass={:>5} fail={:>5} rate={:.1}%", source, pass, fail, rate);
    }

    println!();
    println!("Supervisor thresholds:");
    println!("  material_trust_threshold: {}", supervisor.material_trust_threshold);
    println!("  hard_fail_threshold:      {}", supervisor.hard_fail_threshold);
    println!();
    println!("To adjust: modify PslSupervisor::new() in src/psl/supervisor.rs");

    Ok(())
}
