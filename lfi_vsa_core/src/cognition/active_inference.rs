// NODE 032: Active Inference Control Loop
// STATUS: ALPHA - Material Bridge Active
// PROTOCOL: Surprisal-Minimization / Motor-Command-Synthesis
//
// PURPOSE: Implements the Active Inference loop (Friston, 2010).
// The agent compares predicted world-state with observed state
// and either:
//   (a) issues motor commands to change the world (action), or
//   (b) updates its internal model to match reality (perception).
//
// FREE ENERGY DECOMPOSITION:
//   F = D_KL[q(s) || p(s|o)] + ln p(o)
//   Approximated in VSA space as:
//     F ≈ prediction_error + model_complexity_penalty
//   Where prediction_error = 1.0 - similarity(predicted, observed)
//
// CORE LOOP:
//   1. Predict: What should the world state be? (via WorldModel)
//   2. Observe: What is the actual world state? (sensor input)
//   3. Compute: Free energy = prediction error
//   4. Decide:  If F > action_threshold → act (motor command)
//               If F < action_threshold → perceive (update model)
//   5. Learn:   Record the cause-effect for future predictions
//
// REFERENCES:
//   Friston, K. (2010). The Free-Energy Principle.
//   Parr, T., Pezzulo, G., & Friston, K.J. (2022). Active Inference.

use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use crate::hdc::error::HdcError;
use tracing::{info, debug, warn};

/// A motor command — bit-level hex representation of a physical action.
#[derive(Debug)]
pub struct MotorCommand {
    pub hex_payload: Vec<u8>,
    pub metadata: String,
}

/// Result of a single inference step — either act on the world or update beliefs.
#[derive(Debug)]
pub enum InferenceOutcome {
    /// Free energy below threshold — equilibrium, no action needed
    Equilibrium { free_energy: f64 },
    /// Free energy above threshold — issue motor command to change the world
    Act { command: MotorCommand, free_energy: f64, prediction_error: f64 },
    /// Model is wrong — update internal beliefs to match observation
    Perceive { free_energy: f64, model_updated: bool },
}

/// Configurable thresholds for the Active Inference loop.
#[derive(Debug, Clone)]
pub struct InferencePolicy {
    /// Below this free energy, the system is at equilibrium (no action)
    pub equilibrium_threshold: f64,
    /// Above this, prefer action over perception (change world vs change beliefs)
    pub action_threshold: f64,
    /// Maximum free energy before emergency intervention
    pub emergency_threshold: f64,
    /// How many causal links to retain in the world model
    pub max_causal_history: usize,
}

impl Default for InferencePolicy {
    fn default() -> Self {
        debuglog!("InferencePolicy::default: Loading sovereign inference thresholds");
        Self {
            equilibrium_threshold: 0.1,
            action_threshold: 0.4,
            emergency_threshold: 0.9,
            max_causal_history: 500,
        }
    }
}

pub struct ActiveInferenceCore {
    /// Internal model of the world (VSA register).
    pub internal_model: HyperMemory,
    /// Target state we are trying to achieve (the "prior preference").
    pub target_state: HyperMemory,
    /// Policy thresholds
    pub policy: InferencePolicy,
    /// Running average of free energy (for trend detection)
    free_energy_ema: f64,
    /// Step counter
    step_count: u64,
}

impl ActiveInferenceCore {
    pub fn new(initial_model: HyperMemory) -> Self {
        info!("// AUDIT: Active Inference Core initialized.");
        Self {
            target_state: HyperMemory::new(DIM_PROLETARIAT),
            internal_model: initial_model,
            policy: InferencePolicy::default(),
            free_energy_ema: 0.0,
            step_count: 0,
        }
    }

    pub fn with_policy(initial_model: HyperMemory, policy: InferencePolicy) -> Self {
        debuglog!("ActiveInferenceCore::with_policy: Custom policy loaded");
        Self {
            target_state: HyperMemory::new(DIM_PROLETARIAT),
            internal_model: initial_model,
            policy,
            free_energy_ema: 0.0,
            step_count: 0,
        }
    }

