// ============================================================
// QoS (Quality of Service) — Build-Time & Runtime Compliance
//
// PURPOSE: Enforces coding policies, architectural constraints,
// and runtime health invariants across the sovereign substrate.
//
// POLICIES ENFORCED:
//   1. Memory Safety: #![forbid(unsafe_code)] (compile-time, lib.rs)
//   2. Error Handling: No .unwrap()/.expect() in non-test code
//   3. PII Protection: ForbiddenSpaceAxiom + HDLM intercept
//   4. Telemetry Coverage: debuglog! in all public functions
//   5. PSL Compliance: Axiom pass rate must exceed threshold
//   6. Thermal Compliance: CPU temp under hard limit
//   7. VSA Health: Orthogonality within tolerance
//   8. Identity Compliance: Sovereign auth required for mutations
// ============================================================

use crate::telemetry::MaterialAuditor;
use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};
use serde::{Serialize, Deserialize};

/// QoS policy thresholds — configurable per deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QosPolicy {
    /// Minimum PSL axiom pass rate (0.0 - 1.0)
    pub min_axiom_pass_rate: f64,
    /// Maximum CPU temperature (Celsius) before forced throttle
    pub max_cpu_temp_c: f64,
    /// Maximum VSA aliasing threshold (mean similarity)
    pub max_vsa_aliasing: f64,
    /// Minimum available RAM (MB) for BigBrain tier
    pub min_ram_bigbrain_mb: u64,
    /// Minimum available RAM (MB) for Bridge tier
    pub min_ram_bridge_mb: u64,
    /// Maximum response latency (ms) before degradation warning
    pub max_latency_ms: u64,
}

impl Default for QosPolicy {
    fn default() -> Self {
        debuglog!("QosPolicy::default: Loading sovereign QoS thresholds");
        Self {
            min_axiom_pass_rate: 0.95,
            max_cpu_temp_c: 80.0,
            max_vsa_aliasing: 0.10,
            min_ram_bigbrain_mb: 6000,
            min_ram_bridge_mb: 2000,
            max_latency_ms: 5000,
        }
    }
}

/// Individual QoS check result
#[derive(Debug, Clone, Serialize)]
pub struct QosCheck {
    pub name: String,
    pub passed: bool,
    pub value: String,
    pub threshold: String,
    pub severity: QosSeverity,
}

#[derive(Debug, Clone, Serialize)]
pub enum QosSeverity {
    Critical,
    Warning,
    Info,
}

/// Full QoS audit report
#[derive(Debug, Clone, Serialize)]
pub struct QosReport {
    pub checks: Vec<QosCheck>,
    pub passed: bool,
    pub critical_failures: usize,
    pub warnings: usize,
}

/// The QoS Auditor — enforces compliance across the substrate
pub struct QosAuditor {
    pub policy: QosPolicy,
}

impl QosAuditor {
    pub fn new() -> Self {
        debuglog!("QosAuditor::new: Initializing with default policy");
        Self {
            policy: QosPolicy::default(),
        }
    }

    pub fn with_policy(policy: QosPolicy) -> Self {
        debuglog!("QosAuditor::with_policy: Custom QoS policy loaded");
        Self { policy }
    }

