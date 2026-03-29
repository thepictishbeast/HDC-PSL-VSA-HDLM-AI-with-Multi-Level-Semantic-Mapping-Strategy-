// ============================================================
// Trust Level Hierarchy — The Sovereign Audit Tier
// ============================================================

use serde::{Serialize, Deserialize};

/// Hierarchy of trust for material and neural data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Level 0: Adversarial/Blocked.
    Forbidden,
    /// Level 1: Low-confidence/Untrusted.
    Untrusted,
    /// Level 2: High-confidence/Standard data.
    Trusted,
    /// Level 3: Verified material truth.
    Sovereign,
}

impl TrustLevel {
    pub fn permits_execution(&self) -> bool {
        match self {
            TrustLevel::Trusted | TrustLevel::Sovereign => true,
            _ => false,
        }
    }
}
