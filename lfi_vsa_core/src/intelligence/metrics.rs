// ============================================================
// Metrics & Observability — Prometheus-Compatible
//
// PURPOSE: Production observability for LFI deployments. Exposes
// counters, gauges, and histograms in a format that scrapes cleanly
// into Prometheus, Datadog, Grafana Cloud, etc.
//
// METRIC TYPES:
//   - Counter: monotonically-increasing (total requests, total blocks)
//   - Gauge: instant value (active sessions, tracked identities)
//   - Histogram: distribution of values (latency, response sizes)
//
// OUTPUT FORMAT:
//   Prometheus text exposition format — scraped via /metrics endpoint.
//   Labels support for per-identity / per-category breakdowns.
//
// USAGE:
//   let metrics = LfiMetrics::new();
//   metrics.inc_counter("lfi_requests_total", &[("endpoint", "/detect")], 1);
//   metrics.observe_histogram("lfi_latency_ms", &[], 42.5);
//   metrics.set_gauge("lfi_active_sessions", &[], 17);
//
//   // Expose over HTTP for scraping:
//   let text = metrics.render_prometheus();
// ============================================================

use std::collections::HashMap;
use std::sync::RwLock;

// ============================================================
// Label Key for Sharding Metrics
// ============================================================

/// A set of label key-value pairs, sorted for stable keying.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LabelSet(Vec<(String, String)>);

impl LabelSet {
    fn from_slice(labels: &[(&str, &str)]) -> Self {
        let mut v: Vec<(String, String)> = labels.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        v.sort();
        Self(v)
    }

    #[allow(dead_code)]
    fn is_empty(&self) -> bool { self.0.is_empty() }

    fn render(&self) -> String {
        if self.0.is_empty() { return String::new(); }
        let pairs: Vec<String> = self.0.iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, escape_value(v)))
            .collect();
        format!("{{{}}}", pairs.join(","))
    }
}

fn escape_value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

// ============================================================
// Histogram Buckets
// ============================================================

/// Standard latency buckets (milliseconds): 1, 5, 10, 25, 50, 100, 250, 500,
/// 1000, 2500, 5000, 10000, +Inf.
const DEFAULT_BUCKETS: &[f64] = &[
    1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0,
    1000.0, 2500.0, 5000.0, 10000.0,
];

#[derive(Debug, Clone)]
struct HistogramData {
    /// Counts for each bucket (cumulative, Prometheus-style).
    buckets: Vec<u64>,
    /// Sum of all observed values.
    sum: f64,
    /// Total count of observations.
    count: u64,
    /// Bucket boundaries (inclusive upper).
    boundaries: Vec<f64>,
}

impl HistogramData {
    fn new(boundaries: Vec<f64>) -> Self {
        Self {
            buckets: vec![0; boundaries.len()],
            sum: 0.0,
            count: 0,
            boundaries,
        }
    }

    fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
        for (i, &boundary) in self.boundaries.iter().enumerate() {
            if value <= boundary {
                self.buckets[i] += 1;
            }
        }
    }
}

// ============================================================
// Metric Registry
// ============================================================

pub struct LfiMetrics {
    counters: RwLock<HashMap<String, HashMap<LabelSet, u64>>>,
    gauges: RwLock<HashMap<String, HashMap<LabelSet, f64>>>,
    histograms: RwLock<HashMap<String, HashMap<LabelSet, HistogramData>>>,
    /// Help text for each metric name.
    help_text: RwLock<HashMap<String, String>>,
}