    /// Run a full QoS audit against the current substrate state
    pub fn audit(&self, axiom_pass_rate: f64) -> QosReport {
        debuglog!("QosAuditor::audit: Beginning QoS compliance sweep...");

        let mut checks = Vec::new();

        // 1. PSL Axiom Compliance
        let axiom_passed = axiom_pass_rate >= self.policy.min_axiom_pass_rate;
        debuglog!("QosAuditor::audit: PSL check — passed={}, rate={:.2}", axiom_passed, axiom_pass_rate);
        checks.push(QosCheck {
            name: "PSL Axiom Pass Rate".to_string(),
            passed: axiom_passed,
            value: format!("{:.1}%", axiom_pass_rate * 100.0),
            threshold: format!(">= {:.1}%", self.policy.min_axiom_pass_rate * 100.0),
            severity: if axiom_passed { QosSeverity::Info } else { QosSeverity::Critical },
        });

        // 2. Thermal Compliance
        let probe_hv = HyperMemory::new(DIM_PROLETARIAT);
        let vsa_ortho = probe_hv.audit_orthogonality();
        let stats = MaterialAuditor::get_stats(vsa_ortho, axiom_pass_rate);

        let cpu_temp = stats.cpu_temp_c as f64;
        let thermal_passed = cpu_temp <= self.policy.max_cpu_temp_c;
        debuglog!("QosAuditor::audit: Thermal check — passed={}, temp={:.0}", thermal_passed, cpu_temp);
        checks.push(QosCheck {
            name: "Thermal Compliance".to_string(),
            passed: thermal_passed,
            value: format!("{:.0}C", cpu_temp),
            threshold: format!("<= {:.0}C", self.policy.max_cpu_temp_c),
            severity: if !thermal_passed {
                QosSeverity::Critical
            } else if cpu_temp > self.policy.max_cpu_temp_c * 0.85 {
                QosSeverity::Warning
            } else {
                QosSeverity::Info
            },
        });

        // 3. VSA Aliasing (Orthogonality Health)
        let vsa_passed = vsa_ortho <= self.policy.max_vsa_aliasing;
        debuglog!("QosAuditor::audit: VSA check — passed={}, ortho={:.4}", vsa_passed, vsa_ortho);
        checks.push(QosCheck {
            name: "VSA Orthogonality".to_string(),
            passed: vsa_passed,
            value: format!("{:.4}", vsa_ortho),
            threshold: format!("<= {:.2}", self.policy.max_vsa_aliasing),
            severity: if !vsa_passed { QosSeverity::Warning } else { QosSeverity::Info },
        });

        // 4. RAM Availability
        let ram_passed = stats.ram_available_mb >= self.policy.min_ram_bridge_mb;
        debuglog!("QosAuditor::audit: RAM check — passed={}, available={}MB", ram_passed, stats.ram_available_mb);
        checks.push(QosCheck {
            name: "RAM Availability".to_string(),
            passed: ram_passed,
            value: format!("{}MB", stats.ram_available_mb),
            threshold: format!(">= {}MB (Bridge)", self.policy.min_ram_bridge_mb),
            severity: if !ram_passed {
                QosSeverity::Critical
            } else if stats.ram_available_mb < self.policy.min_ram_bigbrain_mb {
                QosSeverity::Warning
            } else {
                QosSeverity::Info
            },
        });

        // 5. Throttle Status
        let throttle_passed = !stats.is_throttled;
        debuglog!("QosAuditor::audit: Throttle check — passed={}", throttle_passed);
        checks.push(QosCheck {
            name: "Throttle Status".to_string(),
            passed: throttle_passed,
            value: if stats.is_throttled { "THROTTLED".to_string() } else { "NOMINAL".to_string() },
            threshold: "Not throttled".to_string(),
            severity: if stats.is_throttled { QosSeverity::Warning } else { QosSeverity::Info },
        });

        // 6. Memory Safety (compile-time — always passes if we got here)
        debuglog!("QosAuditor::audit: Memory safety — compile-time enforced");
        checks.push(QosCheck {
            name: "Memory Safety (forbid(unsafe_code))".to_string(),
            passed: true,
            value: "ENFORCED".to_string(),
            threshold: "forbid(unsafe_code)".to_string(),
            severity: QosSeverity::Info,
        });

        // Compute summary
        let critical_failures = checks.iter()
            .filter(|c| !c.passed && matches!(c.severity, QosSeverity::Critical))
            .count();
        let warnings = checks.iter()
            .filter(|c| !c.passed && matches!(c.severity, QosSeverity::Warning))
            .count();
        let all_passed = critical_failures == 0;

        debuglog!("QosAuditor::audit: Complete — passed={}, critical={}, warnings={}", all_passed, critical_failures, warnings);

        QosReport {
            checks,
            passed: all_passed,
            critical_failures,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_default_policy() {
        let policy = QosPolicy::default();
        assert!(policy.min_axiom_pass_rate > 0.0, "Axiom rate threshold must be positive");
        assert!(policy.max_cpu_temp_c > 0.0, "Thermal threshold must be positive");
        assert!(policy.max_vsa_aliasing > 0.0, "VSA aliasing threshold must be positive");
        assert!(policy.min_ram_bigbrain_mb > policy.min_ram_bridge_mb, "BigBrain needs more RAM than Bridge");
    }

    #[test]
    fn test_qos_audit_nominal() {
        // Use relaxed policy — real system may be PRoot with <1GB RAM.
        // Thermal threshold raised: laptops during testing can hit 80°C+
        // under load (gaming, compilation). This tests the logic, not the
        // specific environmental state.
        let policy = QosPolicy {
            min_ram_bridge_mb: 64,   // PRoot-safe threshold
            min_ram_bigbrain_mb: 256,
            max_cpu_temp_c: 95.0,    // Allow hot test environments
            ..QosPolicy::default()
        };
        let auditor = QosAuditor::with_policy(policy);
        let report = auditor.audit(1.0);
        assert!(report.passed, "Nominal conditions should pass QoS with relaxed policy");
        assert_eq!(report.critical_failures, 0, "No critical failures expected");
    }

    #[test]
    fn test_qos_audit_low_axiom_rate() {
        let auditor = QosAuditor::new();
        let report = auditor.audit(0.5);
        let psl_check = report.checks.iter().find(|c| c.name.contains("PSL")).unwrap();
        assert!(!psl_check.passed, "Low axiom rate should fail PSL check");
    }

    #[test]
    fn test_qos_memory_safety_always_passes() {
        let auditor = QosAuditor::new();
        let report = auditor.audit(1.0);
        let safety = report.checks.iter().find(|c| c.name.contains("Memory Safety")).unwrap();
        assert!(safety.passed, "Memory safety is compile-time enforced");
    }

    #[test]
    fn test_qos_custom_policy() {
        let policy = QosPolicy {
            min_axiom_pass_rate: 0.99,
            max_cpu_temp_c: 60.0,
            max_vsa_aliasing: 0.05,
            min_ram_bigbrain_mb: 8000,
            min_ram_bridge_mb: 3000,
            max_latency_ms: 2000,
        };
        let auditor = QosAuditor::with_policy(policy);
        let report = auditor.audit(0.995);
        let psl_check = report.checks.iter().find(|c| c.name.contains("PSL")).unwrap();
        assert!(psl_check.passed, "High axiom rate should pass custom policy PSL check");
    }

    #[test]
    fn test_qos_report_structure() {
        let auditor = QosAuditor::new();
        let report = auditor.audit(1.0);
        // Should have at least 6 checks.
        assert!(report.checks.len() >= 6, "QoS should run at least 6 checks, got {}", report.checks.len());
        // Every check should have a name and value.
        for check in &report.checks {
            assert!(!check.name.is_empty());
            assert!(!check.value.is_empty());
            assert!(!check.threshold.is_empty());
        }
    }

    #[test]
    fn test_qos_zero_axiom_rate() {
        let auditor = QosAuditor::new();
        let report = auditor.audit(0.0);
        assert!(report.critical_failures > 0, "Zero axiom rate must be critical");
    }

    #[test]
    fn test_qos_policy_serialization() {
        let policy = QosPolicy::default();
        let json = serde_json::to_string(&policy).unwrap();
        let recovered: QosPolicy = serde_json::from_str(&json).unwrap();
        assert!((policy.min_axiom_pass_rate - recovered.min_axiom_pass_rate).abs() < 0.001);
        assert!((policy.max_cpu_temp_c - recovered.max_cpu_temp_c).abs() < 0.001);
    }

    // ============================================================
    // Stress / invariant tests for QosAuditor
    // ============================================================

    /// INVARIANT: audit always produces at least 6 checks (per documented
    /// policies 1-6).
    #[test]
    fn invariant_audit_produces_six_checks() {
        let auditor = QosAuditor::new();
        for rate in [0.0, 0.5, 0.95, 1.0] {
            let report = auditor.audit(rate);
            assert!(report.checks.len() >= 6,
                "audit should produce >= 6 checks, got {} at rate={}",
                report.checks.len(), rate);
        }
    }

    /// INVARIANT: audit never panics regardless of input axiom_pass_rate.
    #[test]
    fn invariant_audit_never_panics() {
        let auditor = QosAuditor::new();
        for rate in [0.0, 0.5, 1.0, -1.0, 2.0, f64::NAN, f64::INFINITY] {
            let _ = auditor.audit(rate);
        }
    }

    /// INVARIANT: critical_failures + warnings <= total failing checks.
    #[test]
    fn invariant_failure_counts_consistent() {
        let auditor = QosAuditor::new();
        for rate in [0.0, 0.5, 0.95, 1.0] {
            let report = auditor.audit(rate);
            let failing = report.checks.iter().filter(|c| !c.passed).count();
            assert!(report.critical_failures + report.warnings <= failing,
                "critical+warnings {} > failing {}",
                report.critical_failures + report.warnings, failing);
        }
    }

    /// INVARIANT: Report's passed flag matches (critical_failures == 0).
    #[test]
    fn invariant_passed_iff_no_critical_failures() {
        let auditor = QosAuditor::new();
        for rate in [0.0, 0.5, 0.95, 1.0] {
            let report = auditor.audit(rate);
            assert_eq!(report.passed, report.critical_failures == 0,
                "passed flag inconsistent at rate={}: passed={}, critical={}",
                rate, report.passed, report.critical_failures);
        }
    }

    /// INVARIANT: Default policy is deserializable from its own JSON.
    #[test]
    fn invariant_policy_serde_roundtrip_all_fields() -> Result<(), serde_json::Error> {
        let original = QosPolicy::default();
        let json = serde_json::to_string(&original)?;
        let recovered: QosPolicy = serde_json::from_str(&json)?;
        assert_eq!(original.min_axiom_pass_rate, recovered.min_axiom_pass_rate);
        assert_eq!(original.max_cpu_temp_c, recovered.max_cpu_temp_c);
        assert_eq!(original.max_vsa_aliasing, recovered.max_vsa_aliasing);
        assert_eq!(original.min_ram_bigbrain_mb, recovered.min_ram_bigbrain_mb);
        assert_eq!(original.min_ram_bridge_mb, recovered.min_ram_bridge_mb);
        assert_eq!(original.max_latency_ms, recovered.max_latency_ms);
        Ok(())
    }

    /// INVARIANT: Memory Safety check always passes (compile-time enforced).
    #[test]
    fn invariant_memory_safety_always_passes() {
        let auditor = QosAuditor::new();
        for rate in [0.0, 0.5, 1.0] {
            let report = auditor.audit(rate);
            let safety = report.checks.iter()
                .find(|c| c.name.contains("Memory Safety"))
                .expect("Memory Safety check should exist");
            assert!(safety.passed,
                "Memory Safety is compile-time enforced but not passing at rate={}", rate);
        }
    }
}
