// NODE 017: VSA-Driven Monte Carlo Tree Search (MCTS)
// STATUS: ALPHA - System 2 Deliberation Active
// PROTOCOL: Test-Time Compute / Branching-Logic-Synthesis
//
// ARCHITECTURE:
//   - SELECT:  UCB1 traversal down the tree to find a leaf node
//   - EXPAND:  Apply a semantic action operator (not random noise) to create a child
//   - SIMULATE: Audit the ACTUAL expanded state against PSL + goal similarity
//   - BACKPROP: Propagate the reward signal back up through ancestors
//
// SEMANTIC ACTIONS:
//   Instead of binding with random noise (which explores randomly),
//   we use structured VSA operators that represent meaningful cognitive moves:
//     Decompose:  permute(state, 1) — shift perspective / break into sub-problems
//     Specialize: bind(state, goal) — narrow toward the goal
//     Generalize: bundle(state, random) — widen the search space
//     Contrast:   bind(state, permute(state, 3)) — explore the negation/opposite
//
// Each expand() cycles through these actions, ensuring systematic exploration.

use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use crate::psl::supervisor::PslSupervisor;
use crate::psl::axiom::AuditTarget;
use crate::hdc::vector::BipolarVector;
use tracing::info;

/// Semantic action operators for structured MCTS expansion.
#[derive(Debug, Clone, Copy)]
pub enum MctsAction {
    /// Permute state — shift perspective, decompose the problem
    Decompose,
    /// Bind state with goal — narrow toward the target
    Specialize,
    /// Bundle state with random seed — generalize, widen search
    Generalize,
    /// Bind state with its own permutation — explore contrasts
    Contrast,
}

impl MctsAction {
    /// Cycle through all 4 actions deterministically per expansion count
    fn from_index(idx: usize) -> Self {
        match idx % 4 {
            0 => MctsAction::Decompose,
            1 => MctsAction::Specialize,
            2 => MctsAction::Generalize,
            3 => MctsAction::Contrast,
            _ => unreachable!(),
        }
    }
}

/// A node in the Sovereign reasoning tree.
pub struct MctsNode {
    pub state_hv: HyperMemory,
    pub visits: f64,
    pub value: f64,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    /// Which action created this node (None for root)
    pub action: Option<MctsAction>,
    /// Depth in tree (root = 0)
    pub depth: usize,
}

pub struct MctsEngine {
    pub nodes: Vec<MctsNode>,
    pub exploration_constant: f64,
    /// Goal vector — what we're trying to reach
    pub goal: HyperMemory,
    /// Total expansions so far (used to cycle action types)
    expansion_count: usize,
}

impl MctsEngine {
    pub fn new(root_state: HyperMemory, goal: HyperMemory) -> Self {
        debuglog!("MctsEngine::new: Initialized with goal vector, exploration_constant=1.414");
        let root = MctsNode {
            state_hv: root_state,
            visits: 0.0,
            value: 0.0,
            children: Vec::new(),
            parent: None,
            action: None,
            depth: 0,
        };
        Self {
            nodes: vec![root],
            exploration_constant: 1.414, // Standard UCB1
            goal,
            expansion_count: 0,
        }
    }

    /// Backward-compatible constructor when no goal is available
    pub fn new_exploratory(root_state: HyperMemory) -> Self {
        debuglog!("MctsEngine::new_exploratory: No goal vector — using root state as goal");
        let goal = root_state.clone();
        Self::new(root_state, goal)
    }

