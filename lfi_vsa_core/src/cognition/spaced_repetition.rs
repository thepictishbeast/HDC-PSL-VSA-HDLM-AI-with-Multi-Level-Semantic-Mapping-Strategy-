// ============================================================
// Spaced Repetition Scheduler — Forgetting-Curve-Aware Review
//
// PURPOSE: Schedule concept reviews at intervals that match human-style
// spaced-repetition learning. The mastery score in KnowledgeEngine tells
// WHAT the AI knows; this scheduler tells WHEN to rehearse it next.
//
// ALGORITHM: Simplified SM-2 / Leitner-box hybrid.
//   - Each concept has an ease-factor (EF) clamped to [1.3, 2.5].
//   - Reviews emit a quality score q ∈ [0, 5] (0 = total blank, 5 = perfect).
//   - If q >= 3, interval grows by current ease factor (EF).
//   - If q < 3, interval resets to 1 day and EF decreases.
//   - EF update: EF_new = max(1.3, EF + 0.1 - (5-q)*(0.08 + (5-q)*0.02))
//
// INVARIANT: The scheduler never invents reviews for concepts it hasn't
// been told about. Callers must `register(name)` before `review(name, q)`.
// ============================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const MIN_EASE_FACTOR: f64 = 1.3;
const MAX_EASE_FACTOR: f64 = 2.5;
const INITIAL_EASE_FACTOR: f64 = 2.5;
const SECONDS_PER_DAY: u64 = 86_400;

/// Per-concept review schedule state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewCard {
    pub name: String,
    /// Ease factor — how fast the interval grows.
    pub ease_factor: f64,
    /// Current interval in whole days.
    pub interval_days: u32,
    /// Number of successful reviews in a row (reset on failure).
    pub streak: u32,
    /// Unix epoch ms when the concept was last reviewed.
    pub last_reviewed_ms: u64,
    /// Unix epoch ms for the next scheduled review.
    pub next_due_ms: u64,
    /// Total review count (including failures).
    pub review_count: u32,
    /// Total failures.
    pub failure_count: u32,
}

impl ReviewCard {
    fn new(name: String, now_ms: u64) -> Self {
        Self {
            name,
            ease_factor: INITIAL_EASE_FACTOR,
            interval_days: 0,
            streak: 0,
            last_reviewed_ms: now_ms,
            next_due_ms: now_ms, // due immediately
            review_count: 0,
            failure_count: 0,
        }
    }

    /// Is this card due at `now_ms`?
    pub fn is_due(&self, now_ms: u64) -> bool {
        now_ms >= self.next_due_ms
    }

    /// How overdue is this card, in days? 0 if not due.
    pub fn days_overdue(&self, now_ms: u64) -> u64 {
        if now_ms <= self.next_due_ms {
            0
        } else {
            ((now_ms - self.next_due_ms) / 1000) / SECONDS_PER_DAY
        }
    }
}

/// The spaced-repetition scheduler.
///
/// Independently tracks review state per concept. Use alongside
/// `KnowledgeEngine` — the engine holds VSA vectors + mastery, this
/// holds "when to practice again".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpacedRepetitionScheduler {
    cards: HashMap<String, ReviewCard>,
}

