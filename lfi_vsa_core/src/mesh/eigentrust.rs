//! # Purpose
//! EigenTrust — distributed trust propagation for the Supersociety mesh.
//! Computes global trust as the principal eigenvector of the normalized
//! local-trust matrix via power iteration.
//!
//! # Design
//! t^(k+1) = (1-α)·C^T·t^(k) + α·p where p = pre-trusted teleport, α ≈ 0.1
//! Converges in ~20-50 iterations. Resists malicious collectives up to ~70%.

use std::collections::HashMap;

/// EigenTrust engine for computing global trust scores.
pub struct EigenTrust {
    /// Local trust: (i,j) → normalized trust score i gives j.
    local_trust: HashMap<(String, String), f64>,
    /// Pre-trusted peers (teleport distribution).
    pre_trusted: HashMap<String, f64>,
    /// Teleport factor (α).
    alpha: f64,
    /// All known peers.
    peers: Vec<String>,
}

impl EigenTrust {
    pub fn new(alpha: f64) -> Self {
        Self {
            local_trust: HashMap::new(),
            pre_trusted: HashMap::new(),
            alpha: alpha.clamp(0.01, 0.5),
            peers: Vec::new(),
        }
    }

    /// Set local trust from peer i to peer j.
    pub fn set_trust(&mut self, from: &str, to: &str, trust: f64) {
        let trust = trust.clamp(0.0, 1.0);
        self.local_trust.insert((from.into(), to.into()), trust);
        if !self.peers.contains(&from.to_string()) { self.peers.push(from.into()); }
        if !self.peers.contains(&to.to_string()) { self.peers.push(to.into()); }
    }

    /// Mark a peer as pre-trusted (part of teleport distribution).
    pub fn set_pre_trusted(&mut self, peer: &str, weight: f64) {
        self.pre_trusted.insert(peer.into(), weight.clamp(0.0, 1.0));
    }

    /// Compute global trust via power iteration.
    /// Returns (peer_id → global_trust_score) map.
    pub fn compute(&self, max_iterations: usize, epsilon: f64) -> HashMap<String, f64> {
        let n = self.peers.len();
        if n == 0 { return HashMap::new(); }

        // Initialize uniform
        let mut trust: Vec<f64> = vec![1.0 / n as f64; n];

        // Pre-trusted teleport vector
        let p_total: f64 = self.pre_trusted.values().sum::<f64>().max(1.0);
        let p: Vec<f64> = self.peers.iter().map(|peer| {
            self.pre_trusted.get(peer).copied().unwrap_or(0.0) / p_total
        }).collect();
        // If no pre-trusted, use uniform
        let p: Vec<f64> = if p.iter().all(|&x| x == 0.0) {
            vec![1.0 / n as f64; n]
        } else { p };

        for _ in 0..max_iterations {
            let mut new_trust = vec![0.0; n];

            // C^T · t: for each peer j, sum trust[i] * c(i,j) over all i
            for (j, peer_j) in self.peers.iter().enumerate() {
                let mut sum = 0.0;
                for (i, peer_i) in self.peers.iter().enumerate() {
                    let c_ij = self.normalized_trust(peer_i, peer_j);
                    sum += trust[i] * c_ij;
                }
                new_trust[j] = (1.0 - self.alpha) * sum + self.alpha * p[j];
            }

            // Check convergence
            let diff: f64 = trust.iter().zip(new_trust.iter())
                .map(|(a, b)| (a - b).abs()).sum();
            trust = new_trust;
            if diff < epsilon { break; }
        }

        // Build result map
        self.peers.iter().zip(trust.iter())
            .map(|(peer, &score)| (peer.clone(), score))
            .collect()
    }

    /// Get normalized trust c(i,j) = local_trust(i,j) / sum_k(local_trust(i,k))
    fn normalized_trust(&self, from: &str, to: &str) -> f64 {
        let raw = self.local_trust.get(&(from.into(), to.into())).copied().unwrap_or(0.0);
        let total: f64 = self.peers.iter()
            .map(|p| self.local_trust.get(&(from.into(), p.clone())).copied().unwrap_or(0.0))
            .sum();
        if total > 0.0 { raw / total } else { 0.0 }
    }

    pub fn peer_count(&self) -> usize { self.peers.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_trust() {
        let mut et = EigenTrust::new(0.1);
        et.set_trust("A", "B", 0.9);
        et.set_trust("B", "A", 0.8);
        et.set_trust("A", "C", 0.3);
        et.set_pre_trusted("A", 1.0);
        let scores = et.compute(50, 1e-6);
        assert!(scores["A"] > scores["C"], "Pre-trusted A should rank highest");
    }

    #[test]
    fn test_converges() {
        let mut et = EigenTrust::new(0.15);
        for i in 0..10 {
            for j in 0..10 {
                if i != j { et.set_trust(&format!("p{i}"), &format!("p{j}"), 0.5); }
            }
        }
        let scores = et.compute(100, 1e-8);
        // Uniform trust → uniform scores
        let vals: Vec<f64> = scores.values().copied().collect();
        let spread = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - vals.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(spread < 0.01, "Uniform trust should give uniform scores: spread={spread}");
    }

    #[test]
    fn test_malicious_peer_limited() {
        let mut et = EigenTrust::new(0.1);
        // Honest peers trust each other
        et.set_trust("honest1", "honest2", 0.9);
        et.set_trust("honest2", "honest1", 0.9);
        // Malicious peer claims high trust
        et.set_trust("malicious", "malicious", 1.0);
        et.set_trust("malicious", "honest1", 0.0);
        et.set_pre_trusted("honest1", 1.0);
        let scores = et.compute(50, 1e-6);
        assert!(scores["honest1"] > scores["malicious"], "Honest should outrank malicious");
    }

    #[test]
    fn test_empty() {
        let et = EigenTrust::new(0.1);
        let scores = et.compute(10, 1e-6);
        assert!(scores.is_empty());
    }
}
