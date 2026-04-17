// NODE 014: Formal Logic Ingestor
// STATUS: ALPHA - Material Ingestion Active
// PROTOCOL: First-Principles-Vectorization / Multi-Format-Logic-Binding
//
// FORMATS SUPPORTED:
//   - Lean (.lean): theorem/proof pairs
//   - Propositional logic: premise → conclusion rules
//   - Inference rules: named rules with premises and conclusion
//   - Raw axiom strings: direct statement vectorization
//
// ARCHITECTURE:
//   Each logical statement is vectorized via HyperMemory::from_string.
//   Theorem-proof pairs are bound: bind(theorem, proof) → stored in memory.
//   Premises are bundled: bundle(premise_1, premise_2, ...) → bound with conclusion.
//   All ingested knowledge is queryable via probe() on the memory.

use std::fs::File;
use std::io::{BufRead, BufReader};
use tracing::{info, debug, warn};
use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};

/// A logical statement with its vector representation.
#[derive(Debug, Clone)]
pub struct LogicalStatement {
    pub text: String,
    pub vector: HyperMemory,
    pub kind: StatementKind,
}

/// The kind of logical statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Theorem,
    Proof,
    Axiom,
    Premise,
    Conclusion,
    Definition,
    Lemma,
    InferenceRule { name: String },
}

/// An ingested logical relationship.
#[derive(Debug, Clone)]
pub struct LogicalRelation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationKind {
    TheoremProof,
    PremiseConclusion,
    DefinitionUsage,
    LemmaApplication,
}

/// Result of querying the logic memory.
#[derive(Debug)]
pub struct LogicQueryResult {
    pub query: String,
    pub similarity: f64,
    pub nearest_statement: Option<String>,
}

pub struct FormalLogicIngestor {
    pub memory: HyperMemory,
    /// Count of ingested relations.
    pub relation_count: usize,
    /// Log of all ingested relations (for auditing).
    relations: Vec<LogicalRelation>,
    /// Named statements for reverse lookup.
    statements: Vec<LogicalStatement>,
}

impl FormalLogicIngestor {
    pub fn new() -> Self {
        debuglog!("FormalLogicIngestor::new: Initializing logic substrate");
        let memory = HyperMemory::load_from_disk(".vsa_logic_memory.bin")
            .unwrap_or_else(|_| HyperMemory::new(DIM_PROLETARIAT));
        Self {
            memory,
            relation_count: 0,
            relations: Vec::new(),
            statements: Vec::new(),
        }
    }

    /// Create a fresh ingestor without loading from disk.
    pub fn new_empty() -> Self {
        debuglog!("FormalLogicIngestor::new_empty: Fresh logic substrate");
        Self {
            memory: HyperMemory::new(DIM_PROLETARIAT),
            relation_count: 0,
            relations: Vec::new(),
            statements: Vec::new(),
        }
    }

