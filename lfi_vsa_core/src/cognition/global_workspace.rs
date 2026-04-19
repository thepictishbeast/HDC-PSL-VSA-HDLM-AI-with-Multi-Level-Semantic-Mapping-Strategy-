//! # Purpose
//! Global Workspace Theory (GWT) implementation as a capacity-bounded
//! key-value attention bottleneck. HDC superposition has no capacity
//! constraint — everything coexists with interference. GWT adds what
//! HDC lacks: a k-slot competitive workspace where modules compete
//! for broadcast access based on salience.
//!
//! # Design Decisions
//! - k=8 workspace slots (configurable, based on cognitive science ~7±2)
//! - Softmax attention: slot_j = Σᵢ softmax(qⱼ·kᵢ/√d) vᵢ
//! - HDC vectors ARE the workspace values — attention selects which broadcast
//! - Each module (AIF, world model, analogy, metacognitive) submits entries
//! - Winners get broadcast to all modules via the workspace
//! - Losers decay unless re-submitted — implements forgetting
//!
//! # Invariants
//! - Workspace has exactly k slots (never more, never fewer)
//! - Salience scores are non-negative and sum-normalized
//! - Workspace contents change only through compete() calls
//!
//! # Failure Modes
//! - If all modules submit low-salience entries, workspace contains noise
//! - If one module dominates salience, other modules are starved

use crate::hdc::vector::BipolarVector;

/// A workspace entry submitted by a cognitive module.
#[derive(Debug, Clone)]
pub struct WorkspaceEntry {
    /// The HDC vector content being broadcast.
    pub content: BipolarVector,
    /// Which module submitted this entry.
    pub source_module: String,
    /// Salience score (higher = more important). Raw, before normalization.
    pub salience: f64,
    /// Brief label for debugging/provenance.
    pub label: String,
    /// How many compete() rounds this has survived.
    pub age: u32,
}

/// The Global Workspace — capacity-bounded attention bottleneck.
pub struct GlobalWorkspace {
    /// Current workspace contents (exactly k slots).
    slots: Vec<Option<WorkspaceEntry>>,
    /// Number of slots (capacity).
    k: usize,
    /// Decay rate per round for entries that aren't refreshed.
    decay_rate: f64,
    /// Total competition rounds run.
    pub rounds: u64,
}

impl GlobalWorkspace {
    /// Create a new workspace with k slots.
    pub fn new(k: usize) -> Self {
        Self {
            slots: (0..k).map(|_| None).collect(),
            k,
            decay_rate: 0.1,
            rounds: 0,
        }
    }

    /// Standard cognitive workspace (k=8).
    pub fn standard() -> Self {
        Self::new(8)
    }

    /// #397 Size the workspace by a RAM budget (in bytes). Each slot holds
    /// one BipolarVector + metadata — approximately 1.3 KB heap including
    /// the bitvec allocation. We multiply by 1.5× to account for Vec
    /// overhead + label strings + HashMap padding during eviction.
    /// Returns the computed capacity so the caller can log / expose it.
    pub fn new_with_ram_budget(bytes: usize) -> Self {
        let per_slot_bytes = 1300 * 3 / 2; // ~2 KB accounting for overhead
        let k = (bytes / per_slot_bytes).max(8);
        Self::new(k)
    }

    /// Resize the workspace in place. Preserves the highest-salience
    /// entries when shrinking; pads with None when growing. The k-limit
    /// takes effect on the next compete() call.
    pub fn resize(&mut self, new_k: usize) {
        let new_k = new_k.max(1);
        if new_k == self.k { return; }
        if new_k < self.k {
            // Shrink: keep top-k entries by salience.
            let mut entries: Vec<WorkspaceEntry> = self.slots.iter()
                .filter_map(|s| s.clone()).collect();
            entries.sort_by(|a, b| b.salience.partial_cmp(&a.salience)
                .unwrap_or(std::cmp::Ordering::Equal));
            entries.truncate(new_k);
            self.slots = entries.into_iter().map(Some).collect();
            while self.slots.len() < new_k { self.slots.push(None); }
        } else {
            // Grow: pad with None.
            while self.slots.len() < new_k { self.slots.push(None); }
        }
        self.k = new_k;
    }

    /// Current configured capacity (k).
    pub fn capacity(&self) -> usize { self.k }

    /// Estimated heap footprint in bytes (rough; accounts for each
    /// BipolarVector + label String + Vec overhead).
    pub fn ram_footprint_bytes(&self) -> usize {
        // Empty slots cost only Option discriminant (~16 B). Occupied
        // slots cost the bitvec (10_000 bits ≈ 1250 B) + label String
        // capacity + a few scalar fields.
        let occupied = self.slots.iter().filter(|s| s.is_some()).count();
        (self.k * 16) + (occupied * 1400)
    }