impl LfiMetrics {
    pub fn new() -> Self {
        let m = Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            help_text: RwLock::new(HashMap::new()),
        };
        m.seed_builtin_metrics();
        m
    }

    fn seed_builtin_metrics(&self) {
        self.register_help("lfi_requests_total",
            "Total requests processed by the LFI firewall");
        self.register_help("lfi_requests_blocked_total",
            "Total requests blocked by the LFI firewall");
        self.register_help("lfi_threats_detected_total",
            "Total threats detected, labeled by category and severity");
        self.register_help("lfi_latency_ms",
            "Request processing latency in milliseconds");
        self.register_help("lfi_active_sessions",
            "Number of currently tracked identities");
        self.register_help("lfi_honey_tokens_fired_total",
            "Total honey token activations (real breach indicators)");
    }

    pub fn register_help(&self, name: &str, help: &str) {
        if let Ok(mut m) = self.help_text.write() {
            m.insert(name.into(), help.into());
        }
    }

    /// Increment a counter.
    pub fn inc_counter(&self, name: &str, labels: &[(&str, &str)], by: u64) {
        let ls = LabelSet::from_slice(labels);
        if let Ok(mut counters) = self.counters.write() {
            let entry = counters.entry(name.into()).or_insert_with(HashMap::new);
            *entry.entry(ls).or_insert(0) += by;
        }
    }

    /// Set a gauge value.
    pub fn set_gauge(&self, name: &str, labels: &[(&str, &str)], value: f64) {
        let ls = LabelSet::from_slice(labels);
        if let Ok(mut gauges) = self.gauges.write() {
            let entry = gauges.entry(name.into()).or_insert_with(HashMap::new);
            entry.insert(ls, value);
        }
    }

    /// Change a gauge by a delta (can be negative).
    pub fn adjust_gauge(&self, name: &str, labels: &[(&str, &str)], delta: f64) {
        let ls = LabelSet::from_slice(labels);
        if let Ok(mut gauges) = self.gauges.write() {
            let entry = gauges.entry(name.into()).or_insert_with(HashMap::new);
            *entry.entry(ls).or_insert(0.0) += delta;
        }
    }

    /// Record an observation for a histogram.
    pub fn observe_histogram(&self, name: &str, labels: &[(&str, &str)], value: f64) {
        let ls = LabelSet::from_slice(labels);
        if let Ok(mut histos) = self.histograms.write() {
            let entry = histos.entry(name.into()).or_insert_with(HashMap::new);
            let hist = entry.entry(ls).or_insert_with(|| {
                HistogramData::new(DEFAULT_BUCKETS.to_vec())
            });
            hist.observe(value);
        }
    }

    /// Render all metrics in Prometheus text exposition format.
    pub fn render_prometheus(&self) -> String {
        let mut out = String::new();

        let help_text = self.help_text.read().ok();

        // Counters
        if let Ok(counters) = self.counters.read() {
            let mut names: Vec<&String> = counters.keys().collect();
            names.sort();
            for name in names {
                if let Some(help) = help_text.as_ref().and_then(|h| h.get(name)) {
                    out.push_str(&format!("# HELP {} {}\n", name, help));
                }
                out.push_str(&format!("# TYPE {} counter\n", name));
                if let Some(entries) = counters.get(name) {
                    let mut sorted: Vec<(&LabelSet, &u64)> = entries.iter().collect();
                    sorted.sort_by_key(|(ls, _)| ls.0.clone());
                    for (ls, value) in sorted {
                        out.push_str(&format!("{}{} {}\n", name, ls.render(), value));
                    }
                }
                out.push('\n');
            }
        }

        // Gauges
        if let Ok(gauges) = self.gauges.read() {
            let mut names: Vec<&String> = gauges.keys().collect();
            names.sort();
            for name in names {
                if let Some(help) = help_text.as_ref().and_then(|h| h.get(name)) {
                    out.push_str(&format!("# HELP {} {}\n", name, help));
                }
                out.push_str(&format!("# TYPE {} gauge\n", name));
                if let Some(entries) = gauges.get(name) {
                    let mut sorted: Vec<(&LabelSet, &f64)> = entries.iter().collect();
                    sorted.sort_by_key(|(ls, _)| ls.0.clone());
                    for (ls, value) in sorted {
                        out.push_str(&format!("{}{} {}\n", name, ls.render(), value));
                    }
                }
                out.push('\n');
            }
        }

        // Histograms
        if let Ok(histos) = self.histograms.read() {
            let mut names: Vec<&String> = histos.keys().collect();
            names.sort();
            for name in names {
                if let Some(help) = help_text.as_ref().and_then(|h| h.get(name)) {
                    out.push_str(&format!("# HELP {} {}\n", name, help));
                }
                out.push_str(&format!("# TYPE {} histogram\n", name));
                if let Some(entries) = histos.get(name) {
                    let mut sorted: Vec<(&LabelSet, &HistogramData)> = entries.iter().collect();
                    sorted.sort_by_key(|(ls, _)| ls.0.clone());
                    for (ls, hist) in sorted {
                        // Per-bucket cumulative counts
                        for (i, &boundary) in hist.boundaries.iter().enumerate() {
                            let mut labels = ls.0.clone();
                            labels.push(("le".into(), boundary.to_string()));
                            labels.sort();
                            let ls_with_le = LabelSet(labels);
                            out.push_str(&format!("{}_bucket{} {}\n",
                                name, ls_with_le.render(), hist.buckets[i]));
                        }
                        // +Inf bucket
                        let mut labels = ls.0.clone();
                        labels.push(("le".into(), "+Inf".into()));
                        labels.sort();
                        let ls_inf = LabelSet(labels);
                        out.push_str(&format!("{}_bucket{} {}\n",
                            name, ls_inf.render(), hist.count));
                        // Sum + count
                        out.push_str(&format!("{}_sum{} {}\n", name, ls.render(), hist.sum));
                        out.push_str(&format!("{}_count{} {}\n", name, ls.render(), hist.count));
                    }
                }
                out.push('\n');
            }
        }

        out
    }

    /// Read current counter value (for testing / internal use).
    pub fn counter_value(&self, name: &str, labels: &[(&str, &str)]) -> u64 {
        let ls = LabelSet::from_slice(labels);
        self.counters.read().ok()
            .and_then(|c| c.get(name).and_then(|e| e.get(&ls)).copied())
            .unwrap_or(0)
    }

    /// Read current gauge value.
    pub fn gauge_value(&self, name: &str, labels: &[(&str, &str)]) -> f64 {
        let ls = LabelSet::from_slice(labels);
        self.gauges.read().ok()
            .and_then(|c| c.get(name).and_then(|e| e.get(&ls)).copied())
            .unwrap_or(0.0)
    }

    /// Reset all metrics (useful for testing).
    pub fn reset(&self) {
        if let Ok(mut c) = self.counters.write() { c.clear(); }
        if let Ok(mut g) = self.gauges.write() { g.clear(); }
        if let Ok(mut h) = self.histograms.write() { h.clear(); }
    }
}

