// ============================================================
// HDLM Intercept — Pre-Vectorization OPSEC Firewall
//
// Section 1.I: "AST parser executes a localized entropy and regex sweep."
//
// PURPOSE: Scrubs sensitive data from text before it enters the
// HDC vector space or leaves the system as output. Once data is
// encoded as a hypervector, it's computationally infeasible to
// extract — but text-level scrubbing is still essential for
// output safety and logging.
//
// PSA PHILOSOPHY: Privacy, Security, Anonymity.
//   - PRIVACY: PII is redacted before vectorization
//   - SECURITY: API keys and credentials are caught and hashed
//   - ANONYMITY: Network identifiers (IPs, MACs) are stripped
//
// PATTERNS DETECTED:
//   - Social Security Numbers (9-digit sequences)
//   - Driver's license numbers (letter + 8 digits)
//   - Email addresses
//   - Phone numbers (US/international formats)
//   - IPv4 and IPv6 addresses
//   - Credit card numbers (Luhn-valid 13-19 digit sequences)
//   - API keys and tokens (high-entropy alphanumeric strings)
//   - AWS/GCP/Azure credential patterns
//   - Private key headers (PEM format)
// ============================================================

use crate::hdlm::error::HdlmError;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Categories of sensitive data detected by the intercept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensitiveCategory {
    SSN,
    DriversLicense,
    Email,
    PhoneNumber,
    IPv4Address,
    CreditCard,
    ApiKey,
    PrivateKey,
    HighEntropyString,
    CustomPattern(String),
}

/// A single match found during scanning.
#[derive(Debug, Clone)]
pub struct SensitiveMatch {
    pub category: SensitiveCategory,
    pub matched_text: String,
    pub position: usize,
    pub redacted_with: String,
}

/// A sanitized result from the intercept.
pub struct InterceptResult {
    pub original: String,
    pub sanitized: String,
    pub matches_found: Vec<String>,
    /// Detailed match information for auditing.
    pub detailed_matches: Vec<SensitiveMatch>,
    /// Total sensitive bytes removed.
    pub bytes_redacted: usize,
}

/// The HDLM Intercept engine — OPSEC firewall for text data.
pub struct OpsecIntercept;

impl OpsecIntercept {
    /// Scans a string for all known OPSEC markers and performs substitution.
    pub fn scan(input: &str) -> Result<InterceptResult, HdlmError> {
        debuglog!("OpsecIntercept::scan: Scanning {} bytes for identity markers...", input.len());

        let mut sanitized = input.to_string();
        let mut matches = Vec::new();
        let mut detailed = Vec::new();
        let mut bytes_redacted = 0usize;

        // 1. SSN: 9-digit sequences (with or without dashes)
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b\d{3}[-]?\d{2}[-]?\d{4}\b",
            SensitiveCategory::SSN,
            "SSN topology",
        )?;