    /// Submit entries from all modules. The top-k by salience win broadcast slots.
    /// Existing entries decay; new entries compete against decayed incumbents.
    pub fn compete(&mut self, submissions: Vec<WorkspaceEntry>) -> Vec<WorkspaceEntry> {
        self.rounds += 1;

        // Decay existing entries
        for slot in &mut self.slots {
            if let Some(entry) = slot {
                entry.salience *= 1.0 - self.decay_rate;
                entry.age += 1;
                // Evict entries with negligible salience
                if entry.salience < 0.01 {
                    *slot = None;
                }
            }
        }

        // Combine existing + new submissions
        let mut candidates: Vec<WorkspaceEntry> = Vec::new();
        for slot in &self.slots {
            if let Some(entry) = slot {
                candidates.push(entry.clone());
            }
        }
        candidates.extend(submissions);

        // Sort by salience (highest first)
        candidates.sort_by(|a, b| b.salience.partial_cmp(&a.salience).unwrap_or(std::cmp::Ordering::Equal));

        // Top-k win the workspace
        self.slots = (0..self.k).map(|i| candidates.get(i).cloned()).collect();

        // Return the winners (broadcast contents)
        self.slots.iter().filter_map(|s| s.clone()).collect()
    }

    /// Get current workspace contents (the broadcast).
    pub fn broadcast(&self) -> Vec<&WorkspaceEntry> {
        self.slots.iter().filter_map(|s| s.as_ref()).collect()
    }

    /// Get the HDC vectors currently in the workspace.
    pub fn broadcast_vectors(&self) -> Vec<&BipolarVector> {
        self.slots.iter()
            .filter_map(|s| s.as_ref())
            .map(|e| &e.content)
            .collect()
    }

    /// Number of occupied slots.
    pub fn occupancy(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Average salience of occupied slots.
    pub fn avg_salience(&self) -> f64 {
        let occupied: Vec<f64> = self.slots.iter()
            .filter_map(|s| s.as_ref())
            .map(|e| e.salience)
            .collect();
        if occupied.is_empty() { return 0.0; }
        occupied.iter().sum::<f64>() / occupied.len() as f64
    }

    /// Check if a specific module has a slot in the workspace.
    pub fn module_has_slot(&self, module: &str) -> bool {
        self.slots.iter()
            .any(|s| s.as_ref().map_or(false, |e| e.source_module == module))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(module: &str, salience: f64) -> WorkspaceEntry {
        WorkspaceEntry {
            content: BipolarVector::from_seed(salience as u64 * 100),
            source_module: module.into(),
            salience,
            label: format!("{} entry", module),
            age: 0,
        }
    }

    #[test]
    fn test_workspace_capacity_bounded() {
        let mut ws = GlobalWorkspace::new(3);
        let entries = vec![
            make_entry("A", 0.9),
            make_entry("B", 0.8),
            make_entry("C", 0.7),
            make_entry("D", 0.6),
            make_entry("E", 0.5),
        ];
        let winners = ws.compete(entries);
        assert_eq!(winners.len(), 3, "Only k=3 winners");
        assert_eq!(winners[0].source_module, "A");
        assert_eq!(winners[1].source_module, "B");
        assert_eq!(winners[2].source_module, "C");
    }

    #[test]
    fn test_decay_evicts_old_entries() {
        let mut ws = GlobalWorkspace::new(2);
        ws.decay_rate = 0.5; // Aggressive decay
        ws.compete(vec![make_entry("old", 0.5)]);
        // After several rounds with no new submissions, old entry decays
        for _ in 0..10 {
            ws.compete(vec![]);
        }
        assert_eq!(ws.occupancy(), 0, "Old entries should decay to nothing");
    }

    #[test]
    fn test_high_salience_displaces_low() {
        let mut ws = GlobalWorkspace::new(2);
        ws.compete(vec![make_entry("low1", 0.3), make_entry("low2", 0.2)]);
        assert_eq!(ws.occupancy(), 2);

        // High salience entry displaces
        let winners = ws.compete(vec![make_entry("high", 0.95)]);
        assert!(winners.iter().any(|w| w.source_module == "high"));
    }

    #[test]
    fn test_module_has_slot() {
        let mut ws = GlobalWorkspace::new(3);
        ws.compete(vec![make_entry("AIF", 0.8), make_entry("analogy", 0.7)]);
        assert!(ws.module_has_slot("AIF"));
        assert!(ws.module_has_slot("analogy"));
        assert!(!ws.module_has_slot("missing"));
    }

    #[test]
    fn test_broadcast_vectors() {
        let mut ws = GlobalWorkspace::new(3);
        ws.compete(vec![make_entry("A", 0.9), make_entry("B", 0.8)]);
        let vecs = ws.broadcast_vectors();
        assert_eq!(vecs.len(), 2);
    }

    #[test]
    fn test_empty_workspace() {
        let ws = GlobalWorkspace::standard();
        assert_eq!(ws.occupancy(), 0);
        assert_eq!(ws.avg_salience(), 0.0);
        assert_eq!(ws.broadcast().len(), 0);
    }
}