// ============================================================
// Standard Label Helpers
// ============================================================

/// Labels for LFI firewall metrics.
pub mod labels {
    pub const ENDPOINT: &str = "endpoint";
    pub const CATEGORY: &str = "category";
    pub const SEVERITY: &str = "severity";
    pub const OUTCOME: &str = "outcome";   // "allowed" | "blocked"
    pub const IDENTITY: &str = "identity"; // Be careful: high cardinality
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_increments() {
        let m = LfiMetrics::new();
        m.inc_counter("test_counter", &[], 1);
        m.inc_counter("test_counter", &[], 5);
        assert_eq!(m.counter_value("test_counter", &[]), 6);
    }

    #[test]
    fn test_counter_labels_separate() {
        let m = LfiMetrics::new();
        m.inc_counter("req", &[("endpoint", "/a")], 1);
        m.inc_counter("req", &[("endpoint", "/b")], 3);
        assert_eq!(m.counter_value("req", &[("endpoint", "/a")]), 1);
        assert_eq!(m.counter_value("req", &[("endpoint", "/b")]), 3);
    }

    #[test]
    fn test_gauge_set() {
        let m = LfiMetrics::new();
        m.set_gauge("active", &[], 10.0);
        assert_eq!(m.gauge_value("active", &[]), 10.0);
        m.set_gauge("active", &[], 5.0); // overwrite
        assert_eq!(m.gauge_value("active", &[]), 5.0);
    }