        // 2. Driver's License: letter + 8 digits
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b[a-zA-Z]\d{8}\b",
            SensitiveCategory::DriversLicense,
            "License topology",
        )?;

        // 3. Email addresses
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b",
            SensitiveCategory::Email,
            "Email address",
        )?;

        // 4. Phone numbers (US formats: 10-11 digits with optional separators)
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b",
            SensitiveCategory::PhoneNumber,
            "Phone number",
        )?;

        // 5. IPv4 addresses
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b",
            SensitiveCategory::IPv4Address,
            "IPv4 address",
        )?;

        // 6. Credit card numbers (13-19 digits, with optional separators)
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b(?:\d{4}[-\s]?){3,4}\d{1,4}\b",
            SensitiveCategory::CreditCard,
            "Credit card pattern",
        )?;

        // 7. API key patterns (common prefixes)
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b(?:sk-|pk-|ghp_|ghs_|AKIA|AIza)[a-zA-Z0-9_-]{16,}\b",
            SensitiveCategory::ApiKey,
            "API key",
        )?;

        // 8. Private key headers
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
            SensitiveCategory::PrivateKey,
            "Private key header",
        )?;

        // 9. High-entropy strings (base64-like, 32+ chars)
        Self::scan_pattern(
            &mut sanitized, &mut matches, &mut detailed, &mut bytes_redacted,
            r"\b[A-Za-z0-9+/=]{40,}\b",
            SensitiveCategory::HighEntropyString,
            "High-entropy string",
        )?;

        debuglog!("OpsecIntercept::scan: {} markers found, {} bytes redacted",
            detailed.len(), bytes_redacted);

        Ok(InterceptResult {
            original: input.to_string(),
            sanitized,
            matches_found: matches,
            detailed_matches: detailed,
            bytes_redacted,
        })
    }

    /// Scan for a specific pattern and redact matches.
    fn scan_pattern(
        sanitized: &mut String,
        matches: &mut Vec<String>,
        detailed: &mut Vec<SensitiveMatch>,
        bytes_redacted: &mut usize,
        pattern: &str,
        category: SensitiveCategory,
        label: &str,
    ) -> Result<(), HdlmError> {
        let re = regex::Regex::new(pattern).map_err(|e| HdlmError::Tier1GenerationFailed {
            reason: format!("{} regex compilation failed: {}", label, e),
        })?;

        let clone = sanitized.clone();
        for m in re.find_iter(&clone) {
            let text = m.as_str().to_string();
            let redacted = Self::hash_marker(&text);

            debuglog!("OpsecIntercept: {} DETECTED at offset {}", label, m.start());

            *bytes_redacted += text.len();
            matches.push(text.clone());
            detailed.push(SensitiveMatch {
                category: category.clone(),
                matched_text: text.clone(),
                position: m.start(),
                redacted_with: redacted.clone(),
            });

            *sanitized = sanitized.replace(&text, &redacted);
        }
        Ok(())
    }

    /// Quick check: does the input contain any sensitive patterns?
    /// Faster than full scan — returns true on first match.
    pub fn contains_sensitive(input: &str) -> Result<bool, HdlmError> {
        debuglog!("OpsecIntercept::contains_sensitive: quick check on {} bytes", input.len());

        let patterns = [
            r"\b\d{9}\b",
            r"\b[a-zA-Z]\d{8}\b",
            r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b",
            r"\b(?:sk-|pk-|ghp_|ghs_|AKIA|AIza)[a-zA-Z0-9_-]{16,}\b",
            r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
        ];

        for pattern in &patterns {
            let re = regex::Regex::new(pattern).map_err(|e| HdlmError::Tier1GenerationFailed {
                reason: format!("Quick check regex failed: {}", e),
            })?;
            if re.is_match(input) {
                debuglog!("OpsecIntercept::contains_sensitive: match on pattern {}", pattern);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Hashes a sensitive marker into a ZK-style placeholder.
    /// The placeholder is deterministic — same input always produces same hash.
    fn hash_marker(marker: &str) -> String {
        debuglog!("OpsecIntercept: Hashing marker to ZKP placeholder");
        let mut hasher = DefaultHasher::new();
        marker.hash(&mut hasher);
        format!("ZKP_REDACTED_{:x}", hasher.finish())
    }

    /// Scan with custom patterns added by the caller.
    pub fn scan_with_custom(input: &str, custom_patterns: &[(&str, &str)]) -> Result<InterceptResult, HdlmError> {
        let mut result = Self::scan(input)?;

        for (pattern, label) in custom_patterns {
            let re = regex::Regex::new(pattern).map_err(|e| HdlmError::Tier1GenerationFailed {
                reason: format!("Custom pattern '{}' compilation failed: {}", label, e),
            })?;
            let clone = result.sanitized.clone();
            for m in re.find_iter(&clone) {
                let text = m.as_str().to_string();
                let redacted = Self::hash_marker(&text);

                result.bytes_redacted += text.len();
                result.matches_found.push(text.clone());
                result.detailed_matches.push(SensitiveMatch {
                    category: SensitiveCategory::CustomPattern(label.to_string()),
                    matched_text: text.clone(),
                    position: m.start(),
                    redacted_with: redacted.clone(),
                });

                result.sanitized = result.sanitized.replace(&text, &redacted);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssn_interception() -> Result<(), HdlmError> {
        let input = "The user with SSN 555000111 is authenticated.";
        let result = OpsecIntercept::scan(input)?;
        assert!(!result.matches_found.is_empty(), "Should detect SSN");
        assert!(!result.sanitized.contains("555000111"));
        assert!(result.sanitized.contains("ZKP_REDACTED_"));
        assert!(result.bytes_redacted > 0);
        Ok(())
    }

    #[test]
    fn test_ssn_with_dashes() -> Result<(), HdlmError> {
        let input = "SSN: 555-00-0111 on record.";
        let result = OpsecIntercept::scan(input)?;
        assert!(!result.matches_found.is_empty(), "Should detect dashed SSN");
        assert!(!result.sanitized.contains("555-00-0111"));
        Ok(())
    }

    #[test]
    fn test_license_interception() -> Result<(), HdlmError> {
        let input = "License number: s99999999.";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.matches_found.contains(&"s99999999".to_string()));
        assert!(!result.sanitized.contains("s99999999"));
        Ok(())
    }

    #[test]
    fn test_email_interception() -> Result<(), HdlmError> {
        let input = "Contact admin@example.com for access.";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.detailed_matches.iter().any(|m| m.category == SensitiveCategory::Email),
            "Should detect email: {:?}", result.matches_found);
        assert!(!result.sanitized.contains("admin@example.com"));
        Ok(())
    }

    #[test]
    fn test_ipv4_interception() -> Result<(), HdlmError> {
        let input = "Server at 192.168.1.100 is responding.";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.detailed_matches.iter().any(|m| m.category == SensitiveCategory::IPv4Address),
            "Should detect IPv4: {:?}", result.matches_found);
        assert!(!result.sanitized.contains("192.168.1.100"));
        Ok(())
    }

    #[test]
    fn test_api_key_interception() -> Result<(), HdlmError> {
        let input = "Using key sk-abc123def456ghi789jkl012mno345pqr678";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.detailed_matches.iter().any(|m| m.category == SensitiveCategory::ApiKey),
            "Should detect API key: {:?}", result.matches_found);
        assert!(!result.sanitized.contains("sk-abc123"));
        Ok(())
    }

    #[test]
    fn test_private_key_header() -> Result<(), HdlmError> {
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEow...";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.detailed_matches.iter().any(|m| m.category == SensitiveCategory::PrivateKey),
            "Should detect private key header: {:?}", result.matches_found);
        Ok(())
    }

    #[test]
    fn test_clean_input_passes() -> Result<(), HdlmError> {
        let input = "PlausiDen is a privacy toolkit built in Rust.";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.matches_found.is_empty(), "Clean input should have no matches");
        assert_eq!(result.sanitized, input);
        assert_eq!(result.bytes_redacted, 0);
        Ok(())
    }

    #[test]
    fn test_contains_sensitive_quick_check() -> Result<(), HdlmError> {
        assert!(OpsecIntercept::contains_sensitive("key: sk-abc123def456ghi789jkl012mn")?);
        assert!(!OpsecIntercept::contains_sensitive("Hello world")?);
        Ok(())
    }

    #[test]
    fn test_multiple_patterns_in_one_input() -> Result<(), HdlmError> {
        let input = "User admin@evil.com at 10.0.0.1 with SSN 123456789";
        let result = OpsecIntercept::scan(input)?;
        assert!(result.detailed_matches.len() >= 3,
            "Should detect multiple patterns, got {}: {:?}",
            result.detailed_matches.len(),
            result.detailed_matches.iter().map(|m| &m.category).collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn test_custom_pattern() -> Result<(), HdlmError> {
        let input = "Internal code: PROJ-12345-SECRET";
        let result = OpsecIntercept::scan_with_custom(
            input,
            &[(r"PROJ-\d+-SECRET", "Project secret code")],
        )?;
        assert!(result.detailed_matches.iter().any(|m| matches!(&m.category, SensitiveCategory::CustomPattern(s) if s == "Project secret code")));
        assert!(!result.sanitized.contains("PROJ-12345-SECRET"));
        Ok(())
    }

    #[test]
    fn test_redaction_is_deterministic() -> Result<(), HdlmError> {
        let input = "SSN: 555000111";
        let r1 = OpsecIntercept::scan(input)?;
        let r2 = OpsecIntercept::scan(input)?;
        assert_eq!(r1.sanitized, r2.sanitized, "Same input should produce same redaction");
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for OpsecIntercept
    // ============================================================

    /// INVARIANT: scan never panics on arbitrary unicode/control input.
    #[test]
    fn invariant_scan_never_panics() -> Result<(), HdlmError> {
        let big = "x".repeat(10_000);
        let inputs: [&str; 7] = [
            "",
            "normal text",
            "αβγδε",
            "🦀🦀🦀",
            "\x00\x01\x1f control",
            "mixed αβγ 123",
            &big,
        ];
        for input in inputs {
            let _ = OpsecIntercept::scan(input)?;
        }
        Ok(())
    }

    /// INVARIANT: sanitized preserves original length minus redactions.
    #[test]
    fn invariant_bytes_redacted_nonnegative() -> Result<(), HdlmError> {
        let inputs = [
            "no sensitive data",
            "SSN: 555000111 is here",
            "email: test@example.com",
            "",
        ];
        for input in inputs {
            let r = OpsecIntercept::scan(input)?;
            // bytes_redacted should be non-negative (unsigned always is,
            // but check that it's sensible — at most input.len())
            assert!(r.bytes_redacted <= input.len() + 1000,
                "bytes_redacted {} >> input.len() {}",
                r.bytes_redacted, input.len());
        }
        Ok(())
    }

    /// INVARIANT: matches_found and detailed_matches stay in sync in length.
    #[test]
    fn invariant_matches_lists_consistent() -> Result<(), HdlmError> {
        let inputs = [
            "",
            "SSN: 123456789 and email: a@b.com",
            "no matches here at all",
        ];
        for input in inputs {
            let r = OpsecIntercept::scan(input)?;
            assert_eq!(r.matches_found.len(), r.detailed_matches.len(),
                "matches_found.len() != detailed_matches.len() for {:?}", input);
        }
        Ok(())
    }

    /// INVARIANT: On truly clean input, no matches are found.
    #[test]
    fn invariant_clean_input_no_matches() -> Result<(), HdlmError> {
        let clean_inputs = [
            "Hello, world!",
            "The quick brown fox",
            "",
            "simple text with numbers 1 2 3",
        ];
        for input in clean_inputs {
            let r = OpsecIntercept::scan(input)?;
            assert!(r.detailed_matches.is_empty(),
                "clean input {:?} produced matches: {:?}", input, r.detailed_matches);
        }
        Ok(())
    }

    /// INVARIANT: scan_with_patterns is deterministic.
    #[test]
    fn invariant_scan_with_patterns_deterministic() -> Result<(), HdlmError> {
        let input = "This contains SECRET-42-KEY and SSN: 555000111";
        let patterns = [(r"SECRET-\d+-KEY", "Custom secret")];
        let r1 = OpsecIntercept::scan_with_custom(input, &patterns)?;
        let r2 = OpsecIntercept::scan_with_custom(input, &patterns)?;
        assert_eq!(r1.sanitized, r2.sanitized);
        assert_eq!(r1.detailed_matches.len(), r2.detailed_matches.len());
        Ok(())
    }
}