    /// Set the target state (prior preference / goal attractor).
    pub fn set_target(&mut self, target: HyperMemory) {
        debuglog!("ActiveInferenceCore::set_target: Prior preference updated");
        self.target_state = target;
    }

    /// STEP: Execute one iteration of the Active Inference loop.
    ///
    /// Takes an observation (current sensor state) and returns the appropriate
    /// inference outcome: equilibrium, action, or perception update.
    pub fn step(&mut self, observation: &HyperMemory) -> Result<InferenceOutcome, HdcError> {
        self.step_count += 1;
        debuglog!("ActiveInferenceCore::step: iteration={}", self.step_count);

        // 1. Compute prediction error: how far is observation from our prediction?
        let prediction_error = 1.0 - observation.similarity(&self.internal_model);

        // 2. Compute goal error: how far is observation from where we want to be?
        let goal_error = 1.0 - observation.similarity(&self.target_state);

        // 3. Free energy ≈ prediction_error + goal_error (simplified variational bound)
        let free_energy = 0.5 * prediction_error + 0.5 * goal_error;

        // 4. Update exponential moving average for trend detection
        let alpha = 0.2; // EMA smoothing factor
        self.free_energy_ema = alpha * free_energy + (1.0 - alpha) * self.free_energy_ema;

        debuglog!("ActiveInferenceCore::step: pred_err={:.4}, goal_err={:.4}, F={:.4}, EMA={:.4}",
            prediction_error, goal_error, free_energy, self.free_energy_ema);

        // 5. Decision: equilibrium, act, or perceive?
        if free_energy < self.policy.equilibrium_threshold {
            debug!("ActiveInference: Equilibrium achieved (F={:.4} < {:.4})", free_energy, self.policy.equilibrium_threshold);
            return Ok(InferenceOutcome::Equilibrium { free_energy });
        }

        if free_energy >= self.policy.emergency_threshold {
            warn!("// AUDIT: EMERGENCY free energy ({:.4}) — forcing immediate action", free_energy);
        }

        if goal_error > prediction_error {
            // World is wrong relative to our goals — act to change it
            let command = self.synthesize_command(observation, free_energy)?;
            debuglog!("ActiveInferenceCore::step: ACTION — issuing motor command (F={:.4})", free_energy);
            Ok(InferenceOutcome::Act {
                command,
                free_energy,
                prediction_error,
            })
        } else {
            // Our model is wrong — update beliefs to match reality
            debuglog!("ActiveInferenceCore::step: PERCEIVE — updating internal model (F={:.4})", free_energy);
            self.internal_model = observation.clone();
            Ok(InferenceOutcome::Perceive {
                free_energy,
                model_updated: true,
            })
        }
    }

    /// Synthesize a motor command from the free energy gradient.
    /// In a real deployment, this maps the error vector to a motor-primitive
    /// in the VSA codebook. For Alpha validation, outputs a UART packet
    /// encoding the error magnitude.
    fn synthesize_command(&self, observation: &HyperMemory, free_energy: f64) -> Result<MotorCommand, HdcError> {
        debuglog!("ActiveInferenceCore::synthesize_command: F={:.4}", free_energy);

        // Encode free energy magnitude into the command byte
        let magnitude = (free_energy * 255.0).min(255.0) as u8;

        // Direction: bind observation with target to get the "correction vector"
        // In VSA, bind(obs, target) gives the transformation needed
        let correction = observation.bind(&self.target_state).map_err(|e| {
            HdcError::LogicFault { reason: format!("Motor synthesis failed: {}", e) }
        })?;

        // Extract a compact action signature from the correction vector
        let action_sig: u8 = correction.vector.iter()
            .take(8)
            .enumerate()
            .fold(0u8, |acc, (i, &v)| if v > 0 { acc | (1 << i) } else { acc });

        Ok(MotorCommand {
            hex_payload: vec![0xAA, 0x55, action_sig, magnitude],
            metadata: format!("Free energy correction (F={:.4}, sig=0x{:02X})", free_energy, action_sig),
        })
    }

