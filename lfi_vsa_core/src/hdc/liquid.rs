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
    ///
    /// AVP-PASS-N: 2026-04-15 — earlier impl applied `neuron.tau *= factor`
    /// without sanitizing the factor. NaN factors propagated into tau,
    /// producing non-finite state across the entire sensorium. Now any
    /// non-finite or non-positive factor is silently skipped.
    pub fn mutate_tau(&mut self, factors: &[f64]) {
        let n_count = self.neurons.len();
        if n_count == 0 { return; }
        for (i, factor) in factors.iter().enumerate() {
            // SECURITY: drop NaN / Inf / non-positive factors. Mutation
            // intent is reasonable scaling, not arithmetic poisoning.
            if !factor.is_finite() || *factor <= 0.0 {
                continue;
            }
            if let Some(neuron) = self.neurons.get_mut(i % n_count) {
                let new_tau = (neuron.tau * factor).clamp(0.1, 10.0);
                if new_tau.is_finite() && new_tau > 0.0 {
                    neuron.tau = new_tau;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lnn_adaptation() -> Result<(), HdcError> {
        let mut lnn = LiquidSensorium::new(19);
        for _ in 0..100 {
            lnn.step(1.0, 0.01)?;
        }
        let hv = lnn.project_to_vsa()?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_lnn_creation() {
        let lnn = LiquidSensorium::new(10);
        assert_eq!(lnn.neurons.len(), 10);
        for n in &lnn.neurons {
            assert_eq!(n.state, 0.0);
            assert_eq!(n.tau, 1.0);
        }
    }

    #[test]
    fn test_step_changes_state() -> Result<(), HdcError> {
        let mut lnn = LiquidSensorium::new(5);
        lnn.step(1.0, 0.1)?;
        // After one step with input=1.0, states should be positive.
        for n in &lnn.neurons {
            assert!(n.state > 0.0, "State should be positive after positive input: {:.4}", n.state);
        }
        Ok(())
    }

    #[test]
    fn test_different_inputs_different_projections() -> Result<(), HdcError> {
        let mut lnn1 = LiquidSensorium::new(10);
        let mut lnn2 = LiquidSensorium::new(10);

        for _ in 0..50 {
            lnn1.step(1.0, 0.1)?;
            lnn2.step(-1.0, 0.1)?;
        }

        let hv1 = lnn1.project_to_vsa()?;
        let hv2 = lnn2.project_to_vsa()?;
        let sim = hv1.similarity(&hv2)?;
        assert!(sim < 0.9, "Different input histories should produce different projections: {:.4}", sim);
        Ok(())
    }

    #[test]
    fn test_mutate_tau() {
        let mut lnn = LiquidSensorium::new(3);
        lnn.mutate_tau(&[2.0, 0.5, 1.5]);
        assert!((lnn.neurons[0].tau - 2.0).abs() < 0.01);
        assert!((lnn.neurons[1].tau - 0.5).abs() < 0.01);
        assert!((lnn.neurons[2].tau - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_tau_clamping() {
        let mut lnn = LiquidSensorium::new(2);
        lnn.mutate_tau(&[100.0, 0.001]);
        assert!(lnn.neurons[0].tau <= 10.0, "Tau should be clamped to max 10.0");
        assert!(lnn.neurons[1].tau >= 0.1, "Tau should be clamped to min 0.1");
    }

    #[test]
    fn test_projection_balanced() -> Result<(), HdcError> {
        let mut lnn = LiquidSensorium::new(10);
        for _ in 0..50 {
            lnn.step(0.5, 0.1)?;
        }
        let hv = lnn.project_to_vsa()?;
        let ones = hv.count_ones() as f64 / 10000.0;
        // Should be roughly balanced (30%-70% ones).
        assert!(ones > 0.3 && ones < 0.7,
            "Projection should be roughly balanced: {:.1}% ones", ones * 100.0);
        Ok(())
    }

    #[test]
    fn test_empty_sensorium() -> Result<(), HdcError> {
        let lnn = LiquidSensorium::new(0);
        // Should not panic on empty neuron set.
        let hv = lnn.project_to_vsa()?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for LiquidSensorium
    // ============================================================

    /// INVARIANT: project_to_vsa always returns a vector of dimension 10000.
    #[test]
    fn invariant_projection_dim_constant() -> Result<(), HdcError> {
        for n in [0usize, 1, 5, 19, 100] {
            let lnn = LiquidSensorium::new(n);
            let hv = lnn.project_to_vsa()?;
            assert_eq!(hv.dim(), 10000,
                "projection dim must be 10000 for neuron_count={}", n);
        }
        Ok(())
    }

    /// INVARIANT: step() never panics on extreme inputs and keeps neuron
    /// state finite (no NaN/Inf from accumulated drift).
    #[test]
    fn invariant_step_keeps_state_finite() -> Result<(), HdcError> {
        let mut lnn = LiquidSensorium::new(19);
        let inputs = [0.0, 1.0, -1.0, 1e6, -1e6, f64::EPSILON];
        for input in inputs {
            for _ in 0..10 {
                lnn.step(input, 0.01)?;
                for n in &lnn.neurons {
                    assert!(n.state.is_finite(),
                        "neuron state went non-finite under input {}", input);
                }
            }
        }
        Ok(())
    }

    /// INVARIANT: mutate_tau respects clamping under extreme inputs
    /// (no NaN, no negative tau).
    #[test]
    fn invariant_mutate_tau_clamps_safely() {
        let mut lnn = LiquidSensorium::new(5);
        // Pass huge factors and check tau stays positive + finite.
        lnn.mutate_tau(&[1e6, -1e6, 0.0, f64::INFINITY, f64::NAN]);
        for n in &lnn.neurons {
            assert!(n.tau.is_finite(),
                "tau went non-finite under extreme mutation: {}", n.tau);
            assert!(n.tau > 0.0,
                "tau went non-positive: {}", n.tau);
        }
    }
}
