//! # Purpose
//! Natural gradient descent on the Fisher information manifold for Active Inference.
//! Vanilla SGD drifts on curved parameter spaces because it descends in Euclidean
//! coordinates. Natural gradient θ ← θ − η·G(θ)⁻¹·∇L follows geodesics.
//!
//! # Design Decisions
//! - Diagonal Fisher approximation (not full matrix — O(d) not O(d²))
//! - Exponential moving average for Fisher diagonal estimation
//! - Damping term λ for numerical stability (prevents division by near-zero)
//! - Compatible with any parameter vector, not tied to specific AIF implementation
//!
//! # Invariants
//! - Fisher diagonal entries are always positive (squared gradients)
//! - Damping λ > 0 prevents singular inverse
//! - EMA decay ∈ (0, 1)
//!
//! # Failure Modes
//! - If all gradients are zero, Fisher diagonal is all-λ (identity-like)
//! - Very small damping with large Fisher values → tiny steps (undershoot)

/// Diagonal Fisher Information Matrix approximation for natural gradient.
pub struct NaturalGradient {
    /// Diagonal of the Fisher information matrix (running EMA of squared gradients).
    fisher_diag: Vec<f64>,
    /// EMA decay factor for Fisher estimation.
    ema_decay: f64,
    /// Damping term for numerical stability.
    damping: f64,
    /// Learning rate.
    lr: f64,
    /// Whether the Fisher has been initialized with at least one gradient.
    initialized: bool,
}

impl NaturalGradient {
    pub fn new(dim: usize, lr: f64) -> Self {
        Self {
            fisher_diag: vec![1.0; dim],
            ema_decay: 0.99,
            damping: 1e-4,
            lr,
            initialized: false,
        }
    }

    /// Update the Fisher diagonal estimate with a new gradient observation.
    pub fn observe_gradient(&mut self, gradient: &[f64]) {
        if gradient.len() != self.fisher_diag.len() {
            return;
        }
        let alpha = if self.initialized { self.ema_decay } else { 0.0 };
        for (f, &g) in self.fisher_diag.iter_mut().zip(gradient.iter()) {
            *f = alpha * *f + (1.0 - alpha) * g * g;
        }
        self.initialized = true;
    }

    /// Compute natural gradient step: G(θ)⁻¹ · ∇L, scaled by learning rate.
    /// Returns the parameter update vector (subtract from current params).
    pub fn step(&self, gradient: &[f64]) -> Vec<f64> {
        gradient.iter().zip(self.fisher_diag.iter())
            .map(|(&g, &f)| self.lr * g / (f + self.damping))
            .collect()
    }

    /// Combined observe + step in one call.
    pub fn update(&mut self, params: &mut [f64], gradient: &[f64]) {
        self.observe_gradient(gradient);
        let delta = self.step(gradient);
        for (p, d) in params.iter_mut().zip(delta.iter()) {
            *p -= d;
        }
    }

    /// Effective learning rate per dimension (lr / (fisher + damping)).
    pub fn effective_lr(&self) -> Vec<f64> {
        self.fisher_diag.iter()
            .map(|&f| self.lr / (f + self.damping))
            .collect()
    }

    /// Condition number estimate (max/min effective lr ratio).
    pub fn condition_ratio(&self) -> f64 {
        let elr = self.effective_lr();
        let max = elr.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = elr.iter().cloned().fold(f64::INFINITY, f64::min);
        if min > 0.0 { max / min } else { f64::INFINITY }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_gradient_step_scales_by_fisher() {
        let mut ng = NaturalGradient::new(3, 0.1);
        // Observe gradient [1, 10, 100] — Fisher diag becomes [1, 100, 10000]
        ng.observe_gradient(&[1.0, 10.0, 100.0]);
        let step = ng.step(&[1.0, 10.0, 100.0]);
        // Natural gradient normalizes: large-Fisher dims get smaller steps
        assert!(step[0] > step[1], "Dim 0 (small Fisher) gets larger step than dim 1");
        assert!(step[1] > step[2], "Dim 1 gets larger step than dim 2 (largest Fisher)");
    }

    #[test]
    fn test_update_modifies_params() {
        let mut ng = NaturalGradient::new(2, 0.1);
        let mut params = vec![1.0, 1.0];
        let gradient = vec![0.5, 0.5];
        ng.update(&mut params, &gradient);
        assert!(params[0] < 1.0, "Params should decrease with positive gradient");
        assert!(params[1] < 1.0);
    }

    #[test]
    fn test_ema_accumulates_fisher() {
        let mut ng = NaturalGradient::new(1, 0.1);
        ng.observe_gradient(&[1.0]);
        let f1 = ng.fisher_diag[0];
        ng.observe_gradient(&[2.0]);
        let f2 = ng.fisher_diag[0];
        // EMA should increase Fisher when seeing larger gradients
        assert!(f2 > f1, "Fisher should increase with larger gradient observations");
    }

    #[test]
    fn test_damping_prevents_division_by_zero() {
        let ng = NaturalGradient::new(1, 0.1);
        // Fisher is initialized to 1.0, damping is 1e-4
        let step = ng.step(&[1.0]);
        assert!(step[0].is_finite(), "Step should be finite with damping");
    }

    #[test]
    fn test_condition_ratio() {
        let mut ng = NaturalGradient::new(3, 0.1);
        ng.observe_gradient(&[0.01, 1.0, 100.0]);
        let ratio = ng.condition_ratio();
        assert!(ratio > 1.0, "Condition ratio should be > 1 for varied Fisher");
        assert!(ratio.is_finite());
    }
}