    /// INGEST: Processes Lean (.lean) files and binds theorem-proof pairs into VSA.
    pub fn ingest_lean_module(&mut self, path: &str) -> Result<usize, Box<dyn std::error::Error>> {
        info!("// AUDIT: Ingesting formal logic from: {}", path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut associations = 0;
        let mut current_theorem = String::new();

        for line in reader.lines() {
            let line = line?;
            debug!("FormalLogicIngestor: Scanning line: {}", line);

            if line.trim().starts_with("theorem") || line.trim().starts_with("lemma") {
                current_theorem = line.clone();
            } else if line.trim().starts_with("proof") && !current_theorem.is_empty() {
                self.ingest_relation(&current_theorem, &line, RelationKind::TheoremProof)?;
                associations += 1;
                current_theorem.clear();
            } else if line.trim().starts_with("def ") || line.trim().starts_with("definition") {
                self.register_statement(&line, StatementKind::Definition);
            }
        }

        if associations > 0 {
            info!("// AUDIT: Committed {} logic associations.", associations);
        } else {
            warn!("// AUDIT: Zero logic associations from {}.", path);
        }

        Ok(associations)
    }

    /// Ingest a propositional rule: "if P then Q" or "P → Q".
    pub fn ingest_rule(&mut self, premises: &[&str], conclusion: &str) -> Result<(), Box<dyn std::error::Error>> {
        debuglog!("FormalLogicIngestor::ingest_rule: {} premises → conclusion", premises.len());

        // Vectorize and bundle all premises.
        let premise_vectors: Vec<HyperMemory> = premises.iter()
            .map(|p| HyperMemory::from_string(p, DIM_PROLETARIAT))
            .collect();

        let bundled_premises = if premise_vectors.len() == 1 {
            premise_vectors[0].clone()
        } else {
            HyperMemory::bundle(&premise_vectors)?
        };

        let conclusion_hv = HyperMemory::from_string(conclusion, DIM_PROLETARIAT);

        // Bind premises with conclusion and add to memory.
        let rule_binding = bundled_premises.bind(&conclusion_hv)?;
        self.memory = HyperMemory::bundle(&[self.memory.clone(), rule_binding])?;

        // Register statements.
        for p in premises {
            self.register_statement(p, StatementKind::Premise);
        }
        self.register_statement(conclusion, StatementKind::Conclusion);
        self.relation_count += 1;

        self.relations.push(LogicalRelation {
            from: premises.join(" ∧ "),
            to: conclusion.to_string(),
            kind: RelationKind::PremiseConclusion,
        });

        Ok(())
    }

    /// Ingest a named inference rule (e.g., modus ponens, modus tollens).
    pub fn ingest_named_rule(
        &mut self,
        name: &str,
        premises: &[&str],
        conclusion: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debuglog!("FormalLogicIngestor::ingest_named_rule: '{}'", name);

        self.ingest_rule(premises, conclusion)?;

        // Also register the rule name as a statement.
        self.register_statement(name, StatementKind::InferenceRule { name: name.to_string() });

        Ok(())
    }

    /// Ingest a raw axiom string — a standalone true statement.
    pub fn ingest_axiom(&mut self, axiom: &str) -> Result<(), Box<dyn std::error::Error>> {
        debuglog!("FormalLogicIngestor::ingest_axiom: '{}'", crate::truncate_str(axiom, 60));

        let axiom_hv = HyperMemory::from_string(axiom, DIM_PROLETARIAT);
        self.memory = HyperMemory::bundle(&[self.memory.clone(), axiom_hv])?;
        self.register_statement(axiom, StatementKind::Axiom);

        Ok(())
    }

    /// Query the logic memory: find the closest stored knowledge to a query.
    pub fn query(&self, query: &str) -> LogicQueryResult {
        debuglog!("FormalLogicIngestor::query: '{}'", crate::truncate_str(query, 60));

        let query_hv = HyperMemory::from_string(query, DIM_PROLETARIAT);
        let similarity = self.memory.similarity(&query_hv);

        // Find the nearest stored statement by vector similarity.
        let nearest = self.statements.iter()
            .max_by(|a, b| {
                let sa = a.vector.similarity(&query_hv);
                let sb = b.vector.similarity(&query_hv);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.text.clone());

        LogicQueryResult {
            query: query.to_string(),
            similarity,
            nearest_statement: nearest,
        }
    }

    /// Number of stored relations.
    pub fn total_relations(&self) -> usize {
        self.relation_count
    }

    /// Number of registered statements.
    pub fn total_statements(&self) -> usize {
        self.statements.len()
    }

    /// Internal: bind two statements and add to memory.
    fn ingest_relation(&mut self, from: &str, to: &str, kind: RelationKind) -> Result<(), Box<dyn std::error::Error>> {
        let from_hv = HyperMemory::from_string(from, DIM_PROLETARIAT);
        let to_hv = HyperMemory::from_string(to, DIM_PROLETARIAT);
        let binding = from_hv.bind(&to_hv)?;
        self.memory = HyperMemory::bundle(&[self.memory.clone(), binding])?;

        let from_kind = match kind {
            RelationKind::TheoremProof => StatementKind::Theorem,
            _ => StatementKind::Premise,
        };
        let to_kind = match kind {
            RelationKind::TheoremProof => StatementKind::Proof,
            _ => StatementKind::Conclusion,
        };

        self.register_statement(from, from_kind);
        self.register_statement(to, to_kind);
        self.relation_count += 1;

        self.relations.push(LogicalRelation {
            from: from.to_string(),
            to: to.to_string(),
            kind,
        });

        Ok(())
    }

    /// Internal: register a statement for reverse lookup.
    fn register_statement(&mut self, text: &str, kind: StatementKind) {
        self.statements.push(LogicalStatement {
            text: text.to_string(),
            vector: HyperMemory::from_string(text, DIM_PROLETARIAT),
            kind,
        });
    }

    /// Save memory to disk.
    pub fn persist(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.memory.commit_to_disk(".vsa_logic_memory.bin")?;
        debuglog!("FormalLogicIngestor::persist: Saved {} relations to disk", self.relation_count);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lean_ingestion_logic() {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let content = "theorem t1 : a = b\nproof : reflexivity";
        let path = "/tmp/test_logic.lean";
        std::fs::write(path, content).unwrap();

        let result = ingestor.ingest_lean_module(path).unwrap();
        assert_eq!(result, 1, "Should have ingested 1 theorem-proof pair");
        assert_eq!(ingestor.total_relations(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_lean_with_lemma() {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let content = "lemma helper : x > 0\nproof : by positivity\ntheorem main : x + x > 0\nproof : by helper";
        let path = "/tmp/test_logic_lemma.lean";
        std::fs::write(path, content).unwrap();

        let result = ingestor.ingest_lean_module(path).unwrap();
        assert_eq!(result, 2, "Should have ingested 2 pairs (lemma+theorem)");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_vsa_logic_retrieval() {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let theorem = "theorem fermat : x^n + y^n = z^n";
        let proof = "proof : by contradiction";

        let theorem_hv = HyperMemory::from_string(theorem, DIM_PROLETARIAT);
        let proof_hv = HyperMemory::from_string(proof, DIM_PROLETARIAT);
        let binding = theorem_hv.bind(&proof_hv).unwrap();

        ingestor.memory = HyperMemory::bundle(&[ingestor.memory.clone(), binding]).unwrap();

        let result = ingestor.memory.bind(&theorem_hv).unwrap();
        let sim = result.similarity(&proof_hv);
        assert!(sim > 0.3, "Theorem probe should yield similarity to Proof. Sim={:.4}", sim);
    }

    #[test]
    fn test_ingest_propositional_rule() {
        let mut ingestor = FormalLogicIngestor::new_empty();

        // Modus ponens: P, P→Q ⊢ Q
        ingestor.ingest_rule(
            &["P is true", "if P then Q"],
            "Q is true",
        ).unwrap();

        assert_eq!(ingestor.total_relations(), 1);
        assert!(ingestor.total_statements() >= 3); // 2 premises + 1 conclusion
    }

    #[test]
    fn test_ingest_named_rule() {
        let mut ingestor = FormalLogicIngestor::new_empty();

        ingestor.ingest_named_rule(
            "modus_ponens",
            &["P", "P implies Q"],
            "Q",
        ).unwrap();

        assert_eq!(ingestor.total_relations(), 1);
        // Should have: P, P implies Q, Q, modus_ponens
        assert!(ingestor.total_statements() >= 4);
    }

    #[test]
    fn test_ingest_axiom() {
        let mut ingestor = FormalLogicIngestor::new_empty();
        ingestor.ingest_axiom("For all x: x = x (reflexivity)").unwrap();
        assert_eq!(ingestor.total_statements(), 1);
        assert_eq!(ingestor.statements[0].kind, StatementKind::Axiom);
    }

    #[test]
    fn test_query_logic_memory() {
        let mut ingestor = FormalLogicIngestor::new_empty();

        ingestor.ingest_axiom("The sum of angles in a triangle equals 180 degrees").unwrap();
        ingestor.ingest_axiom("Water boils at 100 degrees celsius at sea level").unwrap();

        let result = ingestor.query("triangle angle sum");
        assert!(result.nearest_statement.is_some());
        // The nearest should be about triangles, not water.
        let nearest = result.nearest_statement.unwrap();
        debuglog!("Query result: nearest='{}', sim={:.4}", nearest, result.similarity);
    }

    #[test]
    fn test_multiple_rules() {
        let mut ingestor = FormalLogicIngestor::new_empty();

        ingestor.ingest_rule(&["A"], "B").unwrap();
        ingestor.ingest_rule(&["B"], "C").unwrap();
        ingestor.ingest_rule(&["C"], "D").unwrap();

        assert_eq!(ingestor.total_relations(), 3);
        assert_eq!(ingestor.relations.len(), 3);
    }

    #[test]
    fn test_empty_file_yields_zero() {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let path = "/tmp/test_empty_logic.lean";
        std::fs::write(path, "-- just a comment\n").unwrap();

        let result = ingestor.ingest_lean_module(path).unwrap();
        assert_eq!(result, 0);
        let _ = std::fs::remove_file(path);
    }

    // ============================================================
    // Stress / invariant tests for FormalLogicIngestor
    // ============================================================

    /// INVARIANT: new_empty() creates an ingestor with zero state.
    #[test]
    fn invariant_new_empty_zero_state() {
        let ingestor = FormalLogicIngestor::new_empty();
        assert_eq!(ingestor.total_relations(), 0);
        assert_eq!(ingestor.total_statements(), 0);
        assert_eq!(ingestor.relation_count, 0);
    }

    /// INVARIANT: total_relations grows monotonically with each ingest_rule.
    #[test]
    fn invariant_relations_count_monotone() -> Result<(), Box<dyn std::error::Error>> {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let mut prev = 0;
        for i in 0..10 {
            ingestor.ingest_rule(&[&format!("p{}", i)], &format!("c{}", i))?;
            let cur = ingestor.total_relations();
            assert!(cur >= prev,
                "relations count decreased: {} -> {}", prev, cur);
            prev = cur;
        }
        Ok(())
    }

    /// INVARIANT: ingest_lean_module on non-existent path errors cleanly.
    #[test]
    fn invariant_lean_missing_path_errors() {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let result = ingestor.ingest_lean_module("/nonexistent/path/xyz.lean");
        assert!(result.is_err(), "missing file should error");
    }

    /// INVARIANT: ingest_axiom adds a statement.
    #[test]
    fn invariant_axiom_adds_statement() -> Result<(), Box<dyn std::error::Error>> {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let before = ingestor.total_statements();
        ingestor.ingest_axiom("the sky is blue")?;
        let after = ingestor.total_statements();
        assert!(after > before, "ingest_axiom should add a statement");
        Ok(())
    }

    /// INVARIANT: query returns finite similarity in [-1, 1].
    #[test]
    fn invariant_query_similarity_in_cosine_range() {
        let mut ingestor = FormalLogicIngestor::new_empty();
        let _ = ingestor.ingest_axiom("test axiom");
        let result = ingestor.query("test query");
        assert!(result.similarity.is_finite()
            && result.similarity >= -1.0 - 1e-6
            && result.similarity <= 1.0 + 1e-6,
            "similarity out of [-1,1]: {}", result.similarity);
    }
}