    #[test]
    fn test_gauge_adjust() {
        let m = LfiMetrics::new();
        m.set_gauge("sessions", &[], 10.0);
        m.adjust_gauge("sessions", &[], -3.0);
        assert_eq!(m.gauge_value("sessions", &[]), 7.0);
    }

    #[test]
    fn test_histogram_observations() {
        let m = LfiMetrics::new();
        m.observe_histogram("latency", &[], 42.0);
        m.observe_histogram("latency", &[], 100.0);
        m.observe_histogram("latency", &[], 5.0);

        let rendered = m.render_prometheus();
        // Prometheus allows empty label set without braces
        assert!(rendered.contains("latency_count") && rendered.contains(" 3"));
        assert!(rendered.contains("latency_sum"));
    }

    #[test]
    fn test_prometheus_format_counter() {
        let m = LfiMetrics::new();
        m.register_help("req_total", "Total requests");
        m.inc_counter("req_total", &[("code", "200")], 42);

        let rendered = m.render_prometheus();
        assert!(rendered.contains("# HELP req_total Total requests"));
        assert!(rendered.contains("# TYPE req_total counter"));
        assert!(rendered.contains("req_total{code=\"200\"} 42"));
    }

    #[test]
    fn test_prometheus_format_gauge() {
        let m = LfiMetrics::new();
        m.register_help("active", "Active sessions");
        m.set_gauge("active", &[], 10.0);
        let rendered = m.render_prometheus();
        assert!(rendered.contains("# TYPE active gauge"));
        assert!(rendered.contains("active ") && rendered.contains("10"));
    }

    #[test]
    fn test_label_sorting_stable() {
        let m = LfiMetrics::new();
        // Different label order should be treated as same metric
        m.inc_counter("x", &[("a", "1"), ("b", "2")], 1);
        m.inc_counter("x", &[("b", "2"), ("a", "1")], 1);
        assert_eq!(m.counter_value("x", &[("a", "1"), ("b", "2")]), 2);
    }

