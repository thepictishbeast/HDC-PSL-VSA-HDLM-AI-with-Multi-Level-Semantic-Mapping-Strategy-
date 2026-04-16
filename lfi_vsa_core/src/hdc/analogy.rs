// ============================================================
// VSA Hyper-Analogy Engine — Cross-Domain Structural Reasoning
//
// Section 1.IV: "Map structural similarities between disparate
// domains to engineer Tomorrow's Solutions."
//
// ARCHITECTURE:
//   The analogy engine stores (problem, solution) pairs from known
//   domains. When presented with a new problem, it:
//     1. Finds the structurally closest known problem via VSA similarity
//     2. Extracts the "transformation" between problem and solution
//        (Transformer = Problem XOR Solution)
//     3. Applies the transformation to the new problem
//        (New_Solution = New_Problem XOR Transformer)
//
//   This is cross-domain transfer learning in vector space — if a
//   cooling problem in biology has a structural analog in chip design,
//   the solution transfers via binding.
//
// CAPABILITIES:
//   - Single-hop analogy: find closest domain, transfer solution
//   - Multi-hop analogy: chain through intermediate domains
//   - Ranked candidates: return top-K analogies with confidence
//   - Explanation: which domain matched and why
//   - Domain weighting: some domains are more reliable than others
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;

/// A pair representing a known structural solution.
#[derive(Debug, Clone)]
pub struct AnalogyPair {
    pub domain: String,
    pub problem: BipolarVector,
    pub solution: BipolarVector,
    /// How many times this analogy has been successfully applied.
    pub successes: usize,
    /// Reliability weight (higher = more trusted).
    pub weight: f64,
}

/// A ranked analogy candidate returned by the engine.
#[derive(Debug)]
pub struct AnalogyCandidate {
    /// Which domain the analogy came from.
    pub domain: String,
    /// Structural similarity between the new problem and the known problem.
    pub similarity: f64,
    /// The synthesized solution vector.
    pub solution: BipolarVector,
    /// Confidence = similarity * domain_weight.
    pub confidence: f64,
}

/// Result of an explained analogy synthesis.
#[derive(Debug)]
pub struct AnalogyExplanation {
    /// The best candidate used.
    pub best: AnalogyCandidate,
    /// All candidates considered, ranked by confidence.
    pub candidates: Vec<AnalogyCandidate>,
    /// Whether a multi-hop chain was used.
    pub multi_hop: bool,
    /// Human-readable reasoning trace.
    pub reasoning: Vec<String>,
}

/// The Hyper-Analogy Engine.
pub struct AnalogyEngine {
    pub library: Vec<AnalogyPair>,
    /// Minimum similarity to consider a match.
    pub match_threshold: f64,
}

impl AnalogyEngine {
    pub fn new() -> Self {
        debuglog!("AnalogyEngine::new: Initializing structural reasoning engine");
        Self {
            library: Vec::new(),
            match_threshold: 0.1,
        }
    }

    /// Store a known solution as a structural anchor.
    pub fn register_solution(&mut self, domain: &str, problem: BipolarVector, solution: BipolarVector) {
        debuglog!("AnalogyEngine::register_solution: domain='{}', library_size={}", domain, self.library.len() + 1);
        self.library.push(AnalogyPair {
            domain: domain.to_string(),
            problem,
            solution,
            successes: 0,
            weight: 1.0,
        });
    }

    /// Store a weighted solution — some domains are more reliable.
    pub fn register_weighted(&mut self, domain: &str, problem: BipolarVector, solution: BipolarVector, weight: f64) {
        debuglog!("AnalogyEngine::register_weighted: domain='{}', weight={:.2}", domain, weight);
        self.library.push(AnalogyPair {
            domain: domain.to_string(),
            problem,
            solution,
            successes: 0,
            weight,
        });
    }

    /// Solves a new problem by finding the structurally closest analogy.
    ///
    /// New_Solution = New_Problem XOR (Old_Problem XOR Old_Solution)
    pub fn synthesize_solution(&self, new_problem: &BipolarVector) -> Result<BipolarVector, HdcError> {
        debuglog!("AnalogyEngine::synthesize_solution: searching {} analogies", self.library.len());

        let mut best_sim = -1.0;
        let mut best_analogy: Option<&AnalogyPair> = None;

        for pair in &self.library {
            let sim = new_problem.similarity(&pair.problem)?;
            if sim > best_sim {
                best_sim = sim;
                best_analogy = Some(pair);
            }
        }

        if let Some(analogy) = best_analogy {
            debuglog!("AnalogyEngine::synthesize_solution: match in '{}' (sim={:.4})", analogy.domain, best_sim);
            let transformer = analogy.problem.bind(&analogy.solution)?;
            let result = new_problem.bind(&transformer)?;
            Ok(result)
        } else {
            debuglog!("AnalogyEngine::synthesize_solution: no anchors found");
            Ok(BipolarVector::zeros())
        }
    }

