// ============================================================
// HDLM Intercept — Pre-Vectorization OPSEC Firewall
// Section 1.I: "AST parser executes a localized entropy and regex sweep."
// ============================================================

use crate::debuglog;
use crate::hdlm::error::HdlmError;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A sanitized result from the intercept.
pub struct InterceptResult {
    pub original: String,
    pub sanitized: String,
    pub matches_found: Vec<String>,
}

/// The HDLM Intercept engine.
pub struct OpsecIntercept;

impl OpsecIntercept {
    /// Scans a string for OPSEC markers and performs substitution.
    /// Returns Result to propagate regex compilation errors safely.
    pub fn scan(input: &str) -> Result<InterceptResult, HdlmError> {
        debuglog!("OpsecIntercept: Scanning for identity markers...");

        let mut sanitized = input.to_string();
        let mut matches = Vec::new();

        // 1. Regex Sweep: 9-digit integers (SSN)
        let ssn_regex = r"\b\d{9}\b";
        let re_ssn = regex::Regex::new(ssn_regex).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: format!("SSN regex compilation failed: {}", e),
        })?;
        for m in re_ssn.find_iter(&sanitized.clone()) {
            let s: &str = m.as_str();
            debuglog!("OpsecIntercept: MATCH FOUND (SSN Topology)");
            matches.push(s.to_string());
            sanitized = sanitized.replace(s, &Self::hash_marker(s));
        }

        // 2. Regex Sweep: License Topology (e.g. s23233305)
        let license_regex = r"\b[a-zA-Z]\d{8}\b";
        let re_lic = regex::Regex::new(license_regex).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: format!("License regex compilation failed: {}", e),
        })?;
        for m in re_lic.find_iter(&sanitized.clone()) {
            let s: &str = m.as_str();
            debuglog!("OpsecIntercept: MATCH FOUND (License Topology)");
            matches.push(s.to_string());
            sanitized = sanitized.replace(s, &Self::hash_marker(s));
        }

        // 3. Entropy Sweep: (Placeholder for complex entropy analysis)
        // High entropy blocks often indicate passwords or keys.
        debuglog!("OpsecIntercept: Scan complete — {} markers found", matches.len());

        Ok(InterceptResult {
            original: input.to_string(),
            sanitized,
            matches_found: matches,
        })
    }

    /// Hashes a sensitive marker into a ZK-style placeholder.
    fn hash_marker(marker: &str) -> String {
        debuglog!("OpsecIntercept: Hashing marker to ZKP placeholder");
        let mut hasher = DefaultHasher::new();
        marker.hash(&mut hasher);
        format!("ZKP_REDACTED_{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssn_interception() -> Result<(), HdlmError> {
        let input = "The user with SSN 647568607 is authenticated.";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.matches_found.contains(&"647568607".to_string()));
        assert!(!result.sanitized.contains("647568607"));
        assert!(result.sanitized.contains("ZKP_REDACTED_"));
        Ok(())
    }

    #[test]
    fn test_license_interception() -> Result<(), HdlmError> {
        let input = "License number: s23233305.";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.matches_found.contains(&"s23233305".to_string()));
        assert!(!result.sanitized.contains("s23233305"));
        assert!(result.sanitized.contains("ZKP_REDACTED_"));
        Ok(())
    }
}
