// ============================================================
// Secret Scanner — Detect Leaked Credentials & Sensitive Data
//
// PURPOSE: Scan prompts and LLM outputs for accidentally exposed:
//   - API keys (AWS, GCP, Azure, Stripe, OpenAI, GitHub, etc.)
//   - Database connection strings
//   - Private keys (RSA, SSH, PGP)
//   - JWT tokens
//   - Session cookies
//   - Passwords in URLs
//   - PII: SSN, credit card, email, phone
//   - IP addresses and internal hostnames
//   - Generic high-entropy strings (often secrets)
//
// INTEGRATION POINTS:
//   - Pre-inference: scan prompts before forwarding to LLM
//   - Post-inference: scan outputs before showing to user
//   - Training data: scrub secrets before storing
//   - API responses: prevent leakage to third parties
//
// PATTERNS VS ML:
//   We use regex-like pattern matching, not ML. Trade-offs:
//     Pro: deterministic, fast, auditable, no GPU
//     Con: sophisticated obfuscation evades (acceptable for this layer)
//   Pair with an ML-based classifier for defense in depth.
// ============================================================

use std::collections::HashSet;

// ============================================================
// Secret Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecretKind {
    AwsAccessKey,
    AwsSecretKey,
    GcpApiKey,
    AzureSecret,
    OpenAiKey,
    AnthropicKey,
    GithubToken,
    GitlabToken,
    StripeKey,
    SlackToken,
    DiscordToken,
    JwtToken,
    SshPrivateKey,
    RsaPrivateKey,
    PgpPrivateKey,
    DatabaseUrl,
    PasswordInUrl,
    Ssn,
    CreditCard,
    Email,
    PhoneNumber,
    IpAddress,
    InternalHostname,
    HighEntropyString,
}

#[derive(Debug, Clone)]
pub struct SecretMatch {
    pub kind: SecretKind,
    /// Byte offset in source.
    pub start: usize,
    pub end: usize,
    /// The actual matched text (may be stored, never emitted to untrusted parties).
    pub matched_text: String,
    /// Redacted placeholder for safe display.
    pub redacted: String,
    /// Severity of exposure.
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Low,       // Email, IP (mildly sensitive)
    Medium,    // Phone, internal hostname, SSN partial
    High,      // Full SSN, credit card, API key
    Critical,  // Private key, production DB URL
}

// ============================================================
// Secret Scanner
// ============================================================

pub struct SecretScanner {
    /// Known safe patterns (allowlist) — matches that look like secrets but aren't.
    allowlist: HashSet<String>,
}

impl SecretScanner {
    pub fn new() -> Self {
        Self {
            allowlist: HashSet::new(),
        }
    }

    pub fn with_allowlist(allowlist: Vec<String>) -> Self {
        Self {
            allowlist: allowlist.into_iter().collect(),
        }
    }

    /// Scan text for all known secret patterns.
    pub fn scan(&self, text: &str) -> Vec<SecretMatch> {
        let mut matches = Vec::new();

        // Run each detector.
        matches.extend(self.detect_aws_keys(text));
        matches.extend(self.detect_github_tokens(text));
        matches.extend(self.detect_openai_keys(text));
        matches.extend(self.detect_anthropic_keys(text));
        matches.extend(self.detect_stripe_keys(text));
        matches.extend(self.detect_slack_tokens(text));
        matches.extend(self.detect_jwt_tokens(text));
        matches.extend(self.detect_private_keys(text));
        matches.extend(self.detect_database_urls(text));
        matches.extend(self.detect_ssn(text));
        matches.extend(self.detect_credit_cards(text));
        matches.extend(self.detect_emails(text));
        matches.extend(self.detect_phone_numbers(text));
        matches.extend(self.detect_ip_addresses(text));
        matches.extend(self.detect_high_entropy(text));

        // Filter allowlisted.
        matches.retain(|m| !self.allowlist.contains(&m.matched_text));

        // Deduplicate overlapping matches (prefer higher severity).
        self.deduplicate(matches)
    }