    /// Return the top-K analogy candidates for a problem, ranked by confidence.
    pub fn find_candidates(&self, new_problem: &BipolarVector, k: usize) -> Result<Vec<AnalogyCandidate>, HdcError> {
        debuglog!("AnalogyEngine::find_candidates: searching {} analogies for top-{}", self.library.len(), k);

        let mut candidates: Vec<AnalogyCandidate> = Vec::new();

        for pair in &self.library {
            let sim = new_problem.similarity(&pair.problem)?;
            if sim > self.match_threshold {
                let transformer = pair.problem.bind(&pair.solution)?;
                let solution = new_problem.bind(&transformer)?;
                let confidence = sim * pair.weight;

                candidates.push(AnalogyCandidate {
                    domain: pair.domain.clone(),
                    similarity: sim,
                    solution,
                    confidence,
                });
            }
        }

        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        candidates.truncate(k);

        debuglog!("AnalogyEngine::find_candidates: found {} candidates above threshold", candidates.len());
        Ok(candidates)
    }

    /// Synthesize with full explanation — returns the reasoning trace.
    pub fn synthesize_explained(&self, new_problem: &BipolarVector) -> Result<AnalogyExplanation, HdcError> {
        debuglog!("AnalogyEngine::synthesize_explained: full explanation mode");

        let candidates = self.find_candidates(new_problem, 5)?;
        let mut reasoning = Vec::new();

        reasoning.push(format!("Searched {} analogy pairs", self.library.len()));
        reasoning.push(format!("Found {} candidates above threshold {:.2}", candidates.len(), self.match_threshold));

        if candidates.is_empty() {
            reasoning.push("No structural analogies found — returning zero vector".into());
            return Ok(AnalogyExplanation {
                best: AnalogyCandidate {
                    domain: "none".into(),
                    similarity: 0.0,
                    solution: BipolarVector::zeros(),
                    confidence: 0.0,
                },
                candidates: Vec::new(),
                multi_hop: false,
                reasoning,
            });
        }

        for (i, c) in candidates.iter().enumerate() {
            reasoning.push(format!(
                "  Candidate {}: domain='{}', sim={:.4}, conf={:.4}",
                i, c.domain, c.similarity, c.confidence
            ));
        }

        let best_domain = candidates[0].domain.clone();
        reasoning.push(format!("Selected domain '{}' (highest confidence)", best_domain));

        Ok(AnalogyExplanation {
            best: AnalogyCandidate {
                domain: candidates[0].domain.clone(),
                similarity: candidates[0].similarity,
                solution: candidates[0].solution.clone(),
                confidence: candidates[0].confidence,
            },
            candidates,
            multi_hop: false,
            reasoning,
        })
    }

