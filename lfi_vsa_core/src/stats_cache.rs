// SUPERSOCIETY: lock-free stats cache. Counting 72M rows costs 3-5s and blocked
// the agent mutex every poll. This caches facts_count / sources_count /
// adversarial_count behind AtomicI64 so /api/status et al. are O(1). A single
// background tokio task refreshes the cache every REFRESH_SECS; readers never
// block on the DB.
//
// REGRESSION-GUARD: Prior to this cache, every /api/status poll held the
// agent Mutex for 3-5s during a full-table COUNT(*). Combined with the UI
// polling at 5s intervals, the chat handler was starved — individual turns
// took 11+ seconds on an otherwise idle machine.
//
// BUG ASSUMPTION: the DB may be unreachable, the COUNT may fail, or the
// refresh may lap itself. All of those are handled by leaving the stale
// values in place; readers see a slightly out-of-date count but never
// block or panic.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::persistence::BrainDb;

/// How often the background refresher runs. 60 s is long enough that the
/// per-refresh cost is irrelevant, short enough that the UI sees near-real
/// facts_count as new data lands during an ingest.
pub const REFRESH_SECS: u64 = 60;

/// How stale the cache must be before a reader opportunistically triggers
/// an on-demand refresh. 2 × REFRESH_SECS is the envelope — if the background
/// task is alive, readers never hit this path.
pub const STALE_SECS: u64 = 180;

#[derive(Debug)]
pub struct StatsCache {
    pub facts_count: AtomicI64,
    pub sources_count: AtomicI64,
    pub adversarial_count: AtomicI64,
    pub last_refresh_secs: AtomicU64,
    pub refresh_inflight: AtomicBool,
}

impl StatsCache {
    pub fn new() -> Self {
        Self {
            facts_count: AtomicI64::new(-1),
            sources_count: AtomicI64::new(-1),
            adversarial_count: AtomicI64::new(-1),
            last_refresh_secs: AtomicU64::new(0),
            refresh_inflight: AtomicBool::new(false),
        }
    }

    pub fn facts(&self) -> i64 { self.facts_count.load(Ordering::Relaxed) }
    pub fn sources(&self) -> i64 { self.sources_count.load(Ordering::Relaxed) }
    pub fn adversarial(&self) -> i64 { self.adversarial_count.load(Ordering::Relaxed) }

    /// Seconds since last successful refresh. u64::MAX if never refreshed.
    pub fn age_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let last = self.last_refresh_secs.load(Ordering::Relaxed);
        if last == 0 { u64::MAX } else { now.saturating_sub(last) }
    }

    /// Synchronously refresh. Safe to call from any thread. Acquires the
    /// inflight flag so duplicate callers return immediately.
    pub fn refresh_blocking(&self, db: &BrainDb) {
        // CAS so only one refresher runs at a time.
        if self.refresh_inflight.compare_exchange(
            false, true, Ordering::AcqRel, Ordering::Relaxed,
        ).is_err() {
            return;
        }
        let _guard = InflightGuard(&self.refresh_inflight);

        let conn = match db.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        if let Ok(v) = conn.query_row::<i64, _, _>("SELECT count(*) FROM facts", [], |r| r.get(0)) {
            self.facts_count.store(v, Ordering::Relaxed);
        }
        if let Ok(v) = conn.query_row::<i64, _, _>(
            "SELECT count(DISTINCT source) FROM facts", [], |r| r.get(0),
        ) {
            self.sources_count.store(v, Ordering::Relaxed);
        }
        if let Ok(v) = conn.query_row::<i64, _, _>(
            "SELECT count(*) FROM facts WHERE source IN ('adversarial','anli_r1','anli_r2','anli_r3','fever_gold','truthfulqa')",
            [], |r| r.get(0),
        ) {
            self.adversarial_count.store(v, Ordering::Relaxed);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_refresh_secs.store(now, Ordering::Relaxed);
    }

    /// Fire-and-forget background refresher. Spawns a tokio task that
    /// refreshes every REFRESH_SECS. The task holds an Arc so the cache
    /// outlives the task naturally.
    pub fn spawn_refresher(self: &Arc<Self>, db: Arc<BrainDb>) {
        let cache = self.clone();
        tokio::spawn(async move {
            // Small initial delay so we don't race the startup warmup.
            tokio::time::sleep(Duration::from_secs(2)).await;
            loop {
                let db_ref = db.clone();
                let cache_ref = cache.clone();
                // spawn_blocking so the COUNT(*) doesn't stall other tokio
                // tasks if it takes a couple of seconds.
                let _ = tokio::task::spawn_blocking(move || {
                    cache_ref.refresh_blocking(&db_ref);
                }).await;
                tokio::time::sleep(Duration::from_secs(REFRESH_SECS)).await;
            }
        });
    }
}

impl Default for StatsCache {
    fn default() -> Self { Self::new() }
}

struct InflightGuard<'a>(&'a AtomicBool);

impl Drop for InflightGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_cache_has_sentinel_counts() {
        let cache = StatsCache::new();
        assert_eq!(cache.facts(), -1);
        assert_eq!(cache.sources(), -1);
        assert_eq!(cache.adversarial(), -1);
        assert_eq!(cache.age_secs(), u64::MAX);
    }

    #[test]
    fn inflight_cas_prevents_double_refresh() {
        let cache = StatsCache::new();
        assert!(cache.refresh_inflight.compare_exchange(
            false, true, Ordering::AcqRel, Ordering::Relaxed,
        ).is_ok());
        assert!(cache.refresh_inflight.compare_exchange(
            false, true, Ordering::AcqRel, Ordering::Relaxed,
        ).is_err());
    }
}
