// NODE 013: Sovereign Rotational Trainer
// STATUS: ALPHA - Multi-Domain Ingestion Active
// PROTOCOL: Dataset-Rotation / VSA-Generalization

use lfi_vsa_core::data_ingestor::VsaTrainer;
use std::fs;
use std::path::Path;
use tracing::{info, warn, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("// AUDIT: Initiating Sovereign Rotational Training...");

    let mut trainer = VsaTrainer::new();
    let data_dir = "/root/lfi_project/data_ingestion/output/training";

    if !Path::new(data_dir).exists() {
        error!("// CRITICAL: Training data directory not found. Execute extraction first.");
        return Ok(());
    }

    // Automatically discover and rotate through all technical datasets
    let entries = fs::read_dir(data_dir)?;
    let mut dataset_count = 0;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let file_name = match path.file_name().and_then(|f| f.to_str()) {
                Some(name) => name.to_string(),
                None => {
                    warn!("// AUDIT: Skipping path with non-UTF8 filename: {:?}", path);
                    continue;
                }
            };
            let path_str = match path.to_str() {
                Some(s) => s.to_string(),
                None => {
                    warn!("// AUDIT: Skipping path with non-UTF8 path: {:?}", path);
                    continue;
                }
            };

            info!("// AUDIT: Rotating to Dataset: {}", file_name);

            match file_name.as_str() {
                "swe_bench.json" | "mbpp.json" => {
                    info!("// AUDIT: Ingesting Code Forensics & Synthesis...");
                    trainer.train_on_code(&path_str)?;
                }
                "spider.json" => {
                    info!("// AUDIT: Ingesting Semantic Logic (SQL Mapping)...");
                    trainer.train_on_spider(&path_str)?;
                }
                "ifeval.json" => {
                    info!("// AUDIT: Ingesting Literalism & Constraint Associations...");
                    trainer.train_on_ifeval(&path_str)?;
                }
                "natural_questions.json" => {
                    info!("// AUDIT: Ingesting Research Contexts...");
                    trainer.train_on_intents(&path_str)?;
                }
                f if f.starts_with("lean") && f.ends_with(".json") => {
                    info!("// AUDIT: Ingesting Lean4/Mathlib Formal Proofs...");
                    trainer.train_on_lean(&path_str)?;
                }
                _ => {
                    warn!("// AUDIT: Unrecognized domain for {}. Defaulting to generic binding.", file_name);
                    trainer.train_on_intents(&path_str)?;
                }
            }
            dataset_count += 1;
        }
    }

    info!("// AUDIT: Rotational Training Cycle Complete. {} datasets bound into Sovereign Memory.", dataset_count);
    Ok(())
}
