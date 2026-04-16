// ============================================================
// Rate Limiter — Token-Bucket + Sliding-Window Algorithms
//
// PURPOSE: Protect LFI endpoints from abuse (DoS, brute force,
// model extraction via query volume) with industry-standard
// rate-limiting algorithms.
//
// TWO ALGORITHMS:
//   1. TOKEN BUCKET — allows bursts up to bucket capacity, refills at
//      constant rate. Good for typical API usage with occasional spikes.
//   2. SLIDING WINDOW — strict request count over a time window. Good
//      for anti-abuse (exactly N per minute, no bursting).
//
// SCOPING:
//   - Per-identity (user ID, IP, API key hash)
//   - Per-endpoint (different limits for /detect vs /scan)
//   - Combined (identity + endpoint)
//
// INTEGRATION:
//   Call check() before handling a request. Denied requests get a
//   RateLimitResult with retry_after_ms so the caller can send a 429
//   with Retry-After header.
// ============================================================

use std::collections::HashMap;
use std::sync::Mutex;

// ============================================================
// Token Bucket
// ============================================================

#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current token count (can be fractional).
    tokens: f64,
    /// Maximum capacity.
    capacity: f64,
    /// Tokens refilled per second.
    refill_rate_per_sec: f64,
    /// Last refill time (ms since epoch).
    last_refill_ms: u64,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate_per_sec: f64, now_ms: u64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate_per_sec,
            last_refill_ms: now_ms,
        }
    }

    fn refill(&mut self, now_ms: u64) {
        if now_ms <= self.last_refill_ms { return; }
        let elapsed_sec = (now_ms - self.last_refill_ms) as f64 / 1000.0;
        self.tokens = (self.tokens + elapsed_sec * self.refill_rate_per_sec)
            .min(self.capacity);
        self.last_refill_ms = now_ms;
    }

    /// Try to consume N tokens. Returns true if allowed, false otherwise.
    fn try_consume(&mut self, cost: f64, now_ms: u64) -> bool {
        self.refill(now_ms);
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    /// Milliseconds until the bucket has at least `cost` tokens.
    fn retry_after_ms(&self, cost: f64) -> u64 {
        if self.tokens >= cost { return 0; }
        let shortfall = cost - self.tokens;
        let seconds_needed = shortfall / self.refill_rate_per_sec;
        (seconds_needed * 1000.0).ceil() as u64
    }
}

// ============================================================
// Sliding Window
// ============================================================

#[derive(Debug, Clone)]
struct SlidingWindow {
    /// Request timestamps (ms) within the window.
    timestamps: Vec<u64>,
    /// Maximum requests allowed in window.
    max_requests: usize,
    /// Window size in milliseconds.
    window_ms: u64,
}

impl SlidingWindow {
    fn new(max_requests: usize, window_ms: u64) -> Self {
        Self {
            timestamps: Vec::new(),
            max_requests,
            window_ms,
        }
    }

    fn try_record(&mut self, now_ms: u64) -> bool {
        // Drop timestamps outside the window.
        let cutoff = now_ms.saturating_sub(self.window_ms);
        self.timestamps.retain(|&t| t >= cutoff);

        if self.timestamps.len() < self.max_requests {
            self.timestamps.push(now_ms);
            true
        } else {
            false
        }
    }

    fn retry_after_ms(&self, now_ms: u64) -> u64 {
        if let Some(&oldest) = self.timestamps.first() {
            let expires_at = oldest + self.window_ms;
            if expires_at > now_ms {
                return expires_at - now_ms;
            }
        }
        0
    }
}

// ============================================================
// Rate Limit Policy
// ============================================================

#[derive(Debug, Clone)]
pub enum RateLimitPolicy {
    /// Token bucket: (capacity, refill_per_sec).
    TokenBucket {
        capacity: f64,
        refill_per_sec: f64,
    },
    /// Sliding window: (max_requests, window_ms).
    SlidingWindow {
        max_requests: usize,
        window_ms: u64,
    },
    /// No rate limit (disabled).
    Unlimited,
}

// ============================================================
// Rate Limit Result
// ============================================================

