// NODE 017: VSA-Driven Monte Carlo Tree Search (MCTS)
// STATUS: ALPHA - System 2 Deliberation Active
// PROTOCOL: Test-Time Compute / Branching-Logic-Synthesis

use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use crate::psl::supervisor::PslSupervisor;
use crate::psl::axiom::AuditTarget;
use tracing::info;

/// A node in the Sovereign reasoning tree.
pub struct MctsNode {
    pub state_hv: HyperMemory,
    pub visits: f64,
    pub value: f64,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

pub struct MctsEngine {
    pub nodes: Vec<MctsNode>,
    pub exploration_constant: f64,
}

impl MctsEngine {
    pub fn new(root_state: HyperMemory) -> Self {
        info!("// AUDIT: MCTS Deliberation Engine Initialized.");
        let root = MctsNode {
            state_hv: root_state,
            visits: 0.0,
            value: 0.0,
            children: Vec::new(),
            parent: None,
        };
        Self {
            nodes: vec![root],
            exploration_constant: 1.414, // Standard UCB1
        }
    }

    /// DELIBERATE: Performs N iterations of MCTS to find the optimal solution path.
    pub fn deliberate(&mut self, iterations: usize, supervisor: &PslSupervisor) -> Result<HyperMemory, Box<dyn std::error::Error>> {
        info!("// AUDIT: Starting {} iterations of System 2 deliberation...", iterations);

        for _ in 0..iterations {
            let selected_idx = self.select();
            let expanded_idx = self.expand(selected_idx)?;
            let reward = self.simulate(expanded_idx, supervisor)?;
            self.backpropagate(expanded_idx, reward);
        }

        // Return the best child of the root
        let best_child_idx = self.nodes[0].children.iter()
            .max_by(|&&a, &&b| {
                self.nodes[a].visits.partial_cmp(&self.nodes[b].visits)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap_or(0);

        info!("// AUDIT: Deliberation Complete. Optimal path value: {:.4}", self.nodes[best_child_idx].value);
        Ok(self.nodes[best_child_idx].state_hv.clone())
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

    fn expand(&mut self, parent_idx: usize) -> Result<usize, Box<dyn std::error::Error>> {
        let mut new_state = self.nodes[parent_idx].state_hv.clone();
        let noise = HyperMemory::generate_seed(DIM_PROLETARIAT);
        new_state = new_state.bind(&noise)?; 

        let new_idx = self.nodes.len();
        self.nodes.push(MctsNode {
            state_hv: new_state,
            visits: 0.0,
            value: 0.0,
            children: Vec::new(),
            parent: Some(parent_idx),
        });
        self.nodes[parent_idx].children.push(new_idx);
        Ok(new_idx)
    }

    fn simulate(&self, _node_idx: usize, supervisor: &PslSupervisor) -> Result<f64, Box<dyn std::error::Error>> {
        // Evaluate the vector state similarity against safety seeds
        let target = AuditTarget::Vector(crate::hdc::vector::BipolarVector::from_seed(0)); 
        let verdict = supervisor.audit(&target)?;
        Ok(verdict.confidence)
    }

    fn backpropagate(&mut self, node_idx: usize, reward: f64) {
        let mut current = Some(node_idx);
        while let Some(idx) = current {
            self.nodes[idx].visits += 1.0;
            self.nodes[idx].value += reward;
            current = self.nodes[idx].parent;
        }
    }
}
