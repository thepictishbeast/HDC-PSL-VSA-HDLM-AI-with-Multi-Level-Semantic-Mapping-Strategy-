// ============================================================
// HMAS — Hierarchical Multi-Agent System (Director Protocol)
// Section 1.V: "Hierarchical Multi-Agent System (HMAS). By utilizing 
// rigid, task-specific Agent Templates governed by a central Director."
// ============================================================

use crate::debuglog;
use crate::agent::LfiAgent;
use crate::hdc::error::HdcError;

/// Specific roles for sub-agents (Executors).
#[derive(Debug, Clone, PartialEq)]
pub enum AgentRole {
    Director,
    WeightManager,
    WebIngestor,
    ForensicSentinel,
}

/// A rigid template for a sub-agent.
pub struct AgentTemplate {
    pub role: AgentRole,
    // In a full implementation, this holds the specific VSA hypervector 
    // that defines the sub-agent's restricted capability map.
}

/// The Micro-Supervisor for managing sub-agents and persistent reboots.
pub struct MicroSupervisor {
    pub active_agents: Vec<AgentTemplate>,
}

impl MicroSupervisor {
    pub fn new() -> Self {
        Self {
            active_agents: vec![AgentTemplate { role: AgentRole::Director }],
        }
    }

    /// Spawns a new sub-agent for a specific task.
    pub fn spawn_executor(&mut self, role: AgentRole) {
        debuglog!("MicroSupervisor: Spawning new executor sub-agent: {:?}", role);
        self.active_agents.push(AgentTemplate { role });
    }

    /// Destroys a sub-agent to minimize the blast radius of a potential compromise.
    pub fn destroy_executor(&mut self, role: AgentRole) {
        debuglog!("MicroSupervisor: Destroying executor sub-agent: {:?}", role);
        self.active_agents.retain(|agent| agent.role != role);
    }

    /// Recursive Performance Audit: The Rollback Logic.
    /// Reverts the agent's state if the fitness score degrades.
    pub fn audit_and_revert(&self, agent: &mut LfiAgent, current_fitness: f64, previous_fitness: f64, backup_path: &str) -> Result<(), HdcError> {
        if current_fitness < previous_fitness {
            debuglog!("MicroSupervisor: ALERT - Fitness degraded ({:.2} < {:.2}). Triggering Material Reversion.", current_fitness, previous_fitness);
            // Load the previous "Known-Good" state from the VSA-Encrypted Blob.
            agent.load_persistent_state(backup_path)?;
            debuglog!("MicroSupervisor: State reversion complete.");
        } else {
            debuglog!("MicroSupervisor: Fitness verified. Committing new state.");
            agent.save_persistent_state(backup_path)?;
        }
        Ok(())
    }
}
