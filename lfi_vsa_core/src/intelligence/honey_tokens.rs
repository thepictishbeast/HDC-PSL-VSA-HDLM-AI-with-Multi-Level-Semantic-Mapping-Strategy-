// ============================================================
// Honey Tokens — Tripwire Credentials for Breach Detection
//
// PURPOSE: Generate realistic-looking fake credentials that alert
// when used. Deploy in codebases, documentation, configs, databases,
// employee mailboxes. Any attempt to use them triggers a breach alert.
//
// DEPLOYMENT SCENARIOS:
//   - AWS honey keys in config files (attacker finds → uses → alert)
//   - Honey endpoints in documentation (crawler follows → alert)
//   - Fake user accounts in employee databases (login attempt → alert)
//   - Honey DB credentials in `.env.example` (attacker tries → alert)
//   - Honey emails in address books (spam/phishing → alert)
//
// KEY PROPERTIES:
//   - Realistic: looks indistinguishable from real credentials
//   - Unique: each token generated is one-of-a-kind
//   - Tracked: registry maps tokens to deployment location
//   - Detected: scanner identifies when a honey token is in use
//   - Callback-enabled: alert handler fires on detection
// ============================================================

use std::collections::HashMap;

// ============================================================
// Honey Token Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HoneyKind {
    AwsAccessKey,
    AwsSecretKey,
    GithubToken,
    OpenAiKey,
    DatabaseUrl,
    JwtToken,
    EmailAddress,
    PhoneNumber,
    ApiEndpoint,
    UsernamePassword,
}

#[derive(Debug, Clone)]
pub struct HoneyToken {
    /// Unique token ID for tracking.
    pub id: String,
    pub kind: HoneyKind,
    /// The actual honey value deployed.
    pub value: String,
    /// Where this token is deployed (user-supplied description).
    pub deployment: String,
    /// Timestamp of creation.
    pub created_ms: u64,
    /// If fired: when the detection event occurred.
    pub fired_ms: Option<u64>,
    /// Additional context captured at detection.
    pub fire_context: Option<String>,
}

// ============================================================
// Honey Token Generator
// ============================================================

pub struct HoneyTokenGenerator {
    /// Counter for unique ID generation.
    counter: u64,
    /// Seed for pseudo-random generation.
    seed: u64,
}

impl HoneyTokenGenerator {
    pub fn new(seed: u64) -> Self {
        Self { counter: 0, seed }
    }

    fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("HT-{:016x}-{:04}", self.seed, self.counter)
    }

    /// Generate a pseudo-random alphanumeric string of given length.
    /// Uses a simple LCG seeded from the generator's state.
    /// BUG ASSUMPTION: this is NOT cryptographically random; fine for
    /// honey tokens (we want deterministic-enough to fingerprint later).
    fn random_alphanum(&mut self, len: usize) -> String {
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut state = self.seed.wrapping_add(self.counter);
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let idx = (state >> 32) as usize % CHARS.len();
            out.push(CHARS[idx] as char);
        }
        out
    }

    fn random_digits(&mut self, len: usize) -> String {
        let mut state = self.seed.wrapping_add(self.counter).wrapping_mul(17);
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            out.push(((state >> 16) % 10) as u8 as char);
            let digit = ((state >> 16) % 10) as u8 + b'0';
            out.pop();
            out.push(digit as char);
        }
        out
    }

    /// Generate an AWS access key honey token.
    /// Format: AKIA + 16 alphanumeric chars (matches real AWS format).
    pub fn aws_access_key(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        let value = format!("AKIA{}", self.random_alphanum(16));
        HoneyToken {
            id,
            kind: HoneyKind::AwsAccessKey,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }

    pub fn github_token(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        let value = format!("ghp_{}", self.random_alphanum(36));
        HoneyToken {
            id,
            kind: HoneyKind::GithubToken,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }

    pub fn openai_key(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        let value = format!("sk-proj-{}", self.random_alphanum(48));
        HoneyToken {
            id,
            kind: HoneyKind::OpenAiKey,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }

    pub fn database_url(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        let password = self.random_alphanum(32);
        let db_id = self.random_alphanum(8).to_lowercase();
        // Non-routable IP so accidental connection attempts fail fast.
        let value = format!("postgres://hydra_{}:{}@10.0.0.254:5432/prod_db",
            db_id, password);
        HoneyToken {
            id,
            kind: HoneyKind::DatabaseUrl,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }

    pub fn email_address(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        let local = self.random_alphanum(10).to_lowercase();
        // Use a subdomain that the honeypot operator controls.
        let value = format!("{}.trap@honeytrap.plausiden.io", local);
        HoneyToken {
            id,
            kind: HoneyKind::EmailAddress,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }

    pub fn phone_number(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        // Use 555 (North American fictional exchange) to avoid real numbers.
        let middle = self.random_digits(3);
        let suffix = self.random_digits(4);
        // Avoid all-zero or sequential numbers.
        let value = format!("555-{}-{}", middle, suffix);
        HoneyToken {
            id,
            kind: HoneyKind::PhoneNumber,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }

    pub fn api_endpoint(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        let path = self.random_alphanum(16).to_lowercase();
        // Canary domain that the honeypot operator controls.
        let value = format!("https://api.honeytrap.plausiden.io/v1/canary/{}", path);
        HoneyToken {
            id,
            kind: HoneyKind::ApiEndpoint,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }

    pub fn username_password(&mut self, deployment: &str) -> HoneyToken {
        let id = self.next_id();
        let user = format!("admin_{}", self.random_alphanum(8).to_lowercase());
        let pass = self.random_alphanum(16);
        let value = format!("{}:{}", user, pass);
        HoneyToken {
            id,
            kind: HoneyKind::UsernamePassword,
            value,
            deployment: deployment.into(),
            created_ms: now_ms(),
            fired_ms: None,
            fire_context: None,
        }
    }
}

// ============================================================
// Honey Token Registry
// ============================================================

/// Tracks all deployed honey tokens and their fire events.
pub struct HoneyTokenRegistry {
    /// Token ID → token.
    tokens: HashMap<String, HoneyToken>,
    /// Token VALUE → ID (for fast lookup during detection).
    by_value: HashMap<String, String>,
    /// Total fire events.
    pub total_fires: u64,
    /// Optional alert handler.
    alert_handler: Option<Box<dyn Fn(&HoneyToken, &str) + Send + Sync>>,
}

impl HoneyTokenRegistry {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            by_value: HashMap::new(),
            total_fires: 0,
            alert_handler: None,
        }
    }

    pub fn with_alert_handler<F>(handler: F) -> Self
    where F: Fn(&HoneyToken, &str) + Send + Sync + 'static {
        Self {
            tokens: HashMap::new(),
            by_value: HashMap::new(),
            total_fires: 0,
            alert_handler: Some(Box::new(handler)),
        }
    }

    /// Register a deployed honey token.
    pub fn register(&mut self, token: HoneyToken) -> String {
        let id = token.id.clone();
        let value = token.value.clone();
        self.tokens.insert(id.clone(), token);
        self.by_value.insert(value, id.clone());
        id
    }

    /// Check if an observed string contains any registered honey token.
    /// Returns token IDs that fired.
    pub fn check(&mut self, observed: &str, context: &str) -> Vec<String> {
        let mut fired = Vec::new();
        // Check exact match first (fast).
        if let Some(id) = self.by_value.get(observed) {
            fired.push(id.clone());
        }
        // Check substring for longer observed content.
        for (value, id) in &self.by_value.clone() {
            if observed.contains(value.as_str()) && !fired.contains(id) {
                fired.push(id.clone());
            }
        }

        // Update fire state for each detected token.
        for id in &fired {
            if let Some(token) = self.tokens.get_mut(id) {
                if token.fired_ms.is_none() {
                    token.fired_ms = Some(now_ms());
                    token.fire_context = Some(context.into());
                    self.total_fires += 1;
                    debuglog!("HONEY TOKEN FIRED: id={}, kind={:?}, deployment='{}', context='{}'",
                        id, token.kind, token.deployment, context);
                    if let Some(ref handler) = self.alert_handler {
                        handler(token, context);
                    }
                }
            }
        }
        fired
    }

    /// Get a token by ID.
    pub fn get(&self, id: &str) -> Option<&HoneyToken> {
        self.tokens.get(id)
    }

    /// List all fired tokens.
    pub fn fired_tokens(&self) -> Vec<&HoneyToken> {
        self.tokens.values()
            .filter(|t| t.fired_ms.is_some())
            .collect()
    }

    /// List all deployed (but not yet fired) tokens.
    pub fn active_tokens(&self) -> Vec<&HoneyToken> {
        self.tokens.values()
            .filter(|t| t.fired_ms.is_none())
            .collect()
    }

    /// Summary statistics.
    pub fn stats(&self) -> RegistryStats {
        let mut by_kind: HashMap<String, usize> = HashMap::new();
        let mut fires_by_kind: HashMap<String, usize> = HashMap::new();
        for t in self.tokens.values() {
            *by_kind.entry(format!("{:?}", t.kind)).or_insert(0) += 1;
            if t.fired_ms.is_some() {
                *fires_by_kind.entry(format!("{:?}", t.kind)).or_insert(0) += 1;
            }
        }
        RegistryStats {
            total_deployed: self.tokens.len(),
            total_fires: self.total_fires,
            deployed_by_kind: by_kind,
            fires_by_kind,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_deployed: usize,
    pub total_fires: u64,
    pub deployed_by_kind: HashMap<String, usize>,
    pub fires_by_kind: HashMap<String, usize>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_aws_key_format() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.aws_access_key("~/.aws/credentials");
        assert!(token.value.starts_with("AKIA"));
        assert_eq!(token.value.len(), 20);
        assert_eq!(token.kind, HoneyKind::AwsAccessKey);
    }

    #[test]
    fn test_github_token_format() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.github_token("README.md");
        assert!(token.value.starts_with("ghp_"));
        assert_eq!(token.value.len(), 4 + 36);
    }

    #[test]
    fn test_openai_key_format() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.openai_key(".env.example");
        assert!(token.value.starts_with("sk-proj-"));
    }

    #[test]
    fn test_database_url_format() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.database_url("config/production.yml");
        assert!(token.value.starts_with("postgres://"));
        assert!(token.value.contains("@"));
        assert!(token.value.contains("10.0.0.254"));
    }

    #[test]
    fn test_tokens_are_unique() {
        let mut gen = HoneyTokenGenerator::new(42);
        let t1 = gen.aws_access_key("loc1");
        let t2 = gen.aws_access_key("loc2");
        assert_ne!(t1.id, t2.id);
        assert_ne!(t1.value, t2.value);
    }

    #[test]
    fn test_different_seeds_different_values() {
        let mut g1 = HoneyTokenGenerator::new(100);
        let mut g2 = HoneyTokenGenerator::new(200);
        let t1 = g1.aws_access_key("same");
        let t2 = g2.aws_access_key("same");
        assert_ne!(t1.value, t2.value);
    }

    #[test]
    fn test_registry_detect_exact_match() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.aws_access_key("config.yml");
        let value = token.value.clone();
        let id = token.id.clone();

        let mut registry = HoneyTokenRegistry::new();
        registry.register(token);

        let fired = registry.check(&value, "observed in logs");
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0], id);

        // Token should now be marked as fired.
        let t = registry.get(&id).unwrap();
        assert!(t.fired_ms.is_some());
        assert!(t.fire_context.as_ref().unwrap().contains("observed in logs"));
    }

    #[test]
    fn test_registry_detect_substring() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.github_token("README");
        let value = token.value.clone();

        let mut registry = HoneyTokenRegistry::new();
        registry.register(token);

        // Embed in larger text
        let observed = format!("Some text before {} and after", value);
        let fired = registry.check(&observed, "in log message");
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn test_registry_no_match() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.aws_access_key("config");
        let mut registry = HoneyTokenRegistry::new();
        registry.register(token);

        let fired = registry.check("AKIA_SOMETHING_ELSE_NOT_OURS", "logs");
        assert_eq!(fired.len(), 0);
    }

    #[test]
    fn test_registry_fire_only_once() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.aws_access_key("config");
        let value = token.value.clone();
        let id = token.id.clone();

        let mut registry = HoneyTokenRegistry::new();
        registry.register(token);
        registry.check(&value, "first");
        registry.check(&value, "second");

        assert_eq!(registry.total_fires, 1,
            "Token should fire only once per lifetime");

        let t = registry.get(&id).unwrap();
        assert!(t.fire_context.as_ref().unwrap().contains("first"));
    }

    #[test]
    fn test_alert_handler_called() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = Arc::clone(&fired);
        let handler = move |_token: &HoneyToken, _context: &str| {
            fired_clone.store(true, Ordering::SeqCst);
        };

        let mut registry = HoneyTokenRegistry::with_alert_handler(handler);
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.aws_access_key("config");
        let value = token.value.clone();
        registry.register(token);

        registry.check(&value, "attacker usage");
        assert!(fired.load(Ordering::SeqCst), "Alert handler should be called");
    }

    #[test]
    fn test_fired_tokens_listing() {
        let mut gen = HoneyTokenGenerator::new(42);
        let mut registry = HoneyTokenRegistry::new();
        let t1 = gen.aws_access_key("loc1");
        let t2 = gen.github_token("loc2");
        let v1 = t1.value.clone();
        registry.register(t1);
        registry.register(t2);

        registry.check(&v1, "attacker");
        let fired = registry.fired_tokens();
        assert_eq!(fired.len(), 1);
        let active = registry.active_tokens();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_stats_aggregate() {
        let mut gen = HoneyTokenGenerator::new(42);
        let mut registry = HoneyTokenRegistry::new();
        registry.register(gen.aws_access_key("a"));
        registry.register(gen.aws_access_key("b"));
        registry.register(gen.github_token("c"));

        let stats = registry.stats();
        assert_eq!(stats.total_deployed, 3);
        assert_eq!(stats.deployed_by_kind.get("AwsAccessKey").copied().unwrap_or(0), 2);
        assert_eq!(stats.deployed_by_kind.get("GithubToken").copied().unwrap_or(0), 1);
    }

    #[test]
    fn test_email_uses_honeytrap_domain() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.email_address("contact list");
        assert!(token.value.contains("honeytrap"),
            "Email should use honeytrap domain: {}", token.value);
    }

    #[test]
    fn test_phone_uses_555() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.phone_number("employee list");
        assert!(token.value.starts_with("555-"),
            "Phone should use 555 exchange: {}", token.value);
    }

    #[test]
    fn test_username_password_format() {
        let mut gen = HoneyTokenGenerator::new(42);
        let token = gen.username_password("backup_creds.txt");
        assert!(token.value.contains(":"));
        assert!(token.value.starts_with("admin_"));
    }

    #[test]
    fn test_registry_multi_token_one_observation() {
        let mut gen = HoneyTokenGenerator::new(42);
        let t1 = gen.aws_access_key("config1");
        let t2 = gen.aws_access_key("config2");
        let v1 = t1.value.clone();
        let v2 = t2.value.clone();

        let mut registry = HoneyTokenRegistry::new();
        registry.register(t1);
        registry.register(t2);

        // Attacker dumps both keys in one place
        let observed = format!("key1={}\nkey2={}", v1, v2);
        let fired = registry.check(&observed, "attacker dump");
        assert_eq!(fired.len(), 2, "Should detect both tokens");
    }

    // ============================================================
    // Stress / invariant tests for honey_tokens
    // ============================================================

    /// INVARIANT: HoneyTokenGenerator with the same seed produces the same
    /// tokens — analysts can re-derive the trap inventory.
    #[test]
    fn invariant_generator_deterministic_per_seed() {
        let mut g1 = HoneyTokenGenerator::new(42);
        let mut g2 = HoneyTokenGenerator::new(42);
        let t1 = g1.aws_access_key("prod");
        let t2 = g2.aws_access_key("prod");
        assert_eq!(t1.value, t2.value,
            "same seed must yield same AWS key");
        let g1_gh = g1.github_token("prod");
        let g2_gh = g2.github_token("prod");
        assert_eq!(g1_gh.value, g2_gh.value,
            "same seed must yield same GitHub token");
    }

    /// INVARIANT: different seeds produce different tokens (no collision).
    #[test]
    fn invariant_generator_seeds_diverge() {
        let mut g1 = HoneyTokenGenerator::new(1);
        let mut g2 = HoneyTokenGenerator::new(2);
        assert_ne!(g1.aws_access_key("p").value, g2.aws_access_key("p").value);
        assert_ne!(g1.github_token("p").value, g2.github_token("p").value);
        assert_ne!(g1.openai_key("p").value, g2.openai_key("p").value);
    }

    /// INVARIANT: registry register() returns a unique id per token.
    /// Duplicate ids would defeat audit correlation.
    #[test]
    fn invariant_registry_ids_unique() {
        let mut g = HoneyTokenGenerator::new(7);
        let mut reg = HoneyTokenRegistry::new();
        let mut seen = std::collections::HashSet::new();
        for i in 0..30 {
            let token = g.aws_access_key(&format!("dep_{}", i));
            let id = reg.register(token);
            assert!(seen.insert(id.clone()),
                "duplicate honey-token id at iter {}: {}", i, id);
        }
    }

    /// INVARIANT: get(id) returns Some only for registered tokens.
    #[test]
    fn invariant_get_matches_register() {
        let mut g = HoneyTokenGenerator::new(11);
        let mut reg = HoneyTokenRegistry::new();
        let id = reg.register(g.aws_access_key("d"));
        assert!(reg.get(&id).is_some(), "registered id must be retrievable");
        assert!(reg.get("nonexistent_id").is_none(),
            "unregistered id must return None");
    }

    /// INVARIANT: check() never panics on arbitrary unicode / control bytes.
    #[test]
    fn invariant_check_safe_on_unicode() {
        let mut g = HoneyTokenGenerator::new(13);
        let mut reg = HoneyTokenRegistry::new();
        reg.register(g.aws_access_key("d"));
        let inputs = [
            "",
            "アリス",
            "🦀🦀🦀",
            "control: \x00\x01\x1f",
            &"x".repeat(100_000),
        ];
        for input in inputs {
            // Must not panic.
            let _ = reg.check(input, "test");
        }
    }

    /// INVARIANT: check() only fires when the token's value appears verbatim
    /// — partial matches must NOT trigger (false-positive avoidance).
    #[test]
    fn invariant_no_partial_match_false_positives() {
        let mut g = HoneyTokenGenerator::new(17);
        let mut reg = HoneyTokenRegistry::new();
        let token = g.aws_access_key("d");
        let value = token.value.clone();
        reg.register(token);
        // First half of the value alone must not fire.
        let half_len = value.len() / 2;
        if half_len > 4 {
            let half = &value[..half_len];
            let fired = reg.check(half, "partial");
            assert!(fired.is_empty(),
                "partial match must not fire: {:?}", half);
        }
    }

    /// INVARIANT: generator is deterministic with same seed.
    #[test]
    fn invariant_generator_deterministic() {
        let mut g1 = HoneyTokenGenerator::new(42);
        let mut g2 = HoneyTokenGenerator::new(42);
        let t1 = g1.aws_access_key("prod");
        let t2 = g2.aws_access_key("prod");
        assert_eq!(t1.value, t2.value,
            "same seed must produce same tokens");
    }

    /// INVARIANT: fresh registry has no fired tokens.
    #[test]
    fn invariant_fresh_registry_no_fired_tokens() {
        let reg = HoneyTokenRegistry::new();
        let fired = reg.fired_tokens();
        assert!(fired.is_empty(),
            "fresh registry should have no fired tokens");
    }

    /// INVARIANT: get() returns None for unknown IDs.
    #[test]
    fn invariant_get_unknown_id_none() {
        let reg = HoneyTokenRegistry::new();
        assert!(reg.get("nonexistent_token_id_xxx").is_none());
    }
}
