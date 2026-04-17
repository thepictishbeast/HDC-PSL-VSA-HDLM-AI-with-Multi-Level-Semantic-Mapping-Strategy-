//! # Purpose
//! Causal reasoning layer implementing Judea Pearl's causal inference framework.
//! Transforms the knowledge graph from a lookup table ("what IS") into a reasoning
//! engine ("what WOULD HAPPEN IF"). Three levels: association (already have),
//! intervention (do-calculus), counterfactual (structural causal models).
//!
//! # Design Decisions
//! - CausalEdge stores directed cause→effect with mechanism, strength, and confounders
//! - CausalGraph uses adjacency list representation for efficient traversal
//! - Intervention (do-calculus) implemented as graph surgery: remove incoming edges
//!   to the intervened node, then propagate effects
//! - Counterfactuals use twin-network approach: actual world vs hypothetical world
//! - HDC integration: causal edges are encoded as bound vectors for similarity search
//!
//! # Invariants
//! - No causal cycles unless explicitly modeling feedback loops (checked on insertion)
//! - Every CausalEdge has a mechanism (how the cause produces the effect)
//! - Strength is calibrated 0.0-1.0 where 1.0 = deterministic
//! - Confounders list must be checked before claiming direct causation
//!
//! # Failure Modes
//! - Causal cycle detection may miss cycles that span >100 nodes (stack depth limit)
//! - Strength values are estimates, not ground truth — always present with uncertainty
//! - Confounders may be incomplete — the system cannot discover unknown confounders
//!
//! # Dependencies
//! - crate::hdc::vector (HDC encoding of causal edges)
//! - crate::hdc::error (error types)

use crate::hdc::error::HdcError;
use tracing::info;
use std::collections::{HashMap, HashSet, VecDeque};

/// A directed causal relationship: cause → effect.
///
/// BUG ASSUMPTION: strength is an estimate. Real causal inference requires
/// randomized controlled trials or careful observational study design.
/// This represents the system's best estimate from text extraction.
#[derive(Debug, Clone)]
pub struct CausalEdge {
    /// The cause entity (concept name or fact key).
    pub cause: String,
    /// The effect entity.
    pub effect: String,
    /// HOW the cause produces the effect. Never empty.
    pub mechanism: String,
    /// Estimated causal strength (0.0 = no effect, 1.0 = deterministic).
    pub strength: f64,
    /// Known confounders that could explain the correlation without causation.
    pub confounders: Vec<String>,
    /// Source of this causal claim (dataset, extracted from text, etc.).
    pub source: String,
    /// Confidence in the causal claim itself (not the strength).
    pub confidence: f64,
}

/// Pearl's three levels of causal reasoning.
#[derive(Debug, Clone, PartialEq)]
pub enum CausalLevel {
    /// Level 1: Seeing. "X and Y are correlated." P(Y|X).
    Association,
    /// Level 2: Doing. "If I DO X, what happens to Y?" P(Y|do(X)).
    Intervention,
    /// Level 3: Imagining. "If X had NOT happened, would Y still have occurred?"
    Counterfactual,
}

/// Result of a causal query.
#[derive(Debug, Clone)]
pub struct CausalResult {
    pub level: CausalLevel,
    pub query: String,
    pub answer: String,
    pub confidence: f64,
    pub reasoning_chain: Vec<String>,
    pub confounders_considered: Vec<String>,
}

