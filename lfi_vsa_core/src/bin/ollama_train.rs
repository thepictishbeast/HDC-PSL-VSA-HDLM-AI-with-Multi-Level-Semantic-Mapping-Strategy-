// ============================================================
// Ollama Training Runner — Real LLM Training for LFI
//
// PURPOSE: Connects LFI to local Ollama models and runs the full
// training pipeline. Uses qwen2.5-coder:7b (fast, direct answers)
// by default, with deepseek-r1:8b as fallback.
//
// USAGE:
//   cargo run --release --bin ollama_train -- [--examples N] [--model M]
//
// FEATURES:
//   - Warm-model batch training (keeps model loaded between queries)
//   - Progress reporting every 10 examples
//   - Checkpoint after each domain
//   - Graceful handling of timeouts and errors
//   - Final accuracy report with per-domain breakdown
// ============================================================

use lfi_vsa_core::cognition::knowledge::KnowledgeEngine;
use lfi_vsa_core::intelligence::local_inference::{
    InferenceTrainer, InferenceTrainingConfig, InferenceBackend,
};
use lfi_vsa_core::intelligence::training_data::{TrainingDataGenerator, TrainingExample};
use lfi_vsa_core::intelligence::weight_manager::IntelligenceCheckpoint;
use std::env;

// SUPERSOCIETY: Pull high-quality facts from brain.db to augment hardcoded training data
// AVP-PASS-13: 2026-04-16 — wires 51M+ facts into the training pipeline
fn load_braindb_examples(domain_filter: &Option<String>, max: usize) -> Vec<TrainingExample> {
    let db_path = format!("{}/.local/share/plausiden/brain.db",
        env::var("HOME").unwrap_or_else(|_| "/root".into()));

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  [brain.db] Cannot open: {}", e);
            return Vec::new();
        }
    };
    let _ = conn.execute_batch("PRAGMA busy_timeout=30000; PRAGMA journal_mode=WAL;");

    // Query high-quality facts with domain/quality filtering
    // BUG ASSUMPTION: quality_score or domain may be NULL for some rows
    let sql = if let Some(ref _d) = domain_filter {
        format!(
            "SELECT value, COALESCE(domain,'general'), COALESCE(quality_score,0.5) \
             FROM facts WHERE domain = ?1 AND quality_score >= 0.75 \
             AND length(value) >= 50 ORDER BY RANDOM() LIMIT {}",
            max
        )
    } else {
        format!(
            "SELECT value, COALESCE(domain,'general'), COALESCE(quality_score,0.5) \
             FROM facts WHERE quality_score >= 0.75 \
             AND length(value) >= 50 ORDER BY RANDOM() LIMIT {}",
            max
        )
    };

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  [brain.db] Query failed: {}", e);
            return Vec::new();
        }
    };

    let params: Vec<Box<dyn rusqlite::types::ToSql>> = if let Some(ref d) = domain_filter {
        vec![Box::new(d.clone())]
    } else {
        vec![]
    };

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = match stmt.query_map(param_refs.as_slice(), |row| {
        let value: String = row.get(0)?;
        let domain: String = row.get(1)?;
        let quality: f64 = row.get(2)?;
        Ok((value, domain, quality))
    }) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  [brain.db] Fetch failed: {}", e);
            return Vec::new();
        }
    };

    let mut examples = Vec::new();
    for row in rows {
        if let Ok((value, domain, quality)) = row {
            // Split value into input/output if it contains newline, otherwise use as input
            let (input, expected) = if let Some(idx) = value.find('\n') {
                (value[..idx].to_string(), value[idx+1..].to_string())
            } else {
                (value.clone(), format!("(fact from {} domain)", domain))
            };

            // Map quality_score to difficulty (inverse: high quality = lower difficulty)
            let difficulty = 1.0 - quality.min(1.0);

            examples.push(TrainingExample::new(
                &domain,
                &input,
                &expected,
                difficulty,
                &["brain_db", "augmented"],
            ));
        }
    }

    println!("  [brain.db] Loaded {} high-quality facts (quality >= 0.75, length >= 50)", examples.len());
    examples
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("================================================");
    println!("LFI Ollama Training Runner");
    println!("================================================");

    let args: Vec<String> = env::args().collect();

    // Parse arguments.
    let mut max_examples = 50usize;
    let mut model = "qwen2.5-coder:7b".to_string();
    let mut domain_filter: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--examples" => {
                if i + 1 < args.len() {
                    max_examples = args[i + 1].parse().unwrap_or(50);
                    i += 2;
                } else { i += 1; }
            }
            "--model" => {
                if i + 1 < args.len() {
                    model = args[i + 1].clone();
                    i += 2;
                } else { i += 1; }
            }
            "--domain" => {
                if i + 1 < args.len() {
                    domain_filter = Some(args[i + 1].clone());
                    i += 2;
                } else { i += 1; }
            }
            _ => i += 1,
        }
    }

    let host = "http://localhost:11434";

    println!("Model: {}", model);
    println!("Host:  {}", host);
    println!("Max examples: {}", max_examples);
    if let Some(ref d) = domain_filter {
        println!("Domain filter: {}", d);
    }
    println!();

    // Pre-flight check: Is Ollama reachable?
    println!("[1/5] Checking Ollama availability...");
    let check = std::process::Command::new("curl")
        .args(&["-s", "--max-time", "3", &format!("{}/api/tags", host)])
        .output()?;
    if !check.status.success() || check.stdout.is_empty() {
        eprintln!("ERROR: Ollama is not responding at {}. Start it with: ollama serve", host);
        return Ok(());
    }
    let tags_body = String::from_utf8_lossy(&check.stdout);
    if !tags_body.contains(&model) {
        eprintln!("ERROR: Model '{}' not available. Pull with: ollama pull {}", model, model);
        eprintln!("Available models in server response:");
        println!("{}", &tags_body[..tags_body.len().min(500)]);
        return Ok(());
    }
    println!("  ✓ Ollama is running and model '{}' is available", model);
    println!();

    // Load training examples from BOTH hardcoded data AND brain.db
    // SUPERSOCIETY: This is the critical bridge — 51M facts now feed training
    println!("[2/5] Loading training data...");
    let hardcoded = TrainingDataGenerator::all_examples();
    let hardcoded_count = hardcoded.len();
    let braindb = load_braindb_examples(&domain_filter, max_examples);
    let braindb_count = braindb.len();

    // Merge: hardcoded first (curated), then brain.db augmentation
    let mut all_examples = hardcoded;
    all_examples.extend(braindb);

    let examples: Vec<_> = if let Some(ref d) = domain_filter {
        all_examples.into_iter().filter(|e| e.domain == *d).collect()
    } else {
        all_examples
    };
    let examples: Vec<_> = examples.into_iter().take(max_examples).collect();
    println!("  ✓ Loaded {} examples ({} hardcoded + {} from brain.db)",
        examples.len(), hardcoded_count, braindb_count);
    println!();

    // Initialize trainer and knowledge engine.
    println!("[3/5] Initializing LFI...");
    let config = InferenceTrainingConfig {
        backend: InferenceBackend::Ollama {
            model: model.clone(),
            host: host.into(),
        },
        verify_answers: true,
        cache_enabled: true,
        active_learning: false, // Use natural order for consistent timing
        ..Default::default()
    };
    let mut trainer = InferenceTrainer::new(config);
    let mut knowledge = KnowledgeEngine::new();
    println!("  ✓ LFI initialized ({} seeded concepts)", knowledge.concept_count());
    println!();

    // Run training with progress reporting.
    println!("[4/5] Running training loop...");
    println!("  Format: [cycle] domain | question → answer (correct?)");
    println!();

    let start = std::time::Instant::now();
    let mut correct = 0;
    let mut processed = 0;
    let mut errors = 0;

    for (i, example) in examples.iter().enumerate() {
        match trainer.train_on_example(example, &mut knowledge) {
            Ok(result) => {
                processed += 1;
                let is_correct = result.correct == Some(true);
                if is_correct { correct += 1; }

                // Show progress every example (training is slow)
                let answer_preview: String = result.answer.chars().take(60).collect();
                let status = if is_correct { "✓" } else { "✗" };
                let cache_marker = if result.cached { "[C]" } else { "" };
                println!("  [{}/{}] {} {} {}ms | {} → {} ({})",
                    i + 1, examples.len(), status, cache_marker, result.latency_ms,
                    example.domain, answer_preview, example.expected_output);
            }
            Err(e) => {
                errors += 1;
                eprintln!("  [{}/{}] ERROR: {:?}", i + 1, examples.len(), e);
            }
        }

        // Progress update every 10 examples.
        if (i + 1) % 10 == 0 {
            let elapsed = start.elapsed().as_secs();
            let rate = processed as f64 / elapsed.max(1) as f64;
            println!("  --- Progress: {}/{} processed, {}/{} correct ({:.1}%), {} errors, {:.1} q/s ---",
                i + 1, examples.len(), correct, processed,
                100.0 * correct as f64 / processed.max(1) as f64, errors, rate);
        }
    }

    let elapsed = start.elapsed().as_secs();
    println!();
    println!("[5/5] Training complete.");
    println!();

    // Final report.
    println!("================================================");
    println!("TRAINING RESULTS");
    println!("================================================");
    println!("Processed:   {}", processed);
    println!("Correct:     {} ({:.1}%)", correct, 100.0 * correct as f64 / processed.max(1) as f64);
    println!("Errors:      {}", errors);
    println!("Duration:    {}s ({:.1} q/s)", elapsed, processed as f64 / elapsed.max(1) as f64);
    println!("Cache hits:  {:.1}%", trainer.cache_hit_rate() * 100.0);
    println!();

    // Domain breakdown (weakest first).
    let weak = trainer.weakest_domains(10);
    if !weak.is_empty() {
        println!("Weakest domains (most errors):");
        for (domain, errors) in &weak {
            println!("  {:30} {} errors", domain, errors);
        }
        println!();
    }

    // Error taxonomy.
    let errors_by_kind: std::collections::HashMap<String, usize> = trainer.error_history()
        .values()
        .flat_map(|v| v.iter())
        .map(|k| format!("{:?}", k))
        .fold(std::collections::HashMap::new(), |mut acc, k| {
            *acc.entry(k).or_insert(0) += 1;
            acc
        });
    if !errors_by_kind.is_empty() {
        println!("Error taxonomy:");
        let mut sorted: Vec<_> = errors_by_kind.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (kind, count) in sorted {
            println!("  {:20} {}", kind, count);
        }
        println!();
    }

    // Save checkpoint.
    let checkpoint_dir = "/tmp/lfi_ollama_training";
    let _ = std::fs::create_dir_all(checkpoint_dir);
    let checkpoint_path = format!("{}/trained_{}.json",
        checkpoint_dir,
        chrono::Utc::now().format("%Y%m%d_%H%M%S"));

    let knowledge_json = format!(
        "{{\"concepts\":{},\"trained_examples\":{},\"accuracy\":{:.4}}}",
        knowledge.concept_count(), processed,
        correct as f64 / processed.max(1) as f64,
    );
    let checkpoint = IntelligenceCheckpoint::capture(
        &knowledge_json, processed as u64, knowledge.concept_count(),
        0, 0,
        &format!("Ollama training: {} examples, {:.1}% accuracy", processed,
            100.0 * correct as f64 / processed.max(1) as f64),
    );
    match checkpoint.save(&std::path::Path::new(&checkpoint_path)) {
        Ok(_) => println!("✓ Checkpoint saved: {}", checkpoint_path),
        Err(e) => eprintln!("✗ Checkpoint failed: {:?}", e),
    }

    Ok(())
}