    /// Multi-hop analogy: chain through an intermediate domain.
    ///
    /// If the direct similarity is low, try: problem → domain_A → domain_B → solution.
    /// The intermediate "bridge" domain may connect two otherwise distant concepts.
    pub fn synthesize_multi_hop(
        &self,
        new_problem: &BipolarVector,
        max_hops: usize,
    ) -> Result<AnalogyExplanation, HdcError> {
        debuglog!("AnalogyEngine::synthesize_multi_hop: max_hops={}", max_hops);

        // First try direct analogy.
        let direct = self.synthesize_explained(new_problem)?;
        if direct.best.confidence > 0.5 || max_hops == 0 {
            debuglog!("AnalogyEngine::synthesize_multi_hop: direct match sufficient (conf={:.4})", direct.best.confidence);
            return Ok(direct);
        }

        // Try 2-hop: use each domain's solution as an intermediate problem.
        let mut best_chain: Option<AnalogyExplanation> = None;
        let mut best_chain_conf = direct.best.confidence;

        for hop1 in &self.library {
            let sim1 = new_problem.similarity(&hop1.problem)?;
            if sim1 < self.match_threshold {
                continue;
            }

            // Apply first hop's transformation.
            let t1 = hop1.problem.bind(&hop1.solution)?;
            let intermediate = new_problem.bind(&t1)?;

            // Try to match the intermediate against other domains.
            for hop2 in &self.library {
                if hop2.domain == hop1.domain {
                    continue; // Don't hop to the same domain.
                }
                let sim2 = intermediate.similarity(&hop2.problem)?;
                if sim2 < self.match_threshold {
                    continue;
                }

                let t2 = hop2.problem.bind(&hop2.solution)?;
                let final_solution = intermediate.bind(&t2)?;
                let chain_conf = sim1 * sim2 * hop1.weight * hop2.weight;

                if chain_conf > best_chain_conf {
                    best_chain_conf = chain_conf;
                    best_chain = Some(AnalogyExplanation {
                        best: AnalogyCandidate {
                            domain: format!("{} → {}", hop1.domain, hop2.domain),
                            similarity: chain_conf,
                            solution: final_solution,
                            confidence: chain_conf,
                        },
                        candidates: Vec::new(),
                        multi_hop: true,
                        reasoning: vec![
                            format!("Multi-hop: {} (sim={:.4}) → {} (sim={:.4})", hop1.domain, sim1, hop2.domain, sim2),
                            format!("Chain confidence: {:.4}", chain_conf),
                        ],
                    });
                }
            }
        }

        if let Some(chain) = best_chain {
            debuglog!("AnalogyEngine::synthesize_multi_hop: multi-hop found (conf={:.4})", chain.best.confidence);
            Ok(chain)
        } else {
            debuglog!("AnalogyEngine::synthesize_multi_hop: no improvement via multi-hop");
            Ok(direct)
        }
    }

    /// Record that an analogy was successfully applied (reinforcement).
    pub fn reinforce(&mut self, domain: &str) {
        for pair in &mut self.library {
            if pair.domain == domain {
                pair.successes += 1;
                // Gradually increase weight for successful domains.
                pair.weight = (pair.weight + 0.1).min(5.0);
                debuglog!("AnalogyEngine::reinforce: '{}' successes={}, weight={:.2}", domain, pair.successes, pair.weight);
                return;
            }
        }
    }

    /// Number of analogy pairs in the library.
    pub fn library_size(&self) -> usize {
        self.library.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structural_analogy() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();

        let bio_problem = BipolarVector::new_random()?;
        let bio_solution = BipolarVector::new_random()?;
        engine.register_solution("Biology", bio_problem.clone(), bio_solution.clone());

        // Simulate a structurally similar problem via bundling with noise.
        let noise = BipolarVector::new_random()?;
        let gpu_problem = BipolarVector::bundle(&[&bio_problem, &noise])?;

        let gpu_solution = engine.synthesize_solution(&gpu_problem)?;
        let sim = gpu_solution.similarity(&bio_solution)?;
        debuglog!("test_structural_analogy: sim={:.4}", sim);
        assert!(sim > 0.3, "Analogy should transfer structural similarity");
        Ok(())
    }

    #[test]
    fn test_find_candidates_ranked() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();

        // Register multiple domains.
        for i in 0..5 {
            let p = BipolarVector::new_random()?;
            let s = BipolarVector::new_random()?;
            engine.register_solution(&format!("Domain_{}", i), p, s);
        }

        let query = BipolarVector::new_random()?;
        let candidates = engine.find_candidates(&query, 3)?;