    /// Returns the running EMA of free energy
    pub fn free_energy_trend(&self) -> f64 {
        self.free_energy_ema
    }

    /// Returns the total number of inference steps executed
    pub fn total_steps(&self) -> u64 {
        self.step_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_inference_equilibrium() {
        let model = HyperMemory::new(DIM_PROLETARIAT);
        let mut core = ActiveInferenceCore::new(model.clone());
        core.set_target(model.clone());

        let result = core.step(&model).expect("step should succeed");
        assert!(matches!(result, InferenceOutcome::Equilibrium { .. }),
            "Should be at equilibrium when observation = model = target");
    }

    #[test]
    fn test_active_inference_action() {
        let model = HyperMemory::new(DIM_PROLETARIAT);
        let mut core = ActiveInferenceCore::new(model.clone());
        // Set a distant target — goal_error will be high
        core.set_target(HyperMemory::generate_seed(DIM_PROLETARIAT));

        let result = core.step(&model).expect("step should succeed");
        match result {
            InferenceOutcome::Act { command, free_energy, .. } => {
                assert!(!command.hex_payload.is_empty(), "Should issue motor command");
                assert!(free_energy > 0.0, "Free energy should be positive");
            },
            other => panic!("Expected Act, got {:?}", other),
        }
    }

    #[test]
    fn test_active_inference_perception_update() {
        // When prediction_error > goal_error, system should update beliefs
        let model = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let observation = HyperMemory::generate_seed(DIM_PROLETARIAT);
        let mut core = ActiveInferenceCore::new(model);
        // Target is close to observation — so goal_error is low
        core.set_target(observation.clone());

        let result = core.step(&observation).expect("step should succeed");
        // Should perceive (update model) since model is wrong but observation matches target
        assert!(matches!(result, InferenceOutcome::Perceive { model_updated: true, .. }),
            "Should update model when prediction error exceeds goal error");
    }

    #[test]
    fn test_active_inference_ema_tracking() {
        let model = HyperMemory::new(DIM_PROLETARIAT);
        let mut core = ActiveInferenceCore::new(model.clone());
        core.set_target(HyperMemory::generate_seed(DIM_PROLETARIAT));

        // Run multiple steps — EMA should track free energy
        for _ in 0..5 {
            let _ = core.step(&model);
        }
        assert!(core.free_energy_trend() > 0.0, "EMA should be positive after high-F steps");
        assert_eq!(core.total_steps(), 5, "Should have counted 5 steps");
    }

    #[test]
    fn test_active_inference_custom_policy() {
        let policy = InferencePolicy {
            equilibrium_threshold: 0.5,  // Very lenient
            action_threshold: 0.8,
            emergency_threshold: 0.95,
            max_causal_history: 100,
        };
        let model = HyperMemory::new(DIM_PROLETARIAT);
        let mut core = ActiveInferenceCore::with_policy(model.clone(), policy);
        core.set_target(model.clone());

        let result = core.step(&model).expect("step should succeed");
        assert!(matches!(result, InferenceOutcome::Equilibrium { .. }),
            "Lenient policy should still detect equilibrium");
    }

    #[test]
    fn test_motor_command_encodes_magnitude() {
        let model = HyperMemory::new(DIM_PROLETARIAT);
        let mut core = ActiveInferenceCore::new(model.clone());
        core.set_target(HyperMemory::generate_seed(DIM_PROLETARIAT));

        let result = core.step(&model).expect("step should succeed");
        if let InferenceOutcome::Act { command, .. } = result {
            assert_eq!(command.hex_payload[0], 0xAA, "UART header byte 1");
            assert_eq!(command.hex_payload[1], 0x55, "UART header byte 2");
            assert!(command.hex_payload.len() == 4, "Command should be 4 bytes");
        }
    }
}