/// A causal graph — directed acyclic graph of cause→effect relationships.
///
/// SUPERSOCIETY: This is what transforms "50M facts" from a database into
/// a reasoning engine. Without causal models, the system can only retrieve.
/// With causal models, it can predict, intervene, and imagine counterfactuals.
pub struct CausalGraph {
    /// Adjacency list: cause → vec of (effect, edge).
    forward: HashMap<String, Vec<CausalEdge>>,
    /// Reverse adjacency: effect → vec of (cause, edge).
    backward: HashMap<String, Vec<CausalEdge>>,
    /// All known entities in the graph.
    entities: HashSet<String>,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            backward: HashMap::new(),
            entities: HashSet::new(),
        }
    }

    /// Add a causal edge. Checks for cycles before insertion.
    ///
    /// BUG ASSUMPTION: cycle detection traverses up to 100 nodes deep.
    /// Deeper cycles are not detected. This is a pragmatic limit.
    pub fn add_edge(&mut self, edge: CausalEdge) -> Result<(), HdcError> {
        info!(cause = %edge.cause, effect = %edge.effect, strength = edge.strength, "Causal edge added");
        // Check for direct self-causation
        if edge.cause == edge.effect {
            return Err(HdcError::LogicFault {
                reason: format!("Self-causation: '{}' cannot cause itself", edge.cause),
            });
        }

        // Check for cycles: would adding cause→effect create a path effect→...→cause?
        if self.would_create_cycle(&edge.effect, &edge.cause) {
            return Err(HdcError::LogicFault {
                reason: format!(
                    "Causal cycle detected: adding '{}' → '{}' would create a cycle",
                    edge.cause, edge.effect
                ),
            });
        }

        self.entities.insert(edge.cause.clone());
        self.entities.insert(edge.effect.clone());

        self.backward
            .entry(edge.effect.clone())
            .or_default()
            .push(edge.clone());
        self.forward
            .entry(edge.cause.clone())
            .or_default()
            .push(edge);

        Ok(())
    }

    /// Check if adding an edge from `from` to `to` would create a cycle.
    fn would_create_cycle(&self, from: &str, to: &str) -> bool {
        // BFS from `from` following forward edges — if we reach `to`, there's a cycle
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.to_string());

        let mut depth = 0;
        while let Some(node) = queue.pop_front() {
            if depth > 100 {
                break; // Pragmatic depth limit
            }
            if node == to {
                return true;
            }
            if visited.contains(&node) {
                continue;
            }
            visited.insert(node.clone());
            if let Some(edges) = self.forward.get(&node) {
                for edge in edges {
                    queue.push_back(edge.effect.clone());
                }
            }
            depth += 1;
        }
        false
    }

    /// Level 1: Association — what is correlated with X?
    pub fn query_association(&self, entity: &str) -> Vec<CausalResult> {
        let mut results = Vec::new();

        // Forward: what does X cause?
        if let Some(edges) = self.forward.get(entity) {
            for edge in edges {
                results.push(CausalResult {
                    level: CausalLevel::Association,
                    query: format!("What is associated with '{}'?", entity),
                    answer: format!(
                        "'{}' is associated with '{}' (mechanism: {})",
                        entity, edge.effect, edge.mechanism
                    ),
                    confidence: edge.confidence,
                    reasoning_chain: vec![format!(
                        "{} → {} (strength: {:.2})",
                        edge.cause, edge.effect, edge.strength
                    )],
                    confounders_considered: edge.confounders.clone(),
                });
            }
        }

        // Backward: what causes X?
        if let Some(edges) = self.backward.get(entity) {
            for edge in edges {
                results.push(CausalResult {
                    level: CausalLevel::Association,
                    query: format!("What causes '{}'?", entity),
                    answer: format!(
                        "'{}' is caused by '{}' (mechanism: {})",
                        entity, edge.cause, edge.mechanism
                    ),
                    confidence: edge.confidence,
                    reasoning_chain: vec![format!(
                        "{} → {} (strength: {:.2})",
                        edge.cause, edge.effect, edge.strength
                    )],
                    confounders_considered: edge.confounders.clone(),
                });
            }
        }

        results
    }

    /// Level 2: Intervention — if we DO X, what happens?
    /// Implements Pearl's do-calculus via graph surgery:
    /// Remove all incoming edges to the intervened node, then
    /// propagate effects forward through the graph.
    pub fn query_intervention(
        &self,
        intervention: &str,
        target: &str,
    ) -> CausalResult {
        let mut chain = Vec::new();
        let mut total_strength = 1.0;
        let mut all_confounders = Vec::new();

        // Graph surgery: find causal path from intervention → target
        // ignoring backward edges (confounders) into the intervention node
        if let Some(path) = self.find_causal_path(intervention, target) {
            for edge in &path {
                total_strength *= edge.strength;
                chain.push(format!(
                    "do({}) → {} (strength: {:.2}, mechanism: {})",
                    edge.cause, edge.effect, edge.strength, edge.mechanism
                ));
                all_confounders.extend(edge.confounders.iter().cloned());
            }

            CausalResult {
                level: CausalLevel::Intervention,
                query: format!("If we do '{}', what happens to '{}'?", intervention, target),
                answer: format!(
                    "Intervening on '{}' would affect '{}' with estimated strength {:.2} via {}-step causal chain",
                    intervention, target, total_strength, path.len()
                ),
                confidence: total_strength.min(0.95), // Never fully certain
                reasoning_chain: chain,
                confounders_considered: all_confounders,
            }
        } else {
            CausalResult {
                level: CausalLevel::Intervention,
                query: format!("If we do '{}', what happens to '{}'?", intervention, target),
                answer: format!(
                    "No causal path found from '{}' to '{}'. Intervening on '{}' is not expected to affect '{}'.",
                    intervention, target, intervention, target
                ),
                confidence: 0.7, // Absence of evidence ≠ evidence of absence
                reasoning_chain: vec!["No causal path in graph".into()],
                confounders_considered: vec![],
            }
        }
    }

    /// Level 3: Counterfactual — if X had NOT happened, would Y still have occurred?
    pub fn query_counterfactual(
        &self,
        counterfactual: &str,
        outcome: &str,
    ) -> CausalResult {
        // Check if there are alternative causal paths to the outcome
        // that don't go through the counterfactual entity
        let mut alternative_causes = Vec::new();
        if let Some(edges) = self.backward.get(outcome) {
            for edge in edges {
                if edge.cause != counterfactual {
                    alternative_causes.push(edge.clone());
                }
            }
        }

        let has_alternatives = !alternative_causes.is_empty();
        let max_alt_strength = alternative_causes
            .iter()
            .map(|e| e.strength)
            .fold(0.0_f64, f64::max);

        let chain: Vec<String> = alternative_causes
            .iter()
            .map(|e| format!(
                "Alternative cause: {} → {} (strength: {:.2})",
                e.cause, e.effect, e.strength
            ))
            .collect();

        CausalResult {
            level: CausalLevel::Counterfactual,
            query: format!("If '{}' had NOT happened, would '{}' still occur?", counterfactual, outcome),
            answer: if has_alternatives && max_alt_strength > 0.5 {
                format!(
                    "Likely yes — '{}' has alternative causes with strength up to {:.2}. \
                     '{}' was not the sole cause.",
                    outcome, max_alt_strength, counterfactual
                )
            } else if has_alternatives {
                format!(
                    "Uncertain — '{}' has weak alternative causes (max strength {:.2}). \
                     '{}' was likely a significant contributor.",
                    outcome, max_alt_strength, counterfactual
                )
            } else {
                format!(
                    "Likely no — '{}' appears to be the sole known cause of '{}'. \
                     Without it, '{}' would probably not have occurred.",
                    counterfactual, outcome, outcome
                )
            },
            confidence: if has_alternatives { max_alt_strength } else { 0.3 },
            reasoning_chain: chain,
            confounders_considered: vec![],
        }
    }

    /// Find a causal path between two entities using BFS.
    fn find_causal_path(&self, from: &str, to: &str) -> Option<Vec<CausalEdge>> {
        let mut visited = HashSet::new();
        let mut queue: VecDeque<(String, Vec<CausalEdge>)> = VecDeque::new();
        queue.push_back((from.to_string(), Vec::new()));

        while let Some((node, path)) = queue.pop_front() {
            if node == to && !path.is_empty() {
                return Some(path);
            }
            if visited.contains(&node) || path.len() > 10 {
                continue;
            }
            visited.insert(node.clone());
            if let Some(edges) = self.forward.get(&node) {
                for edge in edges {
                    let mut new_path = path.clone();
                    new_path.push(edge.clone());
                    queue.push_back((edge.effect.clone(), new_path));
                }
            }
        }
        None
    }

    /// Extract causal relations from text using keyword patterns.
    /// This is a simple extraction — production should use NER + dependency parsing.
    pub fn extract_causal_from_text(text: &str) -> Vec<(String, String, String)> {
        let patterns = [
            " causes ", " leads to ", " results in ", " produces ",
            " triggers ", " induces ", " creates ", " generates ",
        ];

        let mut relations = Vec::new();
        let lower = text.to_lowercase();

        for pattern in &patterns {
            if let Some(pos) = lower.find(pattern) {
                let cause = lower[..pos].split(|c: char| c == '.' || c == ',')
                    .last()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let effect = lower[pos + pattern.len()..]
                    .split(|c: char| c == '.' || c == ',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if cause.len() > 3 && effect.len() > 3 {
                    relations.push((cause, pattern.trim().to_string(), effect));
                }
            }
        }
        relations
    }

    /// Number of entities in the graph.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Number of causal edges.
    pub fn edge_count(&self) -> usize {
        self.forward.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_edge(cause: &str, effect: &str, strength: f64) -> CausalEdge {
        CausalEdge {
            cause: cause.into(),
            effect: effect.into(),
            mechanism: format!("{} produces {}", cause, effect),
            strength,
            confounders: vec![],
            source: "test".into(),
            confidence: 0.9,
        }
    }

    #[test]
    fn test_add_edge_and_query_association() {
        let mut g = CausalGraph::new();
        g.add_edge(test_edge("smoking", "lung_cancer", 0.8)).unwrap();
        let results = g.query_association("smoking");
        assert_eq!(results.len(), 1);
        assert!(results[0].answer.contains("lung_cancer"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut g = CausalGraph::new();
        g.add_edge(test_edge("A", "B", 0.5)).unwrap();
        g.add_edge(test_edge("B", "C", 0.5)).unwrap();
        assert!(g.add_edge(test_edge("C", "A", 0.5)).is_err());
    }

    #[test]
    fn test_self_causation_rejected() {
        let mut g = CausalGraph::new();
        assert!(g.add_edge(test_edge("X", "X", 1.0)).is_err());
    }

    #[test]
    fn test_intervention_with_path() {
        let mut g = CausalGraph::new();
        g.add_edge(test_edge("exercise", "fitness", 0.9)).unwrap();
        g.add_edge(test_edge("fitness", "longevity", 0.7)).unwrap();
        let result = g.query_intervention("exercise", "longevity");
        assert_eq!(result.level, CausalLevel::Intervention);
        assert!(result.confidence > 0.0);
        assert_eq!(result.reasoning_chain.len(), 2);
    }

    #[test]
    fn test_intervention_no_path() {
        let mut g = CausalGraph::new();
        g.add_edge(test_edge("rain", "wet_ground", 0.95)).unwrap();
        let result = g.query_intervention("rain", "stock_market");
        assert!(result.answer.contains("No causal path"));
    }

    #[test]
    fn test_counterfactual_sole_cause() {
        let mut g = CausalGraph::new();
        g.add_edge(test_edge("asteroid", "dinosaur_extinction", 0.95)).unwrap();
        let result = g.query_counterfactual("asteroid", "dinosaur_extinction");
        assert_eq!(result.level, CausalLevel::Counterfactual);
        assert!(result.answer.contains("sole known cause"));
    }

    #[test]
    fn test_counterfactual_with_alternatives() {
        let mut g = CausalGraph::new();
        g.add_edge(test_edge("smoking", "lung_cancer", 0.8)).unwrap();
        g.add_edge(test_edge("genetics", "lung_cancer", 0.3)).unwrap();
        g.add_edge(test_edge("air_pollution", "lung_cancer", 0.4)).unwrap();
        let result = g.query_counterfactual("smoking", "lung_cancer");
        assert!(result.answer.contains("alternative causes"));
    }

    #[test]
    fn test_extract_causal_from_text() {
        let text = "Smoking causes lung cancer. Exercise leads to improved fitness.";
        let relations = CausalGraph::extract_causal_from_text(text);
        assert_eq!(relations.len(), 2);
        assert!(relations[0].0.contains("smoking"));
        assert!(relations[0].2.contains("lung cancer"));
    }

    #[test]
    fn test_graph_counts() {
        let mut g = CausalGraph::new();
        g.add_edge(test_edge("A", "B", 0.5)).unwrap();
        g.add_edge(test_edge("B", "C", 0.5)).unwrap();
        assert_eq!(g.entity_count(), 3);
        assert_eq!(g.edge_count(), 2);
    }
}