#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    /// Suggested wait time before retry (ms).
    pub retry_after_ms: u64,
    /// Remaining quota (for HTTP X-RateLimit-Remaining header).
    pub remaining: Option<u64>,
    /// The scope that triggered (identity, endpoint, or combined).
    pub scope: String,
}

// ============================================================
// Rate Limiter
// ============================================================

pub struct RateLimiter {
    /// Default policy when no specific scope matches.
    default_policy: RateLimitPolicy,
    /// Per-scope policies (e.g., "endpoint:/detect" → policy).
    scope_policies: HashMap<String, RateLimitPolicy>,
    /// Active token buckets per scope-key.
    buckets: Mutex<HashMap<String, TokenBucket>>,
    /// Active sliding windows per scope-key.
    windows: Mutex<HashMap<String, SlidingWindow>>,
}

impl RateLimiter {
    pub fn new(default_policy: RateLimitPolicy) -> Self {
        Self {
            default_policy,
            scope_policies: HashMap::new(),
            buckets: Mutex::new(HashMap::new()),
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Register a specific policy for a scope (e.g., "endpoint:/detect").
    pub fn set_policy(&mut self, scope: &str, policy: RateLimitPolicy) {
        self.scope_policies.insert(scope.into(), policy);
    }

    /// Check whether a request is allowed.
    /// `scope` is typically a combined key like "user:alice|endpoint:/detect".
    pub fn check(&self, scope: &str, now_ms: u64) -> RateLimitResult {
        self.check_with_cost(scope, 1.0, now_ms)
    }

    /// Check with a specific cost (allow weighting expensive operations).
    pub fn check_with_cost(&self, scope: &str, cost: f64, now_ms: u64) -> RateLimitResult {
        // Find the matching policy — look for scope prefix matches first,
        // fall back to default.
        let policy = self.scope_policies.iter()
            .find(|(k, _)| scope.contains(k.as_str()))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| self.default_policy.clone());

        match policy {
            RateLimitPolicy::Unlimited => RateLimitResult {
                allowed: true,
                retry_after_ms: 0,
                remaining: None,
                scope: scope.into(),
            },
            RateLimitPolicy::TokenBucket { capacity, refill_per_sec } => {
                let mut buckets = match self.buckets.lock() {
                    Ok(b) => b,
                    Err(_) => return RateLimitResult {
                        allowed: false, retry_after_ms: 1000,
                        remaining: None, scope: scope.into(),
                    },
                };
                let bucket = buckets.entry(scope.to_string())
                    .or_insert_with(|| TokenBucket::new(capacity, refill_per_sec, now_ms));
                let allowed = bucket.try_consume(cost, now_ms);
                RateLimitResult {
                    allowed,
                    retry_after_ms: if allowed { 0 } else { bucket.retry_after_ms(cost) },
                    remaining: Some(bucket.tokens as u64),
                    scope: scope.into(),
                }
            }
            RateLimitPolicy::SlidingWindow { max_requests, window_ms } => {
                let mut windows = match self.windows.lock() {
                    Ok(w) => w,
                    Err(_) => return RateLimitResult {
                        allowed: false, retry_after_ms: 1000,
                        remaining: None, scope: scope.into(),
                    },
                };
                let window = windows.entry(scope.to_string())
                    .or_insert_with(|| SlidingWindow::new(max_requests, window_ms));
                let allowed = window.try_record(now_ms);
                let remaining = max_requests.saturating_sub(window.timestamps.len()) as u64;
                RateLimitResult {
                    allowed,
                    retry_after_ms: if allowed { 0 } else { window.retry_after_ms(now_ms) },
                    remaining: Some(remaining),
                    scope: scope.into(),
                }
            }
        }
    }

    /// Reset state for a specific scope.
    pub fn reset(&self, scope: &str) {
        if let Ok(mut b) = self.buckets.lock() { b.remove(scope); }
        if let Ok(mut w) = self.windows.lock() { w.remove(scope); }
    }

    /// Reset all state.
    pub fn reset_all(&self) {
        if let Ok(mut b) = self.buckets.lock() { b.clear(); }
        if let Ok(mut w) = self.windows.lock() { w.clear(); }
    }

    /// Current number of tracked scopes.
    pub fn tracked_count(&self) -> usize {
        let b_count = self.buckets.lock().map(|b| b.len()).unwrap_or(0);
        let w_count = self.windows.lock().map(|w| w.len()).unwrap_or(0);
        b_count + w_count
    }
}

// ============================================================
// Tiered Rate Limiter (per user tier: Free, Pro, Team, Enterprise)
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UserTier {
    Free,
    Pro,
    Team,
    Enterprise,
}

pub struct TieredRateLimiter {
    /// One limiter per tier, each with that tier's policy configured as default.
    tier_limiters: HashMap<UserTier, RateLimiter>,
}

impl TieredRateLimiter {
    pub fn new() -> Self {
        let mut tier_limiters = HashMap::new();

        tier_limiters.insert(UserTier::Free, RateLimiter::new(
            RateLimitPolicy::SlidingWindow {
                max_requests: 100,
                window_ms: 60_000, // 100/min
            },
        ));
        tier_limiters.insert(UserTier::Pro, RateLimiter::new(
            RateLimitPolicy::TokenBucket {
                capacity: 500.0,
                refill_per_sec: 10.0, // ~36k/hour bursty
            },
        ));
        tier_limiters.insert(UserTier::Team, RateLimiter::new(
            RateLimitPolicy::TokenBucket {
                capacity: 5000.0,
                refill_per_sec: 100.0,
            },
        ));
        tier_limiters.insert(UserTier::Enterprise, RateLimiter::new(
            RateLimitPolicy::Unlimited,
        ));

        Self { tier_limiters }
    }