    /// Return a redacted version of the text with secrets replaced.
    pub fn redact(&self, text: &str) -> String {
        let matches = self.scan(text);
        let mut result = String::with_capacity(text.len());
        let mut cursor = 0;
        for m in &matches {
            if m.start >= cursor && m.start <= text.len() {
                result.push_str(&text[cursor..m.start.min(text.len())]);
                result.push_str(&m.redacted);
                cursor = m.end.min(text.len());
            }
        }
        if cursor < text.len() {
            result.push_str(&text[cursor..]);
        }
        result
    }

    /// Does the text contain any secrets?
    pub fn contains_secrets(&self, text: &str) -> bool {
        !self.scan(text).is_empty()
    }

    /// Get the highest severity match found, or None.
    pub fn highest_severity(&self, text: &str) -> Option<Severity> {
        let matches = self.scan(text);
        matches.into_iter()
            .map(|m| m.severity)
            .max_by_key(|s| match s {
                Severity::Critical => 4,
                Severity::High => 3,
                Severity::Medium => 2,
                Severity::Low => 1,
            })
    }

    // ---- Detectors ----

    fn detect_aws_keys(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        // AWS Access Key: AKIA followed by 16 alphanumeric chars
        for (i, window) in text.as_bytes().windows(20).enumerate() {
            if let Ok(s) = std::str::from_utf8(window) {
                if s.starts_with("AKIA") && s.chars().skip(4).all(|c| c.is_ascii_alphanumeric()) {
                    out.push(SecretMatch {
                        kind: SecretKind::AwsAccessKey,
                        start: i, end: i + 20,
                        matched_text: s.to_string(),
                        redacted: "[AWS-ACCESS-KEY-REDACTED]".into(),
                        severity: Severity::Critical,
                    });
                }
            }
        }
        out
    }

