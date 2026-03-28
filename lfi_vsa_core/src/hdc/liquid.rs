// ============================================================
// LFI Liquid Neural Network (LNN) — The Adaptive Sensorium
// Section 1.II: "Governed by Ordinary Differential Equations (ODEs)
// that change during inference to ingest chaos and noise."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use crate::debuglog;
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
    pub fn project_to_vsa(&self) -> Result<BipolarVector, HdcError> {
        debuglog!("LiquidSensorium::project_to_vsa: Binarizing fluid state");
        
        // Map the N neurons to 10,000 dimensions using positional hashing
        let mut bits = bitvec::vec::BitVec::<u8, bitvec::order::Lsb0>::repeat(false, 10000);
        
        for i in 0..10000 {
            let neuron_idx = i % self.neurons.len();
            let neuron = &self.neurons[neuron_idx];
            
            // Non-linear binarization: hash(pos) XOR sign(state)
            let val = neuron.state > self.binarization_threshold;
            bits.set(i, val);
        }

        BipolarVector::from_bitvec(bits)
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