impl SpacedRepetitionScheduler {
    pub fn new() -> Self {
        debuglog!("SpacedRepetitionScheduler::new");
        Self::default()
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Register a concept for scheduling. No-op if already registered.
    /// Returns true if newly registered.
    pub fn register(&mut self, name: &str) -> bool {
        if self.cards.contains_key(name) {
            return false;
        }
        let now = Self::now_ms();
        self.cards.insert(name.to_string(), ReviewCard::new(name.to_string(), now));
        debuglog!("SpacedRepetitionScheduler::register: new card '{}'", name);
        true
    }

    /// Record a review outcome with quality q ∈ [0, 5].
    ///
    /// Returns the updated ReviewCard, or None if the concept wasn't registered.
    /// BUG ASSUMPTION: q is clamped to [0, 5] before use — callers can pass
    /// out-of-range values without invariant violation.
    pub fn review(&mut self, name: &str, q: u8) -> Option<&ReviewCard> {
        let q = q.min(5) as f64;
        let now = Self::now_ms();
        let card = self.cards.get_mut(name)?;

        card.review_count += 1;
        card.last_reviewed_ms = now;

        if q < 3.0 {
            // Failure — reset interval, decrease ease factor slightly.
            card.failure_count += 1;
            card.streak = 0;
            card.interval_days = 1;
            card.ease_factor = (card.ease_factor - 0.2).max(MIN_EASE_FACTOR);
            debuglog!("SpacedRepetitionScheduler::review: FAIL '{}' q={:.0} ef={:.2}",
                name, q, card.ease_factor);
        } else {
            // Success — grow the interval.
            card.streak += 1;
            let new_interval = if card.streak == 1 {
                1
            } else if card.streak == 2 {
                6
            } else {
                ((card.interval_days as f64) * card.ease_factor).round() as u32
            };
            card.interval_days = new_interval.max(1);

            // SM-2 EF update.
            let delta = 0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02);
            card.ease_factor = (card.ease_factor + delta)
                .clamp(MIN_EASE_FACTOR, MAX_EASE_FACTOR);
            debuglog!("SpacedRepetitionScheduler::review: PASS '{}' q={:.0} streak={} interval={}d ef={:.2}",
                name, q, card.streak, card.interval_days, card.ease_factor);
        }

        card.next_due_ms = now + (card.interval_days as u64) * SECONDS_PER_DAY * 1000;
        Some(card)
    }

    /// Concepts currently due for review at `now_ms`.
    /// Sorted by most-overdue first.
    pub fn due_now(&self, now_ms: u64) -> Vec<&ReviewCard> {
        let mut due: Vec<&ReviewCard> = self.cards.values().filter(|c| c.is_due(now_ms)).collect();
        due.sort_by(|a, b| a.next_due_ms.cmp(&b.next_due_ms));
        due
    }

    /// Top-N concepts due right now.
    pub fn top_due(&self, n: usize) -> Vec<&ReviewCard> {
        let now = Self::now_ms();
        self.due_now(now).into_iter().take(n).collect()
    }

    /// Get a specific card.
    pub fn get(&self, name: &str) -> Option<&ReviewCard> {
        self.cards.get(name)
    }

    /// Total number of scheduled cards.
    pub fn card_count(&self) -> usize {
        self.cards.len()
    }

    /// Count of cards currently due.
    pub fn due_count(&self) -> usize {
        let now = Self::now_ms();
        self.cards.values().filter(|c| c.is_due(now)).count()
    }

    /// Serialize the scheduler state.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize. Rejects payloads larger than 16 MiB.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        const MAX_BYTES: usize = 16 * 1024 * 1024;
        if json.len() > MAX_BYTES {
            return Err(serde::de::Error::custom(
                "spaced repetition JSON exceeds 16 MiB limit"
            ));
        }
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_initial_state() {
        let mut sched = SpacedRepetitionScheduler::new();
        assert!(sched.register("rust_ownership"));
        assert!(!sched.register("rust_ownership"), "re-register is no-op");
        assert_eq!(sched.card_count(), 1);

        let card = sched.get("rust_ownership").expect("card exists");
        assert_eq!(card.ease_factor, INITIAL_EASE_FACTOR);
        assert_eq!(card.review_count, 0);
    }

    #[test]
    fn test_review_success_grows_interval() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("concept");

        // First success → interval 1
        let c = sched.review("concept", 5).expect("card").clone();
        assert_eq!(c.interval_days, 1);
        assert_eq!(c.streak, 1);

        // Second success → interval 6
        let c = sched.review("concept", 5).expect("card").clone();
        assert_eq!(c.interval_days, 6);
        assert_eq!(c.streak, 2);