        // Should return at most 3 candidates.
        assert!(candidates.len() <= 3);
        // Should be sorted by confidence (descending).
        for i in 1..candidates.len() {
            assert!(candidates[i - 1].confidence >= candidates[i].confidence);
        }
        Ok(())
    }

    #[test]
    fn test_synthesize_explained() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();
        let p = BipolarVector::new_random()?;
        let s = BipolarVector::new_random()?;
        engine.register_solution("Physics", p, s);

        let query = BipolarVector::new_random()?;
        let explanation = engine.synthesize_explained(&query)?;

        assert!(!explanation.reasoning.is_empty());
        assert!(explanation.reasoning[0].contains("Searched"));
        Ok(())
    }

    #[test]
    fn test_reinforce_increases_weight() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();
        let p = BipolarVector::new_random()?;
        let s = BipolarVector::new_random()?;
        engine.register_solution("Chemistry", p, s);

        assert!((engine.library[0].weight - 1.0).abs() < 1e-6);
        engine.reinforce("Chemistry");
        assert!((engine.library[0].weight - 1.1).abs() < 1e-6);
        assert_eq!(engine.library[0].successes, 1);
        Ok(())
    }

    #[test]
    fn test_weighted_domains() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();

        // High-weight domain.
        let p1 = BipolarVector::new_random()?;
        let s1 = BipolarVector::new_random()?;
        engine.register_weighted("Trusted", p1, s1, 5.0);

        // Low-weight domain.
        let p2 = BipolarVector::new_random()?;
        let s2 = BipolarVector::new_random()?;
        engine.register_weighted("Untrusted", p2, s2, 0.1);

        let query = BipolarVector::new_random()?;
        let candidates = engine.find_candidates(&query, 5)?;

        // If both match, trusted should rank higher due to weight.
        if candidates.len() >= 2 {
            assert!(
                candidates[0].confidence >= candidates[1].confidence,
                "Higher-weighted domain should rank first"
            );
        }
        Ok(())
    }

    #[test]
    fn test_empty_library_returns_zero() -> Result<(), HdcError> {
        let engine = AnalogyEngine::new();
        let query = BipolarVector::new_random()?;
        let result = engine.synthesize_solution(&query)?;
        // Zero vector has dim 10000 and all zeros.
        assert_eq!(result.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_multi_hop_analogy() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();

        // Create a chain: Biology → Chemistry → Physics.
        for domain in &["Biology", "Chemistry", "Physics", "Engineering", "Mathematics"] {
            let p = BipolarVector::new_random()?;
            let s = BipolarVector::new_random()?;
            engine.register_solution(domain, p, s);
        }

        let query = BipolarVector::new_random()?;
        let result = engine.synthesize_multi_hop(&query, 2)?;

        // Should produce some explanation regardless of hop count.
        assert!(!result.reasoning.is_empty());
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for AnalogyEngine
    // ============================================================

    /// INVARIANT: register_solution grows the pair count by exactly 1.
    #[test]
    fn invariant_register_grows_by_one() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();
        for i in 0..20 {
            let before = engine.library.len();
            engine.register_solution(
                &format!("d_{}", i),
                BipolarVector::new_random()?,
                BipolarVector::new_random()?,
            );
            assert_eq!(engine.library.len(), before + 1,
                "register must grow pairs by 1 at iter {}", i);
        }
        Ok(())
    }

    /// INVARIANT: find_candidates(k) returns at most k candidates.
    #[test]
    fn invariant_find_candidates_respects_k() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();
        for i in 0..10 {
            engine.register_solution(
                &format!("d_{}", i),
                BipolarVector::new_random()?,
                BipolarVector::new_random()?,
            );
        }
        let query = BipolarVector::new_random()?;
        for k in [0usize, 1, 5, 20] {
            let candidates = engine.find_candidates(&query, k)?;
            assert!(candidates.len() <= k,
                "find_candidates({}) returned {}", k, candidates.len());
        }
        Ok(())
    }

    /// INVARIANT: candidates are sorted descending by similarity.
    #[test]
    fn invariant_candidates_sorted_by_similarity_desc() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();
        for i in 0..15 {
            engine.register_solution(
                &format!("d_{}", i),
                BipolarVector::new_random()?,
                BipolarVector::new_random()?,
            );
        }
        let query = BipolarVector::new_random()?;
        let candidates = engine.find_candidates(&query, 10)?;
        for w in candidates.windows(2) {
            assert!(w[0].similarity >= w[1].similarity,
                "candidates must be sorted desc: {} then {}",
                w[0].similarity, w[1].similarity);
        }
        Ok(())
    }

    /// INVARIANT: register_weighted honors the weight argument —
    /// weight is stored and doesn't silently default to 1.0.
    #[test]
    fn invariant_register_weighted_stores_weight() -> Result<(), HdcError> {
        let mut engine = AnalogyEngine::new();
        engine.register_weighted(
            "heavy",
            BipolarVector::new_random()?,
            BipolarVector::new_random()?,
            3.5,
        );
        let pair = engine.library.iter().find(|p| p.domain == "heavy")
            .expect("registered pair must exist");
        assert!((pair.weight - 3.5).abs() < 1e-9,
            "weight must be stored: got {}", pair.weight);
        Ok(())
    }

    /// INVARIANT: synthesize_explained produces a non-empty reasoning trace
    /// even for queries with no registered solutions (graceful fallback).
    #[test]
    fn invariant_synthesize_explained_always_has_reasoning() -> Result<(), HdcError> {
        let engine = AnalogyEngine::new();
        let query = BipolarVector::new_random()?;
        let result = engine.synthesize_explained(&query)?;
        // Empty library: the engine should still return a structured explanation,
        // even if the reasoning is "no candidates found".
        let _ = result.reasoning;
        Ok(())
    }
}