    pub fn with_policies(policies: HashMap<UserTier, RateLimitPolicy>) -> Self {
        let mut tier_limiters = HashMap::new();
        for (tier, policy) in policies {
            tier_limiters.insert(tier, RateLimiter::new(policy));
        }
        Self { tier_limiters }
    }

    pub fn check(&self, identity: &str, tier: &UserTier, now_ms: u64) -> RateLimitResult {
        let limiter = self.tier_limiters.get(tier);
        match limiter {
            Some(l) => l.check(identity, now_ms),
            None => RateLimitResult {
                allowed: false,
                retry_after_ms: 60_000,
                remaining: Some(0),
                scope: format!("unknown-tier:{:?}", tier),
            },
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_starts_full() {
        let mut b = TokenBucket::new(10.0, 1.0, 1000);
        assert!(b.try_consume(5.0, 1000));
        assert!(b.try_consume(5.0, 1000));
        assert!(!b.try_consume(1.0, 1000));
    }

    #[test]
    fn test_token_bucket_refills() {
        let mut b = TokenBucket::new(10.0, 1.0, 1000);
        assert!(b.try_consume(10.0, 1000));
        assert!(!b.try_consume(1.0, 1000));
        // Wait 2 seconds — should have 2 tokens
        assert!(b.try_consume(2.0, 3000));
        assert!(!b.try_consume(1.0, 3000));
    }

    #[test]
    fn test_token_bucket_capped_at_capacity() {
        let mut b = TokenBucket::new(5.0, 100.0, 1000);
        // Wait a huge time — should still be capped at 5
        assert!(b.try_consume(5.0, 1_000_000));
        assert!(!b.try_consume(1.0, 1_000_000));
    }

    #[test]
    fn test_sliding_window_allows_up_to_max() {
        let mut w = SlidingWindow::new(3, 60_000);
        assert!(w.try_record(1000));
        assert!(w.try_record(2000));
        assert!(w.try_record(3000));
        assert!(!w.try_record(4000));
    }

    #[test]
    fn test_sliding_window_expires() {
        let mut w = SlidingWindow::new(2, 60_000);
        assert!(w.try_record(1000));
        assert!(w.try_record(2000));
        assert!(!w.try_record(3000));
        // Past the window
        assert!(w.try_record(70_000));
    }

    #[test]
    fn test_rate_limiter_unlimited() {
        let rl = RateLimiter::new(RateLimitPolicy::Unlimited);
        for i in 0..1000 {
            let r = rl.check("user1", 1000 + i);
            assert!(r.allowed, "Unlimited should always allow");
        }
    }

    #[test]
    fn test_rate_limiter_token_bucket() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 5.0, refill_per_sec: 1.0,
        });
        for _ in 0..5 {
            let r = rl.check("user1", 1000);
            assert!(r.allowed);
        }
        let r = rl.check("user1", 1000);
        assert!(!r.allowed);
        assert!(r.retry_after_ms > 0);
    }