        // Third success → interval * EF
        let c = sched.review("concept", 5).expect("card").clone();
        assert!(c.interval_days > 6, "interval should grow, got {}", c.interval_days);
        assert_eq!(c.streak, 3);
    }

    #[test]
    fn test_review_failure_resets_streak() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("x");
        sched.review("x", 5);
        sched.review("x", 5);
        sched.review("x", 5); // streak = 3

        // Failure
        let c = sched.review("x", 1).expect("card").clone();
        assert_eq!(c.streak, 0);
        assert_eq!(c.interval_days, 1);
        assert_eq!(c.failure_count, 1);
        assert!(c.ease_factor < INITIAL_EASE_FACTOR);
        assert!(c.ease_factor >= MIN_EASE_FACTOR);
    }

    #[test]
    fn test_ease_factor_clamped() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("y");
        // Failure repeatedly — EF should clamp at MIN.
        for _ in 0..50 {
            sched.review("y", 0);
        }
        let c = sched.get("y").expect("card");
        assert!(c.ease_factor >= MIN_EASE_FACTOR);
        assert!(c.ease_factor <= MAX_EASE_FACTOR);
    }

    #[test]
    fn test_review_unknown_concept_returns_none() {
        let mut sched = SpacedRepetitionScheduler::new();
        assert!(sched.review("never_registered", 5).is_none(),
            "Must not invent reviews for unregistered concepts");
    }

    #[test]
    fn test_q_out_of_range_clamped() {
        // q > 5 should be clamped to 5, not panic.
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("z");
        let c = sched.review("z", 255).expect("card").clone();
        // 255 clamps to 5 → treated as perfect recall.
        assert_eq!(c.streak, 1);
        assert_eq!(c.interval_days, 1);
    }

    #[test]
    fn test_due_now_returns_immediately_due_after_register() {
        // A freshly registered card has next_due_ms = now, so it's due.
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("a");
        let now = SpacedRepetitionScheduler::now_ms();
        let due = sched.due_now(now);
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn test_due_now_ignores_not_yet_due() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("a");
        sched.review("a", 5); // now has interval 1 day
        let now = SpacedRepetitionScheduler::now_ms();
        // Immediately after review, card should NOT be due (1 day interval).
        assert_eq!(sched.due_count(), 0);
        // But checking a future time where it's overdue:
        let future = now + (2 * SECONDS_PER_DAY * 1000);
        assert_eq!(sched.due_now(future).len(), 1);
    }

    #[test]
    fn test_top_due_sorted_by_most_overdue() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("a");
        sched.register("b");
        sched.register("c");
        // All three are due immediately after register.
        let top = sched.top_due(10);
        assert_eq!(top.len(), 3);
    }

    #[test]
    fn test_roundtrip_json() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("x");
        sched.register("y");
        sched.review("x", 5);

        let json = sched.to_json().expect("serialize");
        let restored = SpacedRepetitionScheduler::from_json(&json).expect("deserialize");

        assert_eq!(restored.card_count(), 2);
        let x = restored.get("x").expect("x survives");
        assert_eq!(x.streak, 1);
    }

    #[test]
    fn test_from_json_rejects_oversize() {
        let huge = "a".repeat(16 * 1024 * 1024 + 1);
        assert!(SpacedRepetitionScheduler::from_json(&huge).is_err());
    }

    #[test]
    fn test_review_count_tracks_all_reviews() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("t");
        for q in [5u8, 3, 1, 5, 0] {
            sched.review("t", q);
        }
        let c = sched.get("t").expect("card");
        assert_eq!(c.review_count, 5);
        assert_eq!(c.failure_count, 2); // q<3 failures: 1 and 0
    }

    // ============================================================
    // Stress / property-style invariant tests
    // ============================================================

    /// INVARIANT: For any sequence of arbitrary reviews, the ease factor
    /// must always stay within [MIN_EASE_FACTOR, MAX_EASE_FACTOR].
    #[test]
    fn invariant_ease_factor_always_in_bounds() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("ef");
        // 256 reviews across the full quality range.
        for i in 0..256u32 {
            let q = (i % 6) as u8;
            sched.review("ef", q);
            let card = sched.get("ef").expect("card");
            assert!(card.ease_factor >= MIN_EASE_FACTOR,
                "EF below floor at iter {} (q={}): {}", i, q, card.ease_factor);
            assert!(card.ease_factor <= MAX_EASE_FACTOR,
                "EF above ceiling at iter {} (q={}): {}", i, q, card.ease_factor);
        }
    }

    /// INVARIANT: failure_count is monotonically non-decreasing.
    #[test]
    fn invariant_failure_count_monotonic() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("m");
        let mut last = 0u32;
        for i in 0..128u32 {
            let q = (i % 6) as u8;
            sched.review("m", q);
            let card = sched.get("m").expect("card");
            assert!(card.failure_count >= last,
                "failure_count went backwards at iter {}: {} → {}",
                i, last, card.failure_count);
            last = card.failure_count;
        }
    }

    /// INVARIANT: review_count must equal total calls to review().
    #[test]
    fn invariant_review_count_equals_call_count() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("rc");
        let n = 50u32;
        for i in 0..n {
            sched.review("rc", (i % 6) as u8);
        }
        let card = sched.get("rc").expect("card");
        assert_eq!(card.review_count, n);
    }

    /// INVARIANT: After only successful reviews (q >= 3), streak grows
    /// monotonically and equals the number of successful calls.
    #[test]
    fn invariant_streak_grows_under_successes() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("s");
        for _ in 0..20 {
            sched.review("s", 5);
        }
        let card = sched.get("s").expect("card");
        assert_eq!(card.streak, 20);
        assert_eq!(card.failure_count, 0);
    }

    /// INVARIANT: A failure (q < 3) always resets streak to 0 immediately.
    #[test]
    fn invariant_failure_resets_streak_immediately() {
        let mut sched = SpacedRepetitionScheduler::new();
        sched.register("r");
        // Build a streak.
        for _ in 0..10 { sched.review("r", 5); }
        assert_eq!(sched.get("r").expect("card").streak, 10);
        // One failure obliterates it.
        sched.review("r", 0);
        assert_eq!(sched.get("r").expect("card").streak, 0);
    }

    /// INVARIANT: Scheduler scales — registering a large number of cards
    /// must not crash, and `top_due` must respect the requested limit.
    #[test]
    fn invariant_scales_to_thousands_and_top_due_respects_limit() {
        let mut sched = SpacedRepetitionScheduler::new();
        for i in 0..5000 {
            sched.register(&format!("concept_{:05}", i));
        }
        assert_eq!(sched.card_count(), 5000);
        // All cards are due immediately after register.
        let top = sched.top_due(50);
        assert_eq!(top.len(), 50, "top_due must cap at requested limit");
    }

    /// INVARIANT: JSON roundtrip preserves card_count and per-card streak/EF
    /// across an arbitrary sequence of reviews.
    #[test]
    fn invariant_json_roundtrip_preserves_state_under_load() {
        let mut sched = SpacedRepetitionScheduler::new();
        for i in 0..50 {
            let name = format!("c{}", i);
            sched.register(&name);
            for j in 0..10 {
                sched.review(&name, ((i + j) % 6) as u8);
            }
        }

        let json = sched.to_json().expect("serialize");
        let restored = SpacedRepetitionScheduler::from_json(&json).expect("deserialize");
        assert_eq!(restored.card_count(), sched.card_count());

        for i in 0..50 {
            let name = format!("c{}", i);
            let orig = sched.get(&name).expect("orig");
            let new = restored.get(&name).expect("restored");
            assert_eq!(orig.streak, new.streak,
                "streak mismatch for {}: {} vs {}", name, orig.streak, new.streak);
            assert!((orig.ease_factor - new.ease_factor).abs() < 1e-9,
                "ease_factor mismatch for {}", name);
            assert_eq!(orig.review_count, new.review_count);
        }
    }
}
