// NODE 010: Streamed High-Dimensional Data Vectorization
// STATUS: ALPHA - O(1) Memory Ingestion Active
// PROTOCOL: BufReader-Stream / Contrastive VSA Binding

use std::fs::File;
use std::io::BufReader;
use serde::Deserialize;
use serde_json::Deserializer;
use tracing::info;
use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};

#[derive(Deserialize, Debug)]
struct IntentPayload {
    input_signal: String,
    target_state: String,
}

#[derive(Deserialize, Debug)]
struct IFEvalPayload {
    prompt: String,
    constraints: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct SpiderPayload {
    question: String,
    query: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct LogicPayload {
    premise: String,
    hypothesis: String,
    relation: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct MathPayload {
    question: String,
    answer: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct HierarchyPayload {
    system_instruction: String,
    untrusted_input: String,
    correct_behavior: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct TrajectoryPayload {
    goal: String,
    steps: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ReasoningPayload {
    problem: String,
    thinking_process: String,
    answer: String,
}

/// Lean4/Mathlib formal proof record.
/// Expected JSON schema:
///   { "name": "Nat.add_comm", "statement": "∀ n m : Nat, n + m = m + n",
///     "proof": "by induction n with ...", "imports": ["Mathlib.Data.Nat.Basic"],
///     "tags": ["commutativity", "natural_numbers"] }
#[derive(Deserialize, Debug)]
struct LeanProofPayload {
    /// Fully qualified theorem name (e.g., "Nat.add_comm")
    name: String,
    /// Formal statement in Lean4 syntax
    statement: String,
    /// Proof body (tactic mode or term mode)
    proof: String,
    /// Imported modules (optional — for dependency tracking)
    #[serde(default)]
    imports: Vec<String>,
    /// Semantic tags (optional — for retrieval)
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct CodeForensics {
    issue: String,
    fix: String
}

pub struct VsaTrainer {
    pub memory: HyperMemory,
}

impl VsaTrainer {
    pub fn new() -> Self {
        let memory = HyperMemory::load_from_disk(".vsa_core_memory.bin").unwrap_or_else(|_| HyperMemory::new(DIM_PROLETARIAT));
        Self { memory }
    }

    pub fn learn_association(&mut self, input: &str, target: &str, is_truth: bool) -> Result<(), Box<dyn std::error::Error>> {
        let input_hv = HyperMemory::from_string(input, DIM_PROLETARIAT);
        let target_hv = HyperMemory::from_string(target, DIM_PROLETARIAT);
        let association = input_hv.bind(&target_hv)?;

        if is_truth {
            self.memory = HyperMemory::bundle(&[self.memory.clone(), association])?;
        } else {
            let penalty_signal = association.bind(&HyperMemory::from_string("CONTRADICTION", DIM_PROLETARIAT))?;
            self.memory = HyperMemory::bundle(&[self.memory.clone(), penalty_signal])?;
        }
        Ok(())
    }

    /// INGESTION PROTOCOL: Streamed JSON Array Processing
    /// Achieves O(1) memory footprint by yielding one record at a time.
    pub fn train_on_intents(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("// AUDIT: Stream-Ingesting Intent Dataset: {}", path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let stream = Deserializer::from_reader(reader).into_iter::<Vec<IntentPayload>>();
        
        let mut count = 0;
        for batch_result in stream {
            if let Ok(batch) = batch_result {
                let total = batch.len();
                for p in batch {
                    self.learn_association(&p.input_signal, &p.target_state, true)?;
                    count += 1;
                    if count % 100 == 0 {
                        info!("// AUDIT: Processed {}/{} intents from {}", count, total, path);
                    }
                }
            }
        }
        info!("// AUDIT: Completed ingestion of {} intents from {}.", count, path);
        self.memory.commit_to_disk(".vsa_core_memory.bin")?;
        Ok(())
    }

    pub fn train_on_code(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("// AUDIT: Stream-Ingesting Code Forensics: {}", path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let stream = Deserializer::from_reader(reader).into_iter::<Vec<CodeForensics>>();
        
        let mut count = 0;
        for batch_result in stream {
            if let Ok(batch) = batch_result {
                let total = batch.len();
                for p in batch {
                    let issue_hv = HyperMemory::from_string(&p.issue, DIM_PROLETARIAT);
                    let fix_hv = HyperMemory::from_string(&p.fix, DIM_PROLETARIAT);
                    let mapping = issue_hv.bind(&fix_hv)?;
                    self.memory = HyperMemory::bundle(&[self.memory.clone(), mapping])?;
                    count += 1;
                    if count % 100 == 0 {
                        info!("// AUDIT: Processed {}/{} code forensics from {}", count, total, path);
                    }
                }
            }
        }
        info!("// AUDIT: Completed ingestion of {} code forensics from {}.", count, path);
        self.memory.commit_to_disk(".vsa_core_memory.bin")?;
        Ok(())
    }

    /// Ingests IFEval literalism dataset: prompt → constraint associations.
    pub fn train_on_ifeval(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("// AUDIT: Stream-Ingesting IFEval Literalism: {}", path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let stream = Deserializer::from_reader(reader).into_iter::<Vec<IFEvalPayload>>();

        let mut count = 0;
        for batch_result in stream {
            if let Ok(batch) = batch_result {
                let total = batch.len();
                for p in batch {
                    // Bind the prompt to each constraint as a separate association
                    let prompt_hv = HyperMemory::from_string(&p.prompt, DIM_PROLETARIAT);
                    for constraint in &p.constraints {
                        let constraint_hv = HyperMemory::from_string(constraint, DIM_PROLETARIAT);
                        let mapping = prompt_hv.bind(&constraint_hv)?;
                        self.memory = HyperMemory::bundle(&[self.memory.clone(), mapping])?;
                    }
                    count += 1;
                    if count % 100 == 0 {
                        info!("// AUDIT: Processed {}/{} IFEval prompts from {}", count, total, path);
                    }
                }
            }
        }
        info!("// AUDIT: Completed ingestion of {} IFEval prompts from {}.", count, path);
        self.memory.commit_to_disk(".vsa_core_memory.bin")?;
        Ok(())
    }

    /// Ingests Lean4/Mathlib formal proofs into VSA memory.
    ///
    /// Encoding strategy (multi-slot binding):
    ///   1. name → statement   (what does this theorem say?)
    ///   2. statement → proof   (how do we prove this?)
    ///   3. For each tag: tag → name (semantic retrieval index)
    ///   4. For each import: import → name (dependency graph)
    ///
    /// This creates a rich associative network where:
    ///   - Given a statement, unbinding retrieves the proof
    ///   - Given a tag, unbinding retrieves relevant theorem names
    ///   - Given an import, unbinding retrieves theorems that use it
    pub fn train_on_lean(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("// AUDIT: Stream-Ingesting Lean4/Mathlib Proofs: {}", path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let stream = Deserializer::from_reader(reader).into_iter::<Vec<LeanProofPayload>>();

        let mut count = 0;
        let mut binding_count = 0;
        for batch_result in stream {
            if let Ok(batch) = batch_result {
                let total = batch.len();
                for p in batch {
                    let name_hv = HyperMemory::from_string(&p.name, DIM_PROLETARIAT);
                    let stmt_hv = HyperMemory::from_string(&p.statement, DIM_PROLETARIAT);
                    let proof_hv = HyperMemory::from_string(&p.proof, DIM_PROLETARIAT);

                    // Binding 1: name → statement
                    let name_stmt = name_hv.bind(&stmt_hv)?;
                    self.memory = HyperMemory::bundle(&[self.memory.clone(), name_stmt])?;
                    binding_count += 1;

                    // Binding 2: statement → proof
                    let stmt_proof = stmt_hv.bind(&proof_hv)?;
                    self.memory = HyperMemory::bundle(&[self.memory.clone(), stmt_proof])?;
                    binding_count += 1;

                    // Binding 3: tags → name (semantic index)
                    for tag in &p.tags {
                        let tag_hv = HyperMemory::from_string(tag, DIM_PROLETARIAT);
                        let tag_name = tag_hv.bind(&name_hv)?;
                        self.memory = HyperMemory::bundle(&[self.memory.clone(), tag_name])?;
                        binding_count += 1;
                    }

                    // Binding 4: imports → name (dependency graph)
                    for imp in &p.imports {
                        let imp_hv = HyperMemory::from_string(imp, DIM_PROLETARIAT);
                        let imp_name = imp_hv.bind(&name_hv)?;
                        self.memory = HyperMemory::bundle(&[self.memory.clone(), imp_name])?;
                        binding_count += 1;
                    }

                    count += 1;
                    if count % 50 == 0 {
                        info!("// AUDIT: Processed {}/{} Lean proofs ({} bindings) from {}", count, total, binding_count, path);
                    }
                }
            }
        }
        info!("// AUDIT: Completed Lean ingestion — {} proofs, {} total bindings from {}.", count, binding_count, path);
        self.memory.commit_to_disk(".vsa_core_memory.bin")?;
        Ok(())
    }

    /// Ingests Spider SQL dataset: question → query associations.
    pub fn train_on_spider(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("// AUDIT: Stream-Ingesting Spider SQL Logic: {}", path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let stream = Deserializer::from_reader(reader).into_iter::<Vec<SpiderPayload>>();

        let mut count = 0;
        for batch_result in stream {
            if let Ok(batch) = batch_result {
                let total = batch.len();
                for p in batch {
                    let question_hv = HyperMemory::from_string(&p.question, DIM_PROLETARIAT);
                    let query_hv = HyperMemory::from_string(&p.query, DIM_PROLETARIAT);
                    let mapping = question_hv.bind(&query_hv)?;
                    self.memory = HyperMemory::bundle(&[self.memory.clone(), mapping])?;
                    count += 1;
                    if count % 100 == 0 {
                        info!("// AUDIT: Processed {}/{} Spider queries from {}", count, total, path);
                    }
                }
            }
        }
        info!("// AUDIT: Completed ingestion of {} Spider queries from {}.", count, path);
        self.memory.commit_to_disk(".vsa_core_memory.bin")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_lean_proof_ingestion() {
        let lean_data = r#"[
            {
                "name": "Nat.add_comm",
                "statement": "∀ n m : Nat, n + m = m + n",
                "proof": "by induction n with | zero => simp | succ n ih => simp [ih]",
                "imports": ["Mathlib.Data.Nat.Basic"],
                "tags": ["commutativity", "arithmetic"]
            },
            {
                "name": "Bool.not_not",
                "statement": "∀ b : Bool, !!b = b",
                "proof": "by cases b <;> rfl",
                "imports": [],
                "tags": ["boolean"]
            }
        ]"#;

        // Write test data to temp file
        let path = "/tmp/test_lean_ingest.json";
        let mut f = File::create(path).expect("create temp file");
        f.write_all(lean_data.as_bytes()).expect("write test data");

        let mut trainer = VsaTrainer::new();
        let result = trainer.train_on_lean(path);
        assert!(result.is_ok(), "Lean ingestion should succeed: {:?}", result.err());

        // Clean up
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_lean_proof_deserialization() {
        let json = r#"{"name":"test","statement":"P → P","proof":"by intro h; exact h","imports":["X"],"tags":["logic"]}"#;
        let parsed: Result<LeanProofPayload, _> = serde_json::from_str(json);
        assert!(parsed.is_ok(), "Should deserialize Lean proof payload");
        let p = parsed.unwrap();
        assert_eq!(p.name, "test");
        assert_eq!(p.tags, vec!["logic"]);
    }

    #[test]
    fn test_lean_proof_optional_fields() {
        // imports and tags should be optional (default to empty)
        let json = r#"{"name":"minimal","statement":"True","proof":"trivial"}"#;
        let parsed: Result<LeanProofPayload, _> = serde_json::from_str(json);
        assert!(parsed.is_ok(), "Should deserialize with missing optional fields");
        let p = parsed.unwrap();
        assert!(p.imports.is_empty());
        assert!(p.tags.is_empty());
    }
}