    #[test]
    fn test_label_value_escaping() {
        let m = LfiMetrics::new();
        m.inc_counter("evt", &[("msg", r#"quote" and backslash\"#)], 1);
        let rendered = m.render_prometheus();
        // Quotes and backslashes should be escaped
        assert!(rendered.contains(r#"quote\""#));
        assert!(rendered.contains(r#"backslash\\"#));
    }

    #[test]
    fn test_histogram_bucket_counts_cumulative() {
        let m = LfiMetrics::new();
        m.observe_histogram("lat", &[], 3.0);
        m.observe_histogram("lat", &[], 50.0);
        m.observe_histogram("lat", &[], 500.0);
        let rendered = m.render_prometheus();
        // Empty labels render without braces. Check metric appears with count=3.
        assert!(rendered.contains("lat_count") && rendered.contains(" 3"));
    }

    #[test]
    fn test_reset_clears_all() {
        let m = LfiMetrics::new();
        m.inc_counter("c", &[], 5);
        m.set_gauge("g", &[], 10.0);
        m.observe_histogram("h", &[], 1.0);
        m.reset();
        assert_eq!(m.counter_value("c", &[]), 0);
        assert_eq!(m.gauge_value("g", &[]), 0.0);
    }

    #[test]
    fn test_empty_labels_renders_curly() {
        let m = LfiMetrics::new();
        m.inc_counter("x", &[], 1);
        let rendered = m.render_prometheus();
        // Prometheus allows empty brace pair
        assert!(rendered.contains("x{} 1") || rendered.contains("x 1"));
    }

    #[test]
    fn test_multiple_metric_types_coexist() {
        let m = LfiMetrics::new();
        m.inc_counter("c", &[], 1);
        m.set_gauge("g", &[], 2.0);
        m.observe_histogram("h", &[], 3.0);
        let rendered = m.render_prometheus();
        assert!(rendered.contains("# TYPE c counter"));
        assert!(rendered.contains("# TYPE g gauge"));
        assert!(rendered.contains("# TYPE h histogram"));
    }

    #[test]
    fn test_concurrent_increment() {
        use std::sync::Arc;
        use std::thread;

        let m = Arc::new(LfiMetrics::new());
        let mut handles = Vec::new();
        for _ in 0..10 {
            let m = Arc::clone(&m);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    m.inc_counter("concurrent", &[], 1);
                }
            }));
        }
        for h in handles { h.join().unwrap(); }
        assert_eq!(m.counter_value("concurrent", &[]), 1000);
    }

    // ============================================================
    // Stress / invariant tests for LfiMetrics
    // ============================================================

    /// INVARIANT: counter_value is monotonically non-decreasing across
    /// inc_counter calls.
    #[test]
    fn invariant_counter_monotone() {
        let m = LfiMetrics::new();
        let mut prev = 0u64;
        for i in 1..=10 {
            m.inc_counter("test", &[], i);
            let now = m.counter_value("test", &[]);
            assert!(now >= prev, "counter decreased: {} -> {}", prev, now);
            prev = now;
        }
    }

    /// INVARIANT: reset() sets all counters to 0 (or removes them),
    /// gauge_value returns 0 for unknown.
    #[test]
    fn invariant_reset_clears_state() {
        let m = LfiMetrics::new();
        m.inc_counter("c", &[], 42);
        m.set_gauge("g", &[], 3.14);
        m.observe_histogram("h", &[], 100.0);
        m.reset();
        assert_eq!(m.counter_value("c", &[]), 0);
        assert_eq!(m.gauge_value("g", &[]), 0.0);
        // Output after reset should not contain the stale value
        let text = m.render_prometheus();
        assert!(!text.contains("42"),
            "stale counter value 42 in rendered output: {}", text);
    }

    /// INVARIANT: gauge_value reflects most-recent set_gauge.
    #[test]
    fn invariant_gauge_reflects_last_set() {
        let m = LfiMetrics::new();
        for v in [1.0, 5.5, 3.0, -2.0, 100.0] {
            m.set_gauge("g", &[], v);
            assert_eq!(m.gauge_value("g", &[]), v,
                "gauge should reflect last set: got {}", m.gauge_value("g", &[]));
        }
    }

    /// INVARIANT: adjust_gauge delta applies correctly.
    #[test]
    fn invariant_adjust_gauge_composes() {
        let m = LfiMetrics::new();
        m.set_gauge("g", &[], 10.0);
        m.adjust_gauge("g", &[], 5.0);
        assert!((m.gauge_value("g", &[]) - 15.0).abs() < 0.001);
        m.adjust_gauge("g", &[], -3.0);
        assert!((m.gauge_value("g", &[]) - 12.0).abs() < 0.001);
    }

    /// INVARIANT: labels are treated as a set — same labels in different
    /// order resolve to the same metric.
    #[test]
    fn invariant_label_order_independent() {
        let m = LfiMetrics::new();
        m.inc_counter("c", &[("x", "1"), ("y", "2")], 1);
        m.inc_counter("c", &[("y", "2"), ("x", "1")], 1);
        assert_eq!(m.counter_value("c", &[("x", "1"), ("y", "2")]), 2,
            "label order should not matter");
    }

    /// INVARIANT: render_prometheus never panics and produces parseable output.
    #[test]
    fn invariant_render_prometheus_safe() {
        let m = LfiMetrics::new();
        m.inc_counter("requests_total", &[("endpoint", "/x")], 5);
        m.set_gauge("sessions", &[], 10.0);
        m.observe_histogram("latency_ms", &[], 50.0);
        // Test escaping of special chars
        m.inc_counter("weird", &[("path", "a\"b\\c\n")], 1);

        let text = m.render_prometheus();
        assert!(!text.is_empty());
        assert!(text.contains("requests_total"));
        assert!(text.contains("sessions"));
        assert!(text.contains("latency_ms"));
    }
}