    /// DELIBERATE: Performs N iterations of MCTS to find the optimal solution path.
    pub fn deliberate(&mut self, iterations: usize, supervisor: &PslSupervisor) -> Result<HyperMemory, Box<dyn std::error::Error>> {
        info!("// AUDIT: Starting {} iterations of System 2 deliberation...", iterations);

        for i in 0..iterations {
            let selected_idx = self.select();
            let expanded_idx = self.expand(selected_idx)?;
            let reward = self.simulate(expanded_idx, supervisor)?;
            self.backpropagate(expanded_idx, reward);
            if (i + 1) % 50 == 0 {
                debuglog!("MctsEngine::deliberate: iteration {}/{}, nodes={}", i + 1, iterations, self.nodes.len());
            }
        }

        // Return the best child of the root by visit count
        let best_child_idx = self.nodes[0].children.iter()
            .max_by(|&&a, &&b| {
                self.nodes[a].visits.partial_cmp(&self.nodes[b].visits)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(0);

        let best = &self.nodes[best_child_idx];
        info!("// AUDIT: Deliberation Complete. Best value={:.4}, visits={:.0}, action={:?}, depth={}",
            best.value / best.visits.max(1.0), best.visits, best.action, best.depth);
        Ok(best.state_hv.clone())
    }

    fn select(&self) -> usize {
        let mut current = 0;
        while !self.nodes[current].children.is_empty() {
            current = self.nodes[current].children.iter()
                .copied()
                .max_by(|&a, &b| {
                    let uct_a = self.calculate_uct(current, a);
                    let uct_b = self.calculate_uct(current, b);
                    uct_a.partial_cmp(&uct_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(current);
        }
        current
    }

    fn calculate_uct(&self, parent_idx: usize, child_idx: usize) -> f64 {
        let parent = &self.nodes[parent_idx];
        let child = &self.nodes[child_idx];
        if child.visits == 0.0 { return f64::INFINITY; }
        (child.value / child.visits) + self.exploration_constant * (parent.visits.ln() / child.visits).sqrt()
    }

    /// EXPAND: Apply a semantic action operator to create a meaningful child state.
    fn expand(&mut self, parent_idx: usize) -> Result<usize, Box<dyn std::error::Error>> {
        let action = MctsAction::from_index(self.expansion_count);
        self.expansion_count += 1;

        let parent_state = &self.nodes[parent_idx].state_hv;
        let parent_depth = self.nodes[parent_idx].depth;

        let new_state = match action {
            MctsAction::Decompose => {
                // Permute — shifts the representational frame
                debuglog!("MctsEngine::expand: DECOMPOSE (permute by 1)");
                parent_state.permute(1)?
            },
            MctsAction::Specialize => {
                // Bind with goal — pulls the state toward the target
                debuglog!("MctsEngine::expand: SPECIALIZE (bind with goal)");
                parent_state.bind(&self.goal)?
            },
            MctsAction::Generalize => {
                // Bundle with a seeded random vector — broadens the representation
                debuglog!("MctsEngine::expand: GENERALIZE (bundle with seed)");
                let seed = HyperMemory::generate_seed(DIM_PROLETARIAT);
                HyperMemory::bundle(&[parent_state.clone(), seed])?
            },
            MctsAction::Contrast => {
                // Bind with own permutation — explores the opposite/complementary
                debuglog!("MctsEngine::expand: CONTRAST (bind with permuted self)");
                let shifted = parent_state.permute(3)?;
                parent_state.bind(&shifted)?
            },
        };

        let new_idx = self.nodes.len();
        self.nodes.push(MctsNode {
            state_hv: new_state,
            visits: 0.0,
            value: 0.0,
            children: Vec::new(),
            parent: Some(parent_idx),
            action: Some(action),
            depth: parent_depth + 1,
        });
        self.nodes[parent_idx].children.push(new_idx);
        Ok(new_idx)
    }

    /// SIMULATE: Evaluate the expanded node's ACTUAL state.
    ///
    /// Reward = weighted combination of:
    ///   - PSL compliance (axiom satisfaction on the actual state vector)
    ///   - Goal proximity (cosine similarity between state and goal)
    fn simulate(&self, node_idx: usize, supervisor: &PslSupervisor) -> Result<f64, Box<dyn std::error::Error>> {
        let state = &self.nodes[node_idx].state_hv;

        // 1. PSL compliance: audit the actual expanded state
        let raw_bits = state.export_raw_bitvec();
        let state_bv = BipolarVector::from_bitvec(raw_bits).map_err(|e| -> Box<dyn std::error::Error> {
            format!("BipolarVector conversion failed: {:?}", e).into()
        })?;
        let target = AuditTarget::Vector(state_bv);
        let verdict = supervisor.audit(&target)?;
        let psl_score = verdict.confidence;

        // 2. Goal proximity: how close is this state to what we're trying to achieve?
        let goal_similarity = state.similarity(&self.goal);
        // Normalize from [-1,1] cosine range to [0,1] reward range
        let goal_score = (goal_similarity + 1.0) / 2.0;

        // Weighted combination: PSL compliance is a hard constraint, goal proximity is the objective
        let reward = 0.4 * psl_score + 0.6 * goal_score;

        debuglog!("MctsEngine::simulate: node={}, psl={:.3}, goal_sim={:.3}, reward={:.3}",
            node_idx, psl_score, goal_similarity, reward);

        Ok(reward)
    }

    fn backpropagate(&mut self, node_idx: usize, reward: f64) {
        let mut current = Some(node_idx);
        while let Some(idx) = current {
            self.nodes[idx].visits += 1.0;
            self.nodes[idx].value += reward;
            current = self.nodes[idx].parent;
        }
    }

    /// Returns the number of nodes in the tree
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the best action sequence from root to best leaf
    pub fn best_path(&self) -> Vec<MctsAction> {
        let mut path = Vec::new();
        let mut current = 0;
        while !self.nodes[current].children.is_empty() {
            let best = self.nodes[current].children.iter()
                .copied()
                .max_by(|&a, &b| {
                    let va = self.nodes[a].value / self.nodes[a].visits.max(1.0);
                    let vb = self.nodes[b].value / self.nodes[b].visits.max(1.0);
                    va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(current);
            if best == current { break; }
            if let Some(action) = self.nodes[best].action {
                path.push(action);
            }
            current = best;
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psl::axiom::DimensionalityAxiom;

    #[test]
    fn test_mcts_deliberation_runs() {
        let root = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut engine = MctsEngine::new(root, goal);

        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let result = engine.deliberate(20, &supervisor);
        assert!(result.is_ok(), "Deliberation should complete without error");
        assert!(engine.node_count() > 1, "Tree should have expanded nodes");
    }

    #[test]
    fn test_mcts_semantic_actions_cycle() {
        // Verify actions cycle through all 4 types
        assert!(matches!(MctsAction::from_index(0), MctsAction::Decompose));
        assert!(matches!(MctsAction::from_index(1), MctsAction::Specialize));
        assert!(matches!(MctsAction::from_index(2), MctsAction::Generalize));
        assert!(matches!(MctsAction::from_index(3), MctsAction::Contrast));
        assert!(matches!(MctsAction::from_index(4), MctsAction::Decompose)); // cycles
    }

    #[test]
    fn test_mcts_best_path_returns_actions() {
        let root = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let goal = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut engine = MctsEngine::new(root, goal);

        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let _ = engine.deliberate(12, &supervisor);
        let path = engine.best_path();
        // After 12 iterations there should be at least 1 action in the path
        assert!(!path.is_empty(), "Best path should contain at least one action");
    }

    #[test]
    fn test_mcts_simulate_evaluates_actual_state() {
        // Verify that simulate() uses the actual node state, not a fixed seed.
        // Two nodes with different states should produce different rewards.
        let state_a = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let state_b = HyperMemory::generate_seed(DIM_PROLETARIAT);
        // Goal is state_a — so simulating state_a should score higher than state_b
        let goal = state_a.clone();

        let mut supervisor = PslSupervisor::new();
        supervisor.register_axiom(Box::new(DimensionalityAxiom));

        let engine = MctsEngine::new(state_a.clone(), goal);
        // Manually check: root node (state_a = goal) vs a hypothetical node with state_b
        let root_reward = engine.simulate(0, &supervisor).expect("simulate should succeed");

        // Build a second engine with state_b as root, same goal
        let engine_b = MctsEngine::new(state_b, state_a);
        let other_reward = engine_b.simulate(0, &supervisor).expect("simulate should succeed");

        debuglog!("test_simulate: root_reward={:.4} (state=goal), other_reward={:.4} (state!=goal)", root_reward, other_reward);
        // State identical to goal should score higher
        assert!(root_reward > other_reward,
            "Self-similar state should score higher (self={:.4} vs other={:.4})", root_reward, other_reward);
    }
}