    #[test]
    fn test_rate_limiter_sliding_window() {
        let rl = RateLimiter::new(RateLimitPolicy::SlidingWindow {
            max_requests: 3, window_ms: 10_000,
        });
        assert!(rl.check("user1", 1000).allowed);
        assert!(rl.check("user1", 2000).allowed);
        assert!(rl.check("user1", 3000).allowed);
        assert!(!rl.check("user1", 4000).allowed);
    }

    #[test]
    fn test_per_scope_independent() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 1.0, refill_per_sec: 1.0,
        });
        assert!(rl.check("user_a", 1000).allowed);
        assert!(!rl.check("user_a", 1000).allowed);
        // user_b has own bucket
        assert!(rl.check("user_b", 1000).allowed);
    }

    #[test]
    fn test_scope_specific_policy() {
        let mut rl = RateLimiter::new(RateLimitPolicy::SlidingWindow {
            max_requests: 5, window_ms: 60_000,
        });
        rl.set_policy("admin", RateLimitPolicy::Unlimited);
        // Admin scope is unlimited
        for _ in 0..100 {
            assert!(rl.check("admin-user", 1000).allowed);
        }
    }

    #[test]
    fn test_reset_clears_scope() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 1.0, refill_per_sec: 0.001,
        });
        assert!(rl.check("user1", 1000).allowed);
        assert!(!rl.check("user1", 1000).allowed);
        rl.reset("user1");
        assert!(rl.check("user1", 1000).allowed);
    }

    #[test]
    fn test_retry_after_calculated() {
        let rl = RateLimiter::new(RateLimitPolicy::SlidingWindow {
            max_requests: 1, window_ms: 10_000,
        });
        rl.check("user1", 1000);
        let r = rl.check("user1", 1500);
        assert!(!r.allowed);
        // Window expires at 1000 + 10_000 = 11_000. Request at 1500.
        // retry_after_ms should be 11_000 - 1500 = 9500.
        assert!(r.retry_after_ms > 9000 && r.retry_after_ms <= 10_000);
    }

    #[test]
    fn test_tiered_limiter_free_more_restricted() {
        let tl = TieredRateLimiter::new();

        // Free tier: 100/min → 101st should fail
        for i in 0..100 {
            let r = tl.check("user_free", &UserTier::Free, 1000 + i);
            assert!(r.allowed, "Free should allow request {}", i);
        }
        // In practice the test runs so fast they're all in the same window.
        let r = tl.check("user_free", &UserTier::Free, 1100);
        assert!(!r.allowed, "Free should block after 100 in window");
    }

    #[test]
    fn test_tiered_limiter_enterprise_unlimited() {
        let tl = TieredRateLimiter::new();
        for i in 0..10_000 {
            let r = tl.check("enterprise_user", &UserTier::Enterprise, 1000 + i);
            assert!(r.allowed);
        }
    }

    #[test]
    fn test_cost_weighting() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 10.0, refill_per_sec: 1.0,
        });
        // Expensive operations cost more tokens
        assert!(rl.check_with_cost("user1", 5.0, 1000).allowed);
        assert!(rl.check_with_cost("user1", 3.0, 1000).allowed);
        // Only 2 tokens left
        assert!(!rl.check_with_cost("user1", 5.0, 1000).allowed);
    }

    #[test]
    fn test_remaining_reported() {
        let rl = RateLimiter::new(RateLimitPolicy::SlidingWindow {
            max_requests: 5, window_ms: 60_000,
        });
        let r1 = rl.check("u", 1000);
        assert_eq!(r1.remaining, Some(4));
        let r2 = rl.check("u", 2000);
        assert_eq!(r2.remaining, Some(3));
    }

    #[test]
    fn test_tracked_count_grows() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 1.0, refill_per_sec: 1.0,
        });
        assert_eq!(rl.tracked_count(), 0);
        rl.check("a", 1000);
        rl.check("b", 1000);
        rl.check("c", 1000);
        assert_eq!(rl.tracked_count(), 3);
    }

    #[test]
    fn test_reset_all_clears() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 1.0, refill_per_sec: 1.0,
        });
        rl.check("a", 1000);
        rl.check("b", 1000);
        rl.reset_all();
        assert_eq!(rl.tracked_count(), 0);
    }

    // ============================================================
    // Stress / invariant tests for RateLimiter
    // ============================================================

    /// INVARIANT: Unlimited policy always allows and never has retry_after.
    #[test]
    fn invariant_unlimited_policy_always_allows() {
        let rl = RateLimiter::new(RateLimitPolicy::Unlimited);
        for t in 0..100u64 {
            let r = rl.check("alice", t * 100);
            assert!(r.allowed, "unlimited should always allow");
            assert_eq!(r.retry_after_ms, 0);
        }
    }

    /// INVARIANT: TokenBucket allowed → remaining decreases (or stays at 0
    /// once consumed). After capacity exhausted, retry_after_ms > 0.
    #[test]
    fn invariant_token_bucket_exhaustion_gives_retry() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 3.0, refill_per_sec: 0.01,  // very slow refill
        });
        for _ in 0..3 {
            assert!(rl.check("u", 1000).allowed);
        }
        let denied = rl.check("u", 1000);
        assert!(!denied.allowed, "4th request should be denied");
        assert!(denied.retry_after_ms > 0,
            "denied TokenBucket should suggest retry time");
    }

    /// INVARIANT: SlidingWindow with cap=N allows exactly N requests per window.
    #[test]
    fn invariant_sliding_window_hard_cap() {
        let rl = RateLimiter::new(RateLimitPolicy::SlidingWindow {
            max_requests: 5, window_ms: 10_000,
        });
        let mut allowed = 0;
        for _ in 0..10 {
            if rl.check("u", 1000).allowed {
                allowed += 1;
            }
        }
        assert_eq!(allowed, 5, "sliding window should cap at exactly 5");
    }

    /// INVARIANT: SlidingWindow admits new requests once the window elapses.
    #[test]
    fn invariant_sliding_window_recovers_after_window() {
        let rl = RateLimiter::new(RateLimitPolicy::SlidingWindow {
            max_requests: 2, window_ms: 1000,
        });
        assert!(rl.check("u", 1000).allowed);
        assert!(rl.check("u", 1001).allowed);
        assert!(!rl.check("u", 1002).allowed);
        // After window elapses
        assert!(rl.check("u", 3000).allowed);
    }

    /// INVARIANT: reset(scope) clears per-scope state; other scopes unaffected.
    #[test]
    fn invariant_reset_is_scoped() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 1.0, refill_per_sec: 0.001,
        });
        // Exhaust both a and b
        rl.check("a", 1000);
        rl.check("b", 1000);
        assert!(!rl.check("a", 1000).allowed);
        assert!(!rl.check("b", 1000).allowed);
        // Reset only a
        rl.reset("a");
        assert!(rl.check("a", 1000).allowed, "a should be refilled after reset");
        assert!(!rl.check("b", 1000).allowed, "b should still be exhausted");
    }

    /// INVARIANT: check() never panics on extreme time values.
    #[test]
    fn invariant_check_never_panics_on_extreme_time() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 5.0, refill_per_sec: 1.0,
        });
        let _ = rl.check("u", 0);
        let _ = rl.check("u", u64::MAX);
        let _ = rl.check("u", 1);
    }

    /// INVARIANT: retry_after_ms is 0 when allowed, >= 0 always (u64 property).
    #[test]
    fn invariant_retry_after_nonneg_and_zero_when_allowed() {
        let rl = RateLimiter::new(RateLimitPolicy::TokenBucket {
            capacity: 2.0, refill_per_sec: 1.0,
        });
        for i in 0..5 {
            let r = rl.check("u", 1000 + i);
            if r.allowed {
                assert_eq!(r.retry_after_ms, 0,
                    "allowed request must have retry_after=0");
            }
        }
    }
}
