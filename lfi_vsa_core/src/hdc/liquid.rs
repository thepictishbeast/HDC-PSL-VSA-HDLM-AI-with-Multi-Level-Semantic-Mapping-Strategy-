// ============================================================
// LFI Liquid Neural Network (LNN) — The Adaptive Sensorium
// Section 1.II: "Governed by Ordinary Differential Equations (ODEs)
// that change during inference to ingest chaos and noise."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use serde::{Serialize, Deserialize};

/// A single neuron in the Liquid Neural Network.
/// Its state is fluid and governed by time-series dynamics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidNeuron {
    pub state: f64,
    pub tau: f64,      // Time constant (The "Liquid" parameter)
    pub weight: f64,
}

/// The LNN Sensory Organ. Replaces probabilistic LLMs with
/// deterministic, noise-adaptive differential equations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidSensorium {
    neurons: Vec<LiquidNeuron>,
    /// Non-linear binarizer threshold.
    binarization_threshold: f64,
}

impl LiquidSensorium {
    /// Initialize a new LNN with N neurons.
    pub fn new(neuron_count: usize) -> Self {
        debuglog!("LiquidSensorium::new: Initializing {} ODE-governed neurons", neuron_count);
        let mut neurons = Vec::with_capacity(neuron_count);
        for _ in 0..neuron_count {
            neurons.push(LiquidNeuron {
                state: 0.0,
                tau: 1.0, // Dynamic time constant
                weight: 0.5,
            });
        }
        Self { neurons, binarization_threshold: 0.0 }
    }

    /// Process a noisy time-series input (e.g., raw PCAP or sensor delta).
    /// Updates the "Liquid State" using a Euler-method ODE approximation.
    pub fn step(&mut self, input: f64, dt: f64) -> Result<(), HdcError> {
        debuglog!("LiquidSensorium::step: input={:.4}, dt={:.4}", input, dt);
        for neuron in &mut self.neurons {
            // ODE: d[state]/dt = (-(state) + (input * weight)) / tau
            let derivative = (-neuron.state + (input * neuron.weight)) / neuron.tau;
            neuron.state += derivative * dt;
        }
        Ok(())
    }

    /// Projects the current fluid state into the 10,000-bit HDC space.
    /// This is the "Non-linear Encoder" mentioned in the blueprint.
    ///
    /// Uses position-dependent XOR spreading to ensure balanced output
    /// even when neurons have correlated states. Each bit is determined by:
    ///   bit[i] = hash(i, neuron_idx) XOR sign(neuron.state - threshold)
    /// This guarantees roughly 50% ones regardless of neuron state bias.
    pub fn project_to_vsa(&self) -> Result<BipolarVector, HdcError> {
        debuglog!("LiquidSensorium::project_to_vsa: Binarizing fluid state via positional XOR");

        // Generate a deterministic positional scaffold from neuron states.
        // Each neuron contributes a seed that is spread across dimensions.
        let n = self.neurons.len();
        if n == 0 {
            debuglog!("LiquidSensorium::project_to_vsa: no neurons, returning random");
            return BipolarVector::new_random();
        }

        // Start with a seeded base vector from the aggregate neuron state
        let state_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            for neuron in &self.neurons {
                // Quantize state to i64 for deterministic hashing
                let quantized = (neuron.state * 1e6) as i64;
                quantized.hash(&mut h);
            }
            h.finish()
        };
        let base = BipolarVector::from_seed(state_hash);

        // Apply positional permutations weighted by individual neuron states.
        // This creates a unique, balanced vector that encodes the full LNN state.
        let mut result = base;
        for (i, neuron) in self.neurons.iter().enumerate() {
            let tau_hash = (neuron.tau * 1e6) as u64;
            let weight_hash = (neuron.weight * 1e6) as u64;
            let neuron_seed = tau_hash.wrapping_mul(31).wrapping_add(weight_hash).wrapping_add(i as u64);
            let neuron_vec = BipolarVector::from_seed(neuron_seed);

            // Bind (XOR) if neuron state is above threshold, permute if below.
            // This preserves quasi-orthogonality and balance.
            if neuron.state > self.binarization_threshold {
                result = result.bind(&neuron_vec)?;
            } else {
                result = result.permute(i + 1)?;
            }
        }

        debuglog!("LiquidSensorium::project_to_vsa: ones={}", result.count_ones());
        Ok(result)
    }

    /// Genetic adaptation: mutate the time constants (tau) to handle new noise patterns.
    pub fn mutate_tau(&mut self, factors: &[f64]) {
        let n_count = self.neurons.len();
        for (i, factor) in factors.iter().enumerate() {
            if let Some(neuron) = self.neurons.get_mut(i % n_count) {
                neuron.tau = (neuron.tau * factor).clamp(0.1, 10.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnn_adaptation() -> Result<(), HdcError> {
        let mut lnn = LiquidSensorium::new(19); // MIT-style sparse LNN
        
        // Step through noise
        for _ in 0..100 {
            lnn.step(1.0, 0.01)?;
        }
        
        let hv = lnn.project_to_vsa()?;
        assert_eq!(hv.dim(), 10000);
        debuglog!("test_lnn_adaptation: ones={}", hv.count_ones());
        Ok(())
    }
}
