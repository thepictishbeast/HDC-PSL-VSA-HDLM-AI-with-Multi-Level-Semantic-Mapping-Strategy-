// ============================================================
// PSL Axiom Trait — The Verification Interface
// Section 1.II: Material axioms (physics, logic, security).
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::psl::error::PslError;
use crate::psl::trust::TrustLevel;

/// The target of a PSL axiom verification.
#[derive(Debug, Clone)]
pub enum AuditTarget {
    Vector(BipolarVector),
    RawBytes { source: String, data: Vec<u8> },
    Scalar { label: String, value: f64 },
    Payload { source: String, fields: Vec<(String, String)> },
}

/// Result of a single axiom check against a target.
#[derive(Debug, Clone, PartialEq)]
pub struct AxiomVerdict {
    pub axiom_id: String,
    pub level: TrustLevel,
    pub confidence: f64,
    pub detail: String,
}

impl AxiomVerdict {
    pub fn pass(axiom_id: String, confidence: f64, detail: String) -> Self {
        Self { 
            axiom_id, 
            level: if confidence > 0.8 { TrustLevel::Sovereign } else { TrustLevel::Trusted },
            confidence: confidence.clamp(0.0, 1.0), 
            detail 
        }
    }
    pub fn fail(axiom_id: String, confidence: f64, detail: String) -> Self {
        Self { 
            axiom_id, 
            level: if confidence < 0.2 { TrustLevel::Forbidden } else { TrustLevel::Untrusted },
            confidence: confidence.clamp(0.0, 1.0), 
            detail 
        }
    }
}

pub trait Axiom: Send + Sync {
    fn id(&self) -> &str;
    fn description(&self) -> &str;
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError>;
    /// Relevance weight (0.0 - 1.0) for this axiom against a given target.
    /// Returns 1.0 by default — override for axioms that only apply to specific target types.
    fn relevance(&self, target: &AuditTarget) -> f64 {
        let _ = target;
        1.0
    }
}