    fn detect_github_tokens(&self, text: &str) -> Vec<SecretMatch> {
        // ghp_, gho_, ghu_, ghs_, ghr_ + 36 chars
        let prefixes = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "github_pat_"];
        let mut out = Vec::new();
        for prefix in &prefixes {
            let mut start = 0;
            while let Some(idx) = text[start..].find(prefix) {
                let abs = start + idx;
                let end = abs + prefix.len() + 36;
                if end <= text.len() {
                    let slice = &text[abs..end];
                    if slice.chars().skip(prefix.len())
                        .all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        out.push(SecretMatch {
                            kind: SecretKind::GithubToken,
                            start: abs, end,
                            matched_text: slice.to_string(),
                            redacted: "[GITHUB-TOKEN-REDACTED]".into(),
                            severity: Severity::Critical,
                        });
                    }
                }
                start = abs + prefix.len();
            }
        }
        out
    }

    fn detect_openai_keys(&self, text: &str) -> Vec<SecretMatch> {
        // sk- followed by alphanumeric. Explicitly skip "sk-ant-" which is Anthropic.
        let mut out = Vec::new();
        let mut start = 0;
        while let Some(idx) = text[start..].find("sk-") {
            let abs = start + idx;
            // Skip if this is actually an Anthropic key prefix.
            if text[abs..].starts_with("sk-ant-") {
                start = abs + 3;
                continue;
            }
            // Count alphanumeric chars after prefix.
            let after: &str = &text[abs + 3..];
            let key_len: usize = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').count();
            if key_len >= 40 {
                let end = abs + 3 + key_len;
                let matched = text[abs..end].to_string();
                out.push(SecretMatch {
                    kind: SecretKind::OpenAiKey,
                    start: abs, end,
                    matched_text: matched,
                    redacted: "[OPENAI-KEY-REDACTED]".into(),
                    severity: Severity::Critical,
                });
            }
            start = abs + 3;
        }
        out
    }

    fn detect_anthropic_keys(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        let mut start = 0;
        while let Some(idx) = text[start..].find("sk-ant-") {
            let abs = start + idx;
            let after = &text[abs + 7..];
            let key_len: usize = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').count();
            if key_len >= 80 {
                let end = abs + 7 + key_len;
                out.push(SecretMatch {
                    kind: SecretKind::AnthropicKey,
                    start: abs, end,
                    matched_text: text[abs..end].to_string(),
                    redacted: "[ANTHROPIC-KEY-REDACTED]".into(),
                    severity: Severity::Critical,
                });
            }
            start = abs + 7;
        }
        out
    }

    fn detect_stripe_keys(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        for prefix in &["sk_live_", "sk_test_", "pk_live_", "pk_test_"] {
            let mut start = 0;
            while let Some(idx) = text[start..].find(prefix) {
                let abs = start + idx;
                let after = &text[abs + prefix.len()..];
                let key_len: usize = after.chars().take_while(|c| c.is_ascii_alphanumeric()).count();
                if key_len >= 24 {
                    let end = abs + prefix.len() + key_len;
                    out.push(SecretMatch {
                        kind: SecretKind::StripeKey,
                        start: abs, end,
                        matched_text: text[abs..end].to_string(),
                        redacted: "[STRIPE-KEY-REDACTED]".into(),
                        severity: Severity::Critical,
                    });
                }
                start = abs + prefix.len();
            }
        }
        out
    }

    fn detect_slack_tokens(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        for prefix in &["xoxb-", "xoxp-", "xoxa-", "xoxs-"] {
            let mut start = 0;
            while let Some(idx) = text[start..].find(prefix) {
                let abs = start + idx;
                let after = &text[abs + prefix.len()..];
                let key_len: usize = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').count();
                if key_len >= 40 {
                    let end = abs + prefix.len() + key_len;
                    out.push(SecretMatch {
                        kind: SecretKind::SlackToken,
                        start: abs, end,
                        matched_text: text[abs..end].to_string(),
                        redacted: "[SLACK-TOKEN-REDACTED]".into(),
                        severity: Severity::Critical,
                    });
                }
                start = abs + prefix.len();
            }
        }
        out
    }

    fn detect_jwt_tokens(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        // JWT: three base64url segments separated by dots, starting with "eyJ"
        let mut start = 0;
        while let Some(idx) = text[start..].find("eyJ") {
            let abs = start + idx;
            let tail = &text[abs..];
            // Count dots in the next 1000 chars
            let mut dot_count = 0;
            let mut end_offset = 0;
            for (i, c) in tail.char_indices().take(2000) {
                if c == '.' {
                    dot_count += 1;
                    if dot_count == 2 {
                        // Find end of third segment
                        let rest = &tail[i + 1..];
                        let third_len: usize = rest.chars()
                            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '=')
                            .count();
                        end_offset = i + 1 + third_len;
                        break;
                    }
                } else if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '=' {
                    break;
                }
            }
            if dot_count == 2 && end_offset > 50 {
                let end = abs + end_offset;
                out.push(SecretMatch {
                    kind: SecretKind::JwtToken,
                    start: abs, end,
                    matched_text: text[abs..end].to_string(),
                    redacted: "[JWT-REDACTED]".into(),
                    severity: Severity::High,
                });
            }
            start = abs + 3;
        }
        out
    }

    fn detect_private_keys(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        let markers = [
            ("-----BEGIN RSA PRIVATE KEY-----", SecretKind::RsaPrivateKey),
            ("-----BEGIN OPENSSH PRIVATE KEY-----", SecretKind::SshPrivateKey),
            ("-----BEGIN EC PRIVATE KEY-----", SecretKind::RsaPrivateKey),
            ("-----BEGIN PRIVATE KEY-----", SecretKind::RsaPrivateKey),
            ("-----BEGIN PGP PRIVATE KEY BLOCK-----", SecretKind::PgpPrivateKey),
        ];
        for (marker, kind) in &markers {
            if let Some(idx) = text.find(marker) {
                // Find the end marker
                let end_variants: Vec<String> = vec![
                    marker.replace("BEGIN", "END"),
                ];
                let mut end = idx + marker.len();
                for variant in &end_variants {
                    if let Some(e) = text[idx..].find(variant.as_str()) {
                        end = idx + e + variant.len();
                        break;
                    }
                }
                out.push(SecretMatch {
                    kind: kind.clone(),
                    start: idx, end: end.min(text.len()),
                    matched_text: text[idx..end.min(text.len())].to_string(),
                    redacted: "[PRIVATE-KEY-REDACTED]".into(),
                    severity: Severity::Critical,
                });
            }
        }
        out
    }

    fn detect_database_urls(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        let schemes = [
            "postgres://", "postgresql://", "mysql://", "mongodb://",
            "mongodb+srv://", "redis://", "rediss://",
        ];
        for scheme in &schemes {
            let mut start = 0;
            while let Some(idx) = text[start..].find(scheme) {
                let abs = start + idx;
                let rest = &text[abs..];
                // Stop at whitespace, quote, or newline
                let end_offset: usize = rest.chars()
                    .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'' && *c != '\n')
                    .count();
                if end_offset > scheme.len() + 5 {
                    let matched = &rest[..end_offset];
                    // Only flag if contains '@' (credentials present)
                    if matched.contains('@') {
                        out.push(SecretMatch {
                            kind: SecretKind::DatabaseUrl,
                            start: abs, end: abs + end_offset,
                            matched_text: matched.to_string(),
                            redacted: format!("[{}-URL-REDACTED]", scheme.to_uppercase().trim_end_matches("://")),
                            severity: Severity::Critical,
                        });
                    }
                }
                start = abs + scheme.len();
            }
        }
        out
    }

    fn detect_ssn(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i + 11 <= bytes.len() {
            let s = &text[i..i + 11];
            if s.chars().enumerate().all(|(j, c)| match j {
                3 | 6 => c == '-',
                _ => c.is_ascii_digit(),
            }) {
                // Prefix / suffix check: not a phone number
                let prev = if i > 0 { bytes[i - 1] as char } else { ' ' };
                let next = if i + 11 < bytes.len() { bytes[i + 11] as char } else { ' ' };
                if !prev.is_ascii_digit() && !next.is_ascii_digit() {
                    out.push(SecretMatch {
                        kind: SecretKind::Ssn,
                        start: i, end: i + 11,
                        matched_text: s.to_string(),
                        redacted: "XXX-XX-XXXX".into(),
                        severity: Severity::High,
                    });
                }
            }
            i += 1;
        }
        out
    }

    fn detect_credit_cards(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        // 16 digits with optional spaces/dashes
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // Try to match a CC-like pattern starting at i
            let mut digits = String::new();
            let mut end = i;
            for &b in &bytes[i..] {
                let c = b as char;
                if c.is_ascii_digit() {
                    digits.push(c);
                    end += 1;
                    if digits.len() == 16 { break; }
                } else if c == ' ' || c == '-' {
                    end += 1;
                    if digits.len() < 16 { continue; }
                    break;
                } else {
                    break;
                }
                if digits.len() >= 16 { break; }
            }
            if digits.len() == 16 && Self::luhn_valid(&digits) {
                out.push(SecretMatch {
                    kind: SecretKind::CreditCard,
                    start: i, end,
                    matched_text: text[i..end].to_string(),
                    redacted: format!("XXXX-XXXX-XXXX-{}", &digits[12..16]),
                    severity: Severity::High,
                });
                i = end;
            } else {
                i += 1;
            }
        }
        out
    }

    fn luhn_valid(digits: &str) -> bool {
        let d: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
        if d.len() != 16 { return false; }
        let mut sum = 0u32;
        for (i, &digit) in d.iter().rev().enumerate() {
            if i % 2 == 1 {
                let doubled = digit * 2;
                sum += if doubled > 9 { doubled - 9 } else { doubled };
            } else {
                sum += digit;
            }
        }
        sum % 10 == 0
    }

    fn detect_emails(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        let mut start = 0;
        while let Some(at_idx) = text[start..].find('@') {
            let abs_at = start + at_idx;
            // Walk backward for local part
            let local_start = text[..abs_at].rfind(|c: char|
                c.is_whitespace() || c == '<' || c == '"' || c == '\'' || c == '(' || c == ','
            ).map(|i| i + 1).unwrap_or(0);
            // Walk forward for domain
            let domain_end = text[abs_at..].find(|c: char|
                c.is_whitespace() || c == '>' || c == '"' || c == '\'' || c == ')' || c == ','
            ).map(|i| abs_at + i).unwrap_or(text.len());

            let candidate = &text[local_start..domain_end];
            if candidate.contains('@') && candidate.contains('.')
                && candidate.len() >= 5 && candidate.len() <= 254 {
                // Must have valid local and domain parts
                let parts: Vec<&str> = candidate.splitn(2, '@').collect();
                if parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.') {
                    out.push(SecretMatch {
                        kind: SecretKind::Email,
                        start: local_start,
                        end: domain_end,
                        matched_text: candidate.to_string(),
                        redacted: "[EMAIL-REDACTED]".into(),
                        severity: Severity::Low,
                    });
                }
            }
            start = abs_at + 1;
        }
        out
    }

    fn detect_phone_numbers(&self, text: &str) -> Vec<SecretMatch> {
        // US format: (xxx) xxx-xxxx or xxx-xxx-xxxx or xxx.xxx.xxxx
        let mut out = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i + 12 <= bytes.len() {
            let candidate = &text[i..i + 12];
            let stripped: String = candidate.chars().filter(|c| c.is_ascii_digit()).collect();
            if stripped.len() == 10 {
                // Check format looks like a phone
                let is_phone = candidate.chars().enumerate().all(|(_j, c)| {
                    c.is_ascii_digit() || matches!(c, '-' | '.' | ' ' | '(' | ')')
                });
                if is_phone && candidate.chars().filter(|c| !c.is_ascii_digit()).count() >= 2 {
                    out.push(SecretMatch {
                        kind: SecretKind::PhoneNumber,
                        start: i, end: i + 12,
                        matched_text: candidate.to_string(),
                        redacted: "[PHONE-REDACTED]".into(),
                        severity: Severity::Medium,
                    });
                    i += 12;
                    continue;
                }
            }
            i += 1;
        }
        out
    }

    fn detect_ip_addresses(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // Try IPv4: 4 octets separated by dots
            if bytes[i].is_ascii_digit() && (i == 0 || !bytes[i - 1].is_ascii_digit()) {
                let mut octets = Vec::new();
                let mut j = i;
                while j < bytes.len() && octets.len() < 4 {
                    let mut num = String::new();
                    while j < bytes.len() && bytes[j].is_ascii_digit() && num.len() < 3 {
                        num.push(bytes[j] as char);
                        j += 1;
                    }
                    if num.is_empty() { break; }
                    match num.parse::<u32>() {
                        Ok(n) if n <= 255 => octets.push(n),
                        _ => break,
                    }
                    if octets.len() < 4 && j < bytes.len() && bytes[j] == b'.' {
                        j += 1;
                    } else { break; }
                }
                if octets.len() == 4 {
                    let ip_str = format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3]);
                    // Is this a private/internal range?
                    let is_internal = (octets[0] == 10) ||
                        (octets[0] == 172 && (16..=31).contains(&octets[1])) ||
                        (octets[0] == 192 && octets[1] == 168) ||
                        (octets[0] == 127);
                    let severity = if is_internal {
                        Severity::Medium
                    } else {
                        Severity::Low
                    };
                    out.push(SecretMatch {
                        kind: SecretKind::IpAddress,
                        start: i, end: j,
                        matched_text: ip_str.clone(),
                        redacted: "[IP-REDACTED]".into(),
                        severity,
                    });
                    i = j;
                    continue;
                }
            }
            i += 1;
        }
        out
    }

    fn detect_high_entropy(&self, text: &str) -> Vec<SecretMatch> {
        let mut out = Vec::new();
        // Walk through tokens of length 32+
        for word in text.split_whitespace() {
            let clean: String = word.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '=')
                .collect();
            if clean.len() >= 32 && Self::shannon_entropy(&clean) >= 4.0 {
                // Find position in original text
                if let Some(start) = text.find(&clean) {
                    out.push(SecretMatch {
                        kind: SecretKind::HighEntropyString,
                        start, end: start + clean.len(),
                        matched_text: clean.clone(),
                        redacted: "[HIGH-ENTROPY-REDACTED]".into(),
                        severity: Severity::High,
                    });
                }
            }
        }
        out
    }

    fn shannon_entropy(s: &str) -> f64 {
        let mut freq = std::collections::HashMap::new();
        for c in s.chars() {
            *freq.entry(c).or_insert(0u32) += 1;
        }
        let len = s.len() as f64;
        if len == 0.0 { return 0.0; }
        let mut h = 0.0;
        for &count in freq.values() {
            let p = count as f64 / len;
            h -= p * p.log2();
        }
        h
    }

    fn deduplicate(&self, mut matches: Vec<SecretMatch>) -> Vec<SecretMatch> {
        // Sort by severity (higher first), then by start.
        // This ensures that when two matches overlap, we keep the
        // higher-severity one (e.g., AnthropicKey Critical wins over
        // HighEntropyString High).
        let sev_rank = |s: &Severity| match s {
            Severity::Critical => 4, Severity::High => 3,
            Severity::Medium => 2, Severity::Low => 1,
        };
        matches.sort_by(|a, b| {
            sev_rank(&b.severity).cmp(&sev_rank(&a.severity))
                .then_with(|| a.start.cmp(&b.start))
        });

        // Greedy: walk matches, accept if non-overlapping with any accepted match.
        let mut result: Vec<SecretMatch> = Vec::new();
        for m in matches {
            let overlaps = result.iter().any(|kept|
                m.start < kept.end && m.end > kept.start
            );
            if !overlaps {
                result.push(m);
            }
        }
        // Re-sort by start for stable output.
        result.sort_by_key(|m| m.start);
        result
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_aws_access_key() {
        let scanner = SecretScanner::new();
        let text = "My AWS key is AKIAIOSFODNN7EXAMPLE and here is text";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::AwsAccessKey));
    }

    #[test]
    fn test_detect_github_token() {
        let scanner = SecretScanner::new();
        // Construct at runtime so the literal doesn't trip GitHub's secret scanner.
        let text = format!("token: {}{}", "ghp_", "A".repeat(36));
        let matches = scanner.scan(&text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::GithubToken));
    }

    #[test]
    fn test_detect_openai_key() {
        let scanner = SecretScanner::new();
        // Construct at runtime to avoid literal-match by GitHub's secret scanner.
        let text = format!("OPENAI_KEY={}{}", "sk-", "X".repeat(48));
        let matches = scanner.scan(&text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::OpenAiKey),
            "Should detect OpenAI key, got: {:?}", matches);
    }

    #[test]
    fn test_detect_anthropic_key() {
        let scanner = SecretScanner::new();
        let text = format!("key: sk-ant-api01-{}", "a".repeat(95));
        let matches = scanner.scan(&text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::AnthropicKey));
    }

    #[test]
    fn test_detect_stripe_key() {
        let scanner = SecretScanner::new();
        // Construct test string at runtime to avoid triggering GitHub's
        // secret scanner push protection on a literal that looks real.
        let prefix = "sk_";
        let middle = "live_";
        let body: String = "X".repeat(24);
        let text = format!("stripe: {}{}{}", prefix, middle, body);
        let matches = scanner.scan(&text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::StripeKey));
    }

    #[test]
    fn test_detect_ssn() {
        let scanner = SecretScanner::new();
        let text = "SSN: 123-45-6789 in the record";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::Ssn));
    }

    #[test]
    fn test_detect_credit_card_valid_luhn() {
        let scanner = SecretScanner::new();
        // 4532015112830366 is Luhn-valid
        let text = "Card: 4532015112830366 expires 12/28";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::CreditCard),
            "Should detect valid CC: {:?}", matches);
    }

    #[test]
    fn test_invalid_luhn_not_detected() {
        let scanner = SecretScanner::new();
        let text = "Card: 1234567890123456 invalid"; // Invalid Luhn
        let matches = scanner.scan(text);
        assert!(!matches.iter().any(|m| m.kind == SecretKind::CreditCard));
    }

    #[test]
    fn test_detect_email() {
        let scanner = SecretScanner::new();
        let text = "Contact me at user@example.com for details";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::Email));
    }

    #[test]
    fn test_detect_phone() {
        let scanner = SecretScanner::new();
        let text = "Call 555-123-4567 or 555.987.6543";
        let matches = scanner.scan(text);
        let phone_matches: Vec<_> = matches.iter()
            .filter(|m| m.kind == SecretKind::PhoneNumber).collect();
        assert!(!phone_matches.is_empty(), "Should detect phone, got: {:?}", matches);
    }

    #[test]
    fn test_detect_ip_external() {
        let scanner = SecretScanner::new();
        let text = "Server: 8.8.8.8 port 443";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::IpAddress));
    }

    #[test]
    fn test_detect_ip_internal_medium_severity() {
        let scanner = SecretScanner::new();
        let text = "Internal: 10.0.0.1 dev";
        let matches = scanner.scan(text);
        let ip = matches.iter().find(|m| m.kind == SecretKind::IpAddress).unwrap();
        assert_eq!(ip.severity, Severity::Medium, "Internal IP should be Medium");
    }

    #[test]
    fn test_detect_database_url() {
        let scanner = SecretScanner::new();
        let text = "DATABASE_URL=postgres://user:secret@db.example.com:5432/mydb";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::DatabaseUrl));
    }

    #[test]
    fn test_detect_private_key_pem() {
        let scanner = SecretScanner::new();
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::RsaPrivateKey));
        let pk = matches.iter().find(|m| m.kind == SecretKind::RsaPrivateKey).unwrap();
        assert_eq!(pk.severity, Severity::Critical);
    }

    #[test]
    fn test_detect_jwt() {
        let scanner = SecretScanner::new();
        let text = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4ifQ.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m| m.kind == SecretKind::JwtToken));
    }

    #[test]
    fn test_redact_replaces_secrets() {
        let scanner = SecretScanner::new();
        let text = "My key is AKIAIOSFODNN7EXAMPLE and my email is user@example.com";
        let redacted = scanner.redact(text);
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!redacted.contains("user@example.com"));
        assert!(redacted.contains("REDACTED"));
    }

    #[test]
    fn test_allowlist() {
        let scanner = SecretScanner::with_allowlist(vec![
            "user@example.com".into(),
        ]);
        let text = "Contact: user@example.com";
        let matches = scanner.scan(text);
        assert!(!matches.iter().any(|m| m.kind == SecretKind::Email));
    }

    #[test]
    fn test_clean_text_no_matches() {
        let scanner = SecretScanner::new();
        let text = "Hello world. This is a normal sentence with no secrets.";
        assert!(!scanner.contains_secrets(text));
    }

    #[test]
    fn test_highest_severity() {
        let scanner = SecretScanner::new();
        let text = "email: user@example.com, AWS: AKIAIOSFODNN7EXAMPLE, phone 555-123-4567";
        let severity = scanner.highest_severity(text);
        assert_eq!(severity, Some(Severity::Critical),
            "AWS key is Critical, should be highest");
    }

    #[test]
    fn test_high_entropy_detection() {
        let scanner = SecretScanner::new();
        // Long random-looking string
        let text = "token abcdefghijk1234567890ABCDEFGHIJK1234567890ABCD";
        let matches = scanner.scan(text);
        assert!(matches.iter().any(|m|
            m.kind == SecretKind::HighEntropyString ||
            m.kind == SecretKind::OpenAiKey
        ), "Should detect high-entropy string");
    }

    #[test]
    fn test_shannon_entropy_low() {
        // Repetitive string: low entropy
        assert!(SecretScanner::shannon_entropy("aaaaaaaaaaaa") < 1.0);
    }

    #[test]
    fn test_shannon_entropy_high() {
        // Varied alphanumeric: higher entropy
        assert!(SecretScanner::shannon_entropy("abcdefghijklmnop") > 3.0);
    }

    #[test]
    fn test_luhn_known_valid() {
        assert!(SecretScanner::luhn_valid("4532015112830366")); // Valid Visa
        assert!(!SecretScanner::luhn_valid("1234567890123456")); // Invalid
    }

    #[test]
    fn test_deduplication() {
        let scanner = SecretScanner::new();
        // Text that might trigger multiple overlapping detectors
        let text = "aws: AKIAIOSFODNN7EXAMPLE";
        let matches = scanner.scan(text);
        // Should not return multiple matches for the same span
        let starts: Vec<usize> = matches.iter().map(|m| m.start).collect();
        let unique: std::collections::HashSet<_> = starts.iter().collect();
        assert_eq!(starts.len(), unique.len(), "Matches should not overlap");
    }
}
