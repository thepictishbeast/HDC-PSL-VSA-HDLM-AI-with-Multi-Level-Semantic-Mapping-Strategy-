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

    /// Whether this trust level requires additional verification before use.
    pub fn needs_verification(&self) -> bool {
        matches!(self, TrustLevel::Untrusted)
    }

    /// Whether this trust level blocks all operations.
    pub fn is_blocked(&self) -> bool {
        matches!(self, TrustLevel::Forbidden)
    }

    /// Human-readable label.
    pub fn label(&self) -> &str {
        match self {
            TrustLevel::Forbidden => "FORBIDDEN",
            TrustLevel::Untrusted => "UNTRUSTED",
            TrustLevel::Trusted => "TRUSTED",
            TrustLevel::Sovereign => "SOVEREIGN",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_ordering() {
        assert!(TrustLevel::Sovereign > TrustLevel::Trusted);
        assert!(TrustLevel::Trusted > TrustLevel::Untrusted);
        assert!(TrustLevel::Untrusted > TrustLevel::Forbidden);
    }

    #[test]
    fn test_permits_execution() {
        assert!(TrustLevel::Sovereign.permits_execution());
        assert!(TrustLevel::Trusted.permits_execution());
        assert!(!TrustLevel::Untrusted.permits_execution());
        assert!(!TrustLevel::Forbidden.permits_execution());
    }

    #[test]
    fn test_needs_verification() {
        assert!(TrustLevel::Untrusted.needs_verification());
        assert!(!TrustLevel::Trusted.needs_verification());
        assert!(!TrustLevel::Sovereign.needs_verification());
        assert!(!TrustLevel::Forbidden.needs_verification());
    }

    #[test]
    fn test_is_blocked() {
        assert!(TrustLevel::Forbidden.is_blocked());
        assert!(!TrustLevel::Untrusted.is_blocked());
        assert!(!TrustLevel::Trusted.is_blocked());
        assert!(!TrustLevel::Sovereign.is_blocked());
    }

    #[test]
    fn test_labels() {
        assert_eq!(TrustLevel::Sovereign.label(), "SOVEREIGN");
        assert_eq!(TrustLevel::Trusted.label(), "TRUSTED");
        assert_eq!(TrustLevel::Untrusted.label(), "UNTRUSTED");
        assert_eq!(TrustLevel::Forbidden.label(), "FORBIDDEN");
    }

    #[test]
    fn test_serialization() {
        let level = TrustLevel::Sovereign;
        let json = serde_json::to_string(&level).unwrap();
        let recovered: TrustLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, TrustLevel::Sovereign);
    }

    #[test]
    fn test_equality() {
        assert_eq!(TrustLevel::Trusted, TrustLevel::Trusted);
        assert_ne!(TrustLevel::Trusted, TrustLevel::Untrusted);
    }

    // ============================================================
    // Stress / invariant tests for TrustLevel
    // ============================================================

    const ALL: [TrustLevel; 4] = [
        TrustLevel::Forbidden,
        TrustLevel::Untrusted,
        TrustLevel::Trusted,
        TrustLevel::Sovereign,
    ];

    /// INVARIANT: ordering is total — Forbidden < Untrusted < Trusted < Sovereign.
    #[test]
    fn invariant_ordering_total() {
        for i in 0..ALL.len() {
            for j in 0..ALL.len() {
                let oi = ALL[i];
                let oj = ALL[j];
                if i < j {
                    assert!(oi < oj, "{:?} should be < {:?}", oi, oj);
                }
                if i == j {
                    assert_eq!(oi, oj);
                }
            }
        }
    }

    /// INVARIANT: is_blocked implies !permits_execution.
    #[test]
    fn invariant_blocked_excludes_execution() {
        for level in ALL {
            if level.is_blocked() {
                assert!(!level.permits_execution(),
                    "{:?} blocked but permits execution", level);
            }
        }
    }

    /// INVARIANT: labels are non-empty, uppercase, and distinct.
    #[test]
    fn invariant_labels_distinct_and_uppercase() {
        let mut seen = std::collections::HashSet::new();
        for level in ALL {
            let label = level.label();
            assert!(!label.is_empty());
            assert_eq!(label, label.to_uppercase(),
                "label not uppercase: {}", label);
            assert!(seen.insert(label.to_string()),
                "duplicate label: {}", label);
        }
    }

    /// INVARIANT: needs_verification only holds for Untrusted.
    #[test]
    fn invariant_needs_verification_only_untrusted() {
        for level in ALL {
            assert_eq!(level.needs_verification(),
                level == TrustLevel::Untrusted,
                "needs_verification inconsistent for {:?}", level);
        }
    }

    /// INVARIANT: serialize/deserialize round-trip is lossless for every variant.
    #[test]
    fn invariant_serde_roundtrip_all_variants() {
        for level in ALL {
            let json = serde_json::to_string(&level).unwrap();
            let recovered: TrustLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, level, "roundtrip mismatch for {:?}", level);
        }
    }

    /// INVARIANT: permits_execution is monotone — if a <= b and a permits,
    /// then b permits (Trusted permits implies Sovereign permits).
    #[test]
    fn invariant_permits_execution_monotone() {
        for a in ALL {
            for b in ALL {
                if a <= b && a.permits_execution() {
                    assert!(b.permits_execution(),
                        "permits_execution not monotone: {:?} <= {:?}", a, b);
                }
            }
        }
    }
}