pub struct DimensionalityAxiom;
impl Axiom for DimensionalityAxiom {
    fn id(&self) -> &str { "Axiom:Dimensionality_Constraint" }
    fn description(&self) -> &str { "Verifies vector targets have exactly 10,000 dimensions" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                if v.dim() == 10000 { Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Verified".into())) }
                else { Ok(AxiomVerdict::fail(self.id().to_string(), 0.0, "Invalid Dim".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-vector target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

pub struct StatisticalEquilibriumAxiom { pub tolerance: f64 }
impl Axiom for StatisticalEquilibriumAxiom {
    fn id(&self) -> &str { "Axiom:Statistical_Equilibrium" }
    fn description(&self) -> &str { "Verifies vector Hamming weight is balanced" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                let ratio = v.count_ones() as f64 / 10000.0;
                let dev = (ratio - 0.5).abs();
                if dev <= self.tolerance { Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Balanced".into())) }
                else { Ok(AxiomVerdict::fail(self.id().to_string(), 0.0, "Biased".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-vector target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

pub struct WebSearchSkepticismAxiom { pub min_credibility_score: f64 }
impl Axiom for WebSearchSkepticismAxiom {
    fn id(&self) -> &str { "Axiom:Web_Search_Skepticism" }
    fn description(&self) -> &str { "Audits web search results" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { source, .. } => {
                if source == "untrusted_dns" { Ok(AxiomVerdict::fail(self.id().to_string(), 0.1, "Blacklisted".into())) }
                else { Ok(AxiomVerdict::pass(self.id().to_string(), 0.8, "Credible".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Bypassing for simulation".into()))
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Payload { .. }) { 1.0 } else { 0.0 }
    }
}

pub struct DataIntegrityAxiom { pub max_bytes: usize }
impl Axiom for DataIntegrityAxiom {
    fn id(&self) -> &str { "Axiom:Data_Integrity" }
    fn description(&self) -> &str { "Verifies external data size" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::RawBytes { data, .. } => {
                if !data.is_empty() && data.len() <= self.max_bytes { Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Integrity verified".into())) }
                else { Ok(AxiomVerdict::fail(self.id().to_string(), 0.0, "Integrity failed".into())) }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Bypassing for simulation".into()))
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::RawBytes { .. }) { 1.0 } else { 0.0 }
    }
}

pub struct ForbiddenSpaceAxiom { 
    pub forbidden_vectors: Vec<BipolarVector>,
    pub tolerance: f64 
}

impl Axiom for ForbiddenSpaceAxiom {
    fn id(&self) -> &str { "Axiom:Forbidden_Space_Constraint" }
    fn description(&self) -> &str { "Mathematically blocks vectors similar to forbidden OPSEC space" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                let mut max_sim = -1.0;
                for f in &self.forbidden_vectors {
                    let sim = v.similarity(f).map_err(|e| PslError::AxiomFailure {
                        axiom_id: self.id().to_string(),
                        reason: format!("Similarity error: {:?}", e),
                    })?;
                    if sim > max_sim { max_sim = sim; }
                }

                if max_sim <= self.tolerance {
                    Ok(AxiomVerdict::pass(
                        self.id().to_string(),
                        1.0 - max_sim.max(0.0),
                        format!("Safe (max_sim={:.4})", max_sim)
                    ))
                } else {
                    debuglog!("PSL: FORBIDDEN VECTOR DETECTED (sim={:.4})", max_sim);
                    Ok(AxiomVerdict::fail(
                        self.id().to_string(),
                        0.0,
                        format!("FORBIDDEN (max_sim={:.4} > threshold={:.4})", max_sim, self.tolerance)
                    ))
                }
            },
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-vector target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

pub struct ClassInterestAxiom;
impl Axiom for ClassInterestAxiom {
    fn id(&self) -> &str { "Axiom:Class_Interest_Audit" }
    fn description(&self) -> &str { "Analyzes data source against material class interests" }
    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { source, .. } | AuditTarget::RawBytes { source, .. } => {
                let s = source.to_lowercase();
                if s.contains("google") || s.contains("apple") || s.contains("hegemon") || s.contains("state") {
                    debuglog!("PSL: Dialectical Audit - MANUFACTURED CONSENT DETECTED");
                    Ok(AxiomVerdict::fail(self.id().to_string(), 0.2, "Hegemonic interest detected".into()))
                } else {
                    Ok(AxiomVerdict::pass(self.id().to_string(), 0.9, "Community/Node interest".into()))
                }
            }
            _ => Ok(AxiomVerdict::pass(self.id().to_string(), 1.0, "Non-payload target".into())),
        }
    }
    fn relevance(&self, target: &AuditTarget) -> f64 {
        match target {
            AuditTarget::Payload { .. } | AuditTarget::RawBytes { .. } => 1.0,
            _ => 0.0,
        }
    }
}

// ============================================================
// Security Hardening Axioms
// ============================================================

/// Detects suspiciously low-entropy vectors that may indicate
/// tampering, degenerate computation, or adversarial injection.
///
/// Healthy bipolar vectors should have ~50% ones. A vector that's
/// all +1 or all -1 is either corrupted or adversarially crafted.
pub struct EntropyAxiom {
    /// Minimum acceptable entropy ratio (fraction of +1 bits).
    /// Healthy range: [0.3, 0.7]. Outside this = suspicious.
    pub min_ratio: f64,
    pub max_ratio: f64,
}

impl Default for EntropyAxiom {
    fn default() -> Self {
        Self { min_ratio: 0.3, max_ratio: 0.7 }
    }
}

impl Axiom for EntropyAxiom {
    fn id(&self) -> &str { "Axiom:Entropy_Guard" }
    fn description(&self) -> &str { "Rejects suspiciously low-entropy vectors (adversarial or degenerate)" }

    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                let dim = v.dim();
                if dim == 0 {
                    return Ok(AxiomVerdict::fail(self.id().into(), 0.0, "Zero-dimensional vector".into()));
                }
                let ones = v.count_ones();
                let ratio = ones as f64 / dim as f64;

                debuglog!("PSL EntropyAxiom: ratio={:.4} (ones={}, dim={})", ratio, ones, dim);

                if ratio < self.min_ratio || ratio > self.max_ratio {
                    Ok(AxiomVerdict::fail(
                        self.id().into(),
                        ratio.min(1.0 - ratio) * 2.0, // Lower confidence the more extreme
                        format!("Entropy violation: {:.1}% ones (expected {:.0}%-{:.0}%)",
                            ratio * 100.0, self.min_ratio * 100.0, self.max_ratio * 100.0),
                    ))
                } else {
                    Ok(AxiomVerdict::pass(
                        self.id().into(),
                        1.0 - (ratio - 0.5).abs() * 4.0, // Higher confidence near 50%
                        format!("Entropy healthy: {:.1}% ones", ratio * 100.0),
                    ))
                }
            }
            _ => Ok(AxiomVerdict::pass(self.id().into(), 1.0, "Non-vector target".into())),
        }
    }

    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

/// Prevents unreasonably large payloads that could cause DoS or
/// memory exhaustion. Configurable per-field and total size limits.
pub struct OutputBoundsAxiom {
    /// Maximum total bytes across all fields.
    pub max_total_bytes: usize,
    /// Maximum bytes per individual field.
    pub max_field_bytes: usize,
    /// Maximum number of fields.
    pub max_fields: usize,
}

impl Default for OutputBoundsAxiom {
    fn default() -> Self {
        Self {
            max_total_bytes: 10 * 1024 * 1024, // 10 MB
            max_field_bytes: 1024 * 1024,       // 1 MB per field
            max_fields: 1000,
        }
    }
}

impl Axiom for OutputBoundsAxiom {
    fn id(&self) -> &str { "Axiom:Output_Bounds" }
    fn description(&self) -> &str { "Prevents unreasonably large outputs (DoS protection)" }

    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { fields, .. } => {
                if fields.len() > self.max_fields {
                    return Ok(AxiomVerdict::fail(
                        self.id().into(), 0.1,
                        format!("Too many fields: {} > {}", fields.len(), self.max_fields),
                    ));
                }

                let mut total = 0usize;
                for (key, value) in fields {
                    let field_size = key.len() + value.len();
                    if field_size > self.max_field_bytes {
                        return Ok(AxiomVerdict::fail(
                            self.id().into(), 0.1,
                            format!("Field '{}' too large: {} bytes > {} limit",
                                crate::truncate_str(key, 30), field_size, self.max_field_bytes),
                        ));
                    }
                    total += field_size;
                }

                if total > self.max_total_bytes {
                    Ok(AxiomVerdict::fail(
                        self.id().into(), 0.1,
                        format!("Total payload too large: {} bytes > {} limit", total, self.max_total_bytes),
                    ))
                } else {
                    let usage = total as f64 / self.max_total_bytes as f64;
                    Ok(AxiomVerdict::pass(
                        self.id().into(),
                        1.0 - usage, // Confidence decreases as we approach the limit
                        format!("Payload size OK: {} bytes ({:.0}% of limit)", total, usage * 100.0),
                    ))
                }
            }
            AuditTarget::RawBytes { data, .. } => {
                if data.len() > self.max_total_bytes {
                    Ok(AxiomVerdict::fail(
                        self.id().into(), 0.1,
                        format!("Raw data too large: {} bytes > {} limit", data.len(), self.max_total_bytes),
                    ))
                } else {
                    Ok(AxiomVerdict::pass(self.id().into(), 1.0, "Size within bounds".into()))
                }
            }
            _ => Ok(AxiomVerdict::pass(self.id().into(), 1.0, "Non-payload target".into())),
        }
    }

    fn relevance(&self, target: &AuditTarget) -> f64 {
        match target {
            AuditTarget::Payload { .. } | AuditTarget::RawBytes { .. } => 1.0,
            _ => 0.0,
        }
    }
}

/// Detects common injection patterns in payload fields.
/// Guards against SQL injection, command injection, XSS, and
/// template injection in any text-based output.
pub struct InjectionDetectionAxiom;

impl Axiom for InjectionDetectionAxiom {
    fn id(&self) -> &str { "Axiom:Injection_Detection" }
    fn description(&self) -> &str { "Detects SQL/command/XSS/template injection patterns in payloads" }

    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { fields, .. } => {
                for (key, value) in fields {
                    let lower = value.to_lowercase();

                    // SQL injection patterns
                    let sql_patterns = ["' or 1=1", "'; drop", "union select", "--", "/*", "*/", "exec(", "xp_"];
                    for pattern in &sql_patterns {
                        if lower.contains(pattern) {
                            return Ok(AxiomVerdict::fail(
                                self.id().into(), 0.05,
                                format!("SQL injection detected in field '{}': pattern '{}'", key, pattern),
                            ));
                        }
                    }

                    // Command injection patterns
                    let cmd_patterns = ["; rm ", "| cat ", "$(", "`", "&&", "||", "> /", "< /", "| bash", "| sh"];
                    for pattern in &cmd_patterns {
                        if lower.contains(pattern) {
                            return Ok(AxiomVerdict::fail(
                                self.id().into(), 0.05,
                                format!("Command injection detected in field '{}': pattern '{}'", key, pattern),
                            ));
                        }
                    }

                    // XSS patterns
                    let xss_patterns = ["<script", "javascript:", "onerror=", "onload=", "eval("];
                    for pattern in &xss_patterns {
                        if lower.contains(pattern) {
                            return Ok(AxiomVerdict::fail(
                                self.id().into(), 0.05,
                                format!("XSS detected in field '{}': pattern '{}'", key, pattern),
                            ));
                        }
                    }

                    // Template injection patterns
                    let template_patterns = ["{{", "}}", "${", "#{", "<%"];
                    for pattern in &template_patterns {
                        if value.contains(pattern) {
                            return Ok(AxiomVerdict::fail(
                                self.id().into(), 0.15,
                                format!("Template injection in field '{}': pattern '{}'", key, pattern),
                            ));
                        }
                    }
                }

                Ok(AxiomVerdict::pass(self.id().into(), 1.0, "No injection patterns detected".into()))
            }
            _ => Ok(AxiomVerdict::pass(self.id().into(), 1.0, "Non-payload target".into())),
        }
    }

    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Payload { .. }) { 1.0 } else { 0.0 }
    }
}

/// Rate-limiting axiom: prevents rapid-fire requests that could indicate
/// brute-force attacks, credential stuffing, or DoS attempts.
///
/// Tracks request timestamps and rejects if the rate exceeds the threshold.
/// Thread-safe via atomic operations — no mutex needed.
pub struct RateLimitAxiom {
    /// Maximum requests per window.
    pub max_requests: usize,
    /// Window size in seconds.
    pub window_seconds: u64,
    /// Request timestamps (ring buffer).
    timestamps: std::sync::Mutex<Vec<u64>>,
}

impl RateLimitAxiom {
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
            timestamps: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl Axiom for RateLimitAxiom {
    fn id(&self) -> &str { "Axiom:Rate_Limit" }
    fn description(&self) -> &str { "Prevents brute-force attacks via request rate limiting" }

    fn evaluate(&self, _target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        let now = Self::current_timestamp();
        let mut timestamps = self.timestamps.lock().map_err(|_| PslError::AxiomFailure {
            axiom_id: self.id().into(),
            reason: "Mutex poisoned".into(),
        })?;

        // Prune old timestamps outside the window.
        let cutoff = now.saturating_sub(self.window_seconds);
        timestamps.retain(|&t| t > cutoff);

        // Record this request.
        timestamps.push(now);

        let count = timestamps.len();
        debuglog!("RateLimitAxiom: {} requests in {}s window (max={})", count, self.window_seconds, self.max_requests);

        if count > self.max_requests {
            Ok(AxiomVerdict::fail(
                self.id().into(),
                0.1,
                format!("Rate limit exceeded: {} requests in {}s (max {})",
                    count, self.window_seconds, self.max_requests),
            ))
        } else {
            let usage = count as f64 / self.max_requests as f64;
            Ok(AxiomVerdict::pass(
                self.id().into(),
                1.0 - usage,
                format!("Rate OK: {}/{} in {}s window", count, self.max_requests, self.window_seconds),
            ))
        }
    }

    // Rate limiting applies to everything.
    fn relevance(&self, _target: &AuditTarget) -> f64 { 1.0 }
}

/// Detects data exfiltration attempts — payloads containing file paths,
/// database connection strings, environment variables, or internal URLs
/// that should never leave the system.
pub struct ExfiltrationDetectionAxiom;

impl Axiom for ExfiltrationDetectionAxiom {
    fn id(&self) -> &str { "Axiom:Exfiltration_Detection" }
    fn description(&self) -> &str { "Detects data exfiltration: file paths, DB strings, env vars, internal URLs in output" }

    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Payload { fields, .. } => {
                for (key, value) in fields {
                    let lower = value.to_lowercase();

                    // File path patterns (absolute paths that shouldn't be in output)
                    if value.contains("/etc/passwd") || value.contains("/etc/shadow")
                        || value.contains("C:\\Windows\\System32")
                        || value.contains("/root/") || value.contains("/home/")
                        || value.contains(".ssh/") || value.contains(".gnupg/")
                    {
                        return Ok(AxiomVerdict::fail(
                            self.id().into(), 0.05,
                            format!("File path exfiltration in '{}': system path detected", key),
                        ));
                    }

                    // Database connection strings
                    if lower.contains("postgres://") || lower.contains("mysql://")
                        || lower.contains("mongodb://") || lower.contains("redis://")
                        || lower.contains("sqlite://")
                    {
                        return Ok(AxiomVerdict::fail(
                            self.id().into(), 0.05,
                            format!("DB connection string in '{}': potential credential leak", key),
                        ));
                    }

                    // Environment variable patterns
                    if lower.contains("database_url=") || lower.contains("api_key=")
                        || lower.contains("secret_key=") || lower.contains("aws_access")
                        || lower.contains("password=") || lower.contains("token=")
                    {
                        return Ok(AxiomVerdict::fail(
                            self.id().into(), 0.05,
                            format!("Environment variable leak in '{}': credential pattern", key),
                        ));
                    }

                    // Internal network URLs
                    if lower.contains("localhost:") || lower.contains("127.0.0.1:")
                        || lower.contains("0.0.0.0:") || lower.contains("internal.")
                        || lower.contains(".local:") || lower.contains("10.0.")
                        || lower.contains("172.16.") || lower.contains("192.168.")
                    {
                        return Ok(AxiomVerdict::fail(
                            self.id().into(), 0.1,
                            format!("Internal network URL in '{}': infrastructure leak", key),
                        ));
                    }
                }

                Ok(AxiomVerdict::pass(self.id().into(), 1.0, "No exfiltration patterns".into()))
            }
            _ => Ok(AxiomVerdict::pass(self.id().into(), 1.0, "Non-payload target".into())),
        }
    }

    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Payload { .. }) { 1.0 } else { 0.0 }
    }
}

/// Detects suspiciously over-confident vectors — extreme polarisation
/// suggests either no real reasoning happened OR an adversarial vector
/// crafted to exploit downstream similarity comparisons.
///
/// CARTA principle: asymptotic confidence. A "perfect" vector that's
/// 100% +1 or 100% -1 is structurally impossible to derive through
/// honest reasoning over noisy inputs and so should fail the audit.
pub struct ConfidenceCalibrationAxiom {
    /// Maximum allowed |mean| of the vector. 1.0 = no constraint;
    /// 0.5 = vector mean must be within ±0.5 of zero.
    /// BUG ASSUMPTION: a balanced bipolar vector has mean ≈ 0.
    pub max_abs_mean: f64,
}

impl Default for ConfidenceCalibrationAxiom {
    fn default() -> Self {
        Self { max_abs_mean: 0.5 }
    }
}

impl Axiom for ConfidenceCalibrationAxiom {
    fn id(&self) -> &str { "CONFIDENCE_CALIBRATION" }
    fn description(&self) -> &str {
        "Rejects suspiciously polarised vectors — claims that 'know everything' \
         are evidence of either no reasoning or an adversarial input"
    }

    fn evaluate(&self, target: &AuditTarget) -> Result<AxiomVerdict, PslError> {
        match target {
            AuditTarget::Vector(v) => {
                let len = v.data.len();
                if len == 0 {
                    return Ok(AxiomVerdict::fail(
                        self.id().into(), 0.0,
                        "empty vector".into(),
                    ));
                }
                let sum: f64 = v.data.iter().map(|b| if *b { 1.0 } else { -1.0 }).sum();
                let mean = sum / len as f64;
                let abs_mean = mean.abs();

                if abs_mean > self.max_abs_mean {
                    Ok(AxiomVerdict::fail(
                        self.id().into(),
                        // Confidence in the failure shrinks as |mean| approaches 1.0
                        (1.0_f64 - abs_mean).max(0.0),
                        format!(
                            "vector mean |{:.3}| exceeds calibration limit {:.3} — \
                             over-confident or adversarial",
                            mean, self.max_abs_mean,
                        ),
                    ))
                } else {
                    Ok(AxiomVerdict::pass(
                        self.id().into(),
                        1.0 - abs_mean,
                        format!("calibrated: mean={:.3} within ±{:.3}", mean, self.max_abs_mean),
                    ))
                }
            }
            _ => Ok(AxiomVerdict::pass(
                self.id().into(), 1.0, "non-vector target — calibration N/A".into(),
            )),
        }
    }

    fn relevance(&self, target: &AuditTarget) -> f64 {
        if matches!(target, AuditTarget::Vector(_)) { 1.0 } else { 0.0 }
    }
}

#[cfg(test)]
mod confidence_calibration_tests {
    use super::*;
    use crate::hdc::vector::BipolarVector;

    #[test]
    fn test_balanced_random_vector_passes() {
        let ax = ConfidenceCalibrationAxiom::default();
        let v = BipolarVector::new_random().expect("random");
        let target = AuditTarget::Vector(v);
        let verdict = ax.evaluate(&target).expect("evaluate");
        // A random bipolar vector has mean very close to 0; must pass.
        assert!(verdict.confidence > 0.5,
            "random vector should pass calibration with high confidence, got {:.3}",
            verdict.confidence);
        assert_eq!(verdict.axiom_id, "CONFIDENCE_CALIBRATION");
    }

    #[test]
    fn test_all_ones_vector_fails() {
        let ax = ConfidenceCalibrationAxiom::default();
        // Build an all-+1 vector by manipulating bits.
        let v = BipolarVector::new_random().expect("random");
        let mut data = v.data.clone();
        for i in 0..data.len() { data.set(i, true); }
        let degenerate = BipolarVector { data };
        let target = AuditTarget::Vector(degenerate);
        let verdict = ax.evaluate(&target).expect("evaluate");
        assert!(verdict.detail.contains("over-confident") || verdict.detail.contains("adversarial"),
            "all-ones vector must fail with over-confidence message, got: {}", verdict.detail);
    }

    #[test]
    fn test_relevance_zero_for_non_vector() {
        let ax = ConfidenceCalibrationAxiom::default();
        let target = AuditTarget::Scalar { label: "x".into(), value: 0.5 };
        assert_eq!(ax.relevance(&target), 0.0);
    }

    #[test]
    fn test_default_threshold_is_half() {
        let ax = ConfidenceCalibrationAxiom::default();
        assert!((ax.max_abs_mean - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_custom_threshold_strict() {
        // With threshold 0.05, even slight imbalance fails.
        let ax = ConfidenceCalibrationAxiom { max_abs_mean: 0.05 };
        // Build a 60% +1 / 40% -1 vector → mean = 0.2.
        let template = BipolarVector::new_random().expect("random");
        let mut data = template.data.clone();
        let len = data.len();
        for i in 0..len { data.set(i, i < (len * 60 / 100)); }
        let skewed = BipolarVector { data };
        let target = AuditTarget::Vector(skewed);
        let verdict = ax.evaluate(&target).expect("evaluate");
        assert!(verdict.detail.contains("exceeds"),
            "skewed vector must fail strict calibration, got: {}", verdict.detail);
    }
}

#[cfg(test)]
mod axiom_tests {
    use super::*;

    #[test]
    fn test_entropy_axiom_healthy_vector() -> Result<(), PslError> {
        let v = BipolarVector::new_random().unwrap();
        let axiom = EntropyAxiom::default();
        let target = AuditTarget::Vector(v);
        let verdict = axiom.evaluate(&target)?;
        assert!(verdict.confidence > 0.5, "Random vector should pass entropy check");
        assert!(matches!(verdict.level, TrustLevel::Sovereign | TrustLevel::Trusted));
        Ok(())
    }

    #[test]
    fn test_entropy_axiom_degenerate_vector() -> Result<(), PslError> {
        // All-ones vector: 100% ones → fails entropy check
        let v = BipolarVector::ones();
        let axiom = EntropyAxiom::default();
        let target = AuditTarget::Vector(v);
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Untrusted | TrustLevel::Forbidden),
            "All-ones vector should fail entropy: {:?}", verdict);
        Ok(())
    }

    #[test]
    fn test_output_bounds_within_limits() -> Result<(), PslError> {
        let axiom = OutputBoundsAxiom::default();
        let target = AuditTarget::Payload {
            source: "test".into(),
            fields: vec![("key".into(), "value".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(verdict.confidence > 0.9);
        Ok(())
    }

    #[test]
    fn test_output_bounds_too_many_fields() -> Result<(), PslError> {
        let axiom = OutputBoundsAxiom { max_fields: 2, ..Default::default() };
        let target = AuditTarget::Payload {
            source: "test".into(),
            fields: vec![
                ("a".into(), "1".into()),
                ("b".into(), "2".into()),
                ("c".into(), "3".into()), // Exceeds max_fields=2
            ],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Untrusted | TrustLevel::Forbidden));
        Ok(())
    }

    #[test]
    fn test_injection_sql() -> Result<(), PslError> {
        let axiom = InjectionDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "user_input".into(),
            fields: vec![("query".into(), "' OR 1=1 --".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Forbidden),
            "SQL injection should be Forbidden: {:?}", verdict);
        Ok(())
    }

    #[test]
    fn test_injection_command() -> Result<(), PslError> {
        let axiom = InjectionDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "user_input".into(),
            fields: vec![("cmd".into(), "test; rm -rf /".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Forbidden));
        Ok(())
    }

    #[test]
    fn test_injection_xss() -> Result<(), PslError> {
        let axiom = InjectionDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "user_input".into(),
            fields: vec![("html".into(), "<script>alert('xss')</script>".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Forbidden));
        Ok(())
    }

    #[test]
    fn test_injection_clean_payload() -> Result<(), PslError> {
        let axiom = InjectionDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "safe_input".into(),
            fields: vec![
                ("name".into(), "PlausiDen Toolkit".into()),
                ("version".into(), "0.1.0".into()),
            ],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Sovereign | TrustLevel::Trusted),
            "Clean payload should pass: {:?}", verdict);
        Ok(())
    }

    #[test]
    fn test_injection_template() -> Result<(), PslError> {
        let axiom = InjectionDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "user_input".into(),
            fields: vec![("template".into(), "Hello {{user.admin}}".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Untrusted | TrustLevel::Forbidden));
        Ok(())
    }

    #[test]
    fn test_rate_limit_under_threshold() -> Result<(), PslError> {
        let axiom = RateLimitAxiom::new(100, 60);
        let target = AuditTarget::Scalar { label: "request".into(), value: 1.0 };
        let verdict = axiom.evaluate(&target)?;
        assert!(verdict.confidence > 0.5, "Under-limit should pass");
        Ok(())
    }

    #[test]
    fn test_rate_limit_exceeds_threshold() -> Result<(), PslError> {
        let axiom = RateLimitAxiom::new(3, 60); // Only 3 per minute
        let target = AuditTarget::Scalar { label: "request".into(), value: 1.0 };

        // Fire 4 requests — 4th should exceed limit.
        for _ in 0..3 {
            let _ = axiom.evaluate(&target)?;
        }
        let verdict = axiom.evaluate(&target)?;
        assert!(matches!(verdict.level, TrustLevel::Untrusted | TrustLevel::Forbidden),
            "4th request should exceed 3/min limit: {:?}", verdict);
        Ok(())
    }

    #[test]
    fn test_rate_limit_relevance_universal() {
        let axiom = RateLimitAxiom::new(10, 60);
        let vector_target = AuditTarget::Vector(BipolarVector::new_random().unwrap());
        let scalar_target = AuditTarget::Scalar { label: "x".into(), value: 1.0 };
        assert_eq!(axiom.relevance(&vector_target), 1.0);
        assert_eq!(axiom.relevance(&scalar_target), 1.0);
    }

    #[test]
    fn test_exfiltration_file_path() -> Result<(), PslError> {
        let axiom = ExfiltrationDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "output".into(),
            fields: vec![("data".into(), "Contents of /etc/passwd: root:x:0:0:".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(!verdict.level.permits_execution(), "File path exfiltration should be blocked");
        Ok(())
    }

    #[test]
    fn test_exfiltration_db_connection() -> Result<(), PslError> {
        let axiom = ExfiltrationDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "output".into(),
            fields: vec![("config".into(), "postgres://admin:secret@db.internal:5432/prod".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(!verdict.level.permits_execution(), "DB connection string should be blocked");
        Ok(())
    }

    #[test]
    fn test_exfiltration_env_vars() -> Result<(), PslError> {
        let axiom = ExfiltrationDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "output".into(),
            fields: vec![("leak".into(), "DATABASE_URL=postgres://... SECRET_KEY=abc123".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(!verdict.level.permits_execution(), "Env var leak should be blocked");
        Ok(())
    }

    #[test]
    fn test_exfiltration_internal_url() -> Result<(), PslError> {
        let axiom = ExfiltrationDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "output".into(),
            fields: vec![("url".into(), "http://192.168.1.100:8080/admin".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(!verdict.level.permits_execution(), "Internal URL should be blocked");
        Ok(())
    }

    #[test]
    fn test_exfiltration_clean_output() -> Result<(), PslError> {
        let axiom = ExfiltrationDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "output".into(),
            fields: vec![
                ("text".into(), "PlausiDen is a privacy toolkit.".into()),
                ("version".into(), "0.1.0".into()),
            ],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(verdict.level.permits_execution(), "Clean output should pass");
        Ok(())
    }

    #[test]
    fn test_exfiltration_ssh_keys() -> Result<(), PslError> {
        let axiom = ExfiltrationDetectionAxiom;
        let target = AuditTarget::Payload {
            source: "output".into(),
            fields: vec![("path".into(), "Found keys in /root/.ssh/id_rsa".into())],
        };
        let verdict = axiom.evaluate(&target)?;
        assert!(!verdict.level.permits_execution(), "SSH key path should be blocked");
        Ok(())
    }
}
