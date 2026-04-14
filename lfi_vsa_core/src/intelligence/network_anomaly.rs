// ============================================================
// Network Anomaly Detection — Stateless Pattern-Based Defense
//
// PURPOSE: Detect network-layer threats from observation records.
// Intended to complement defensive_ai for full-stack sovereign defense.
//
// SCOPE:
//   Given observations (connection attempts, DNS queries, port scans,
//   TLS handshakes), classify suspicious patterns:
//     - Unexpected outbound to non-allowlisted destinations (exfil)
//     - DNS tunneling (long subdomains, high query rate to single domain)
//     - Port scanning (many distinct destination ports from one source)
//     - TLS downgrade or invalid certificate patterns
//     - Beacon-like regular timing (C2 infrastructure)
//     - Domain age anomalies (newly-registered domains)
//
// DEPLOYMENT:
//   - Integrate with tcpdump / DNS logs / firewall events
//   - Feed observations into analyzer
//   - Emit threat scores to SIEM / alerting
//
// BUG ASSUMPTION:
//   We do NOT implement packet capture. The caller provides observations.
//   We don't ship a PCAP parser — that's specialized infrastructure.
// ============================================================

use std::collections::HashMap;

// ============================================================
// Network Observations
// ============================================================

#[derive(Debug, Clone)]
pub struct ConnectionAttempt {
    pub source_ip: String,
    pub dest_ip: String,
    pub dest_port: u16,
    pub timestamp_ms: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub tls_version: Option<String>, // e.g., "TLS 1.3", "TLS 1.0"
    pub cert_valid: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct DnsQuery {
    pub source_ip: String,
    pub domain: String,
    pub query_type: String, // A, AAAA, TXT, etc.
    pub timestamp_ms: u64,
}

// ============================================================
// Threat Types
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkThreatKind {
    /// Outbound to non-allowlisted destination.
    UnexpectedExfiltration { dest: String },
    /// DNS tunneling indicators.
    DnsTunneling { domain: String, reason: String },
    /// Port scanning from source.
    PortScan { source: String, port_count: usize },
    /// TLS downgrade (using TLS 1.0 or 1.1).
    TlsDowngrade { version: String },
    /// Invalid certificate.
    InvalidCertificate { dest: String },
    /// Beacon-like regular timing.
    BeaconingTraffic { dest: String, interval_ms: u64 },
    /// Unusually large outbound transfer.
    LargeDataTransfer { dest: String, bytes: u64 },
    /// Connection to known-bad destination.
    KnownBadDestination { dest: String },
}

#[derive(Debug, Clone)]
pub struct NetworkThreat {
    pub kind: NetworkThreatKind,
    pub severity: NetworkSeverity,
    pub confidence: f64,
    pub first_seen_ms: u64,
    pub mitigation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================
// DNS Tunneling Detector
// ============================================================

pub struct DnsTunnelingDetector;

impl DnsTunnelingDetector {
    /// Detect long subdomains (common in DNS tunneling data encoding).
    pub fn check_subdomain_length(domain: &str) -> Option<NetworkThreat> {
        // Extract leftmost label (subdomain)
        let label = domain.split('.').next()?;
        if label.len() > 50 {
            return Some(NetworkThreat {
                kind: NetworkThreatKind::DnsTunneling {
                    domain: domain.into(),
                    reason: format!("subdomain label length {} (> 50)", label.len()),
                },
                severity: NetworkSeverity::High,
                confidence: (label.len() as f64 / 100.0).min(0.95),
                first_seen_ms: 0,
                mitigation: "Investigate subdomain pattern. Consider blocking domain if unknown.".into(),
            });
        }
        None
    }

    /// Detect high query rate to a single domain.
    pub fn check_query_rate(queries: &[DnsQuery]) -> Vec<NetworkThreat> {
        let mut by_domain: HashMap<String, Vec<&DnsQuery>> = HashMap::new();
        for q in queries {
            let root = Self::root_domain(&q.domain);
            by_domain.entry(root).or_insert_with(Vec::new).push(q);
        }

        let mut threats = Vec::new();
        for (domain, group) in by_domain {
            if group.len() < 20 { continue; }
            // Check if queries span less than 60 seconds (high rate).
            let first = group.first().map(|q| q.timestamp_ms).unwrap_or(0);
            let last = group.last().map(|q| q.timestamp_ms).unwrap_or(0);
            let span_sec = (last - first) as f64 / 1000.0;
            if span_sec > 0.0 && (group.len() as f64 / span_sec) > 10.0 {
                threats.push(NetworkThreat {
                    kind: NetworkThreatKind::DnsTunneling {
                        domain: domain.clone(),
                        reason: format!("{} queries in {:.1}s = {:.1}/sec",
                            group.len(), span_sec,
                            group.len() as f64 / span_sec),
                    },
                    severity: NetworkSeverity::High,
                    confidence: 0.85,
                    first_seen_ms: first,
                    mitigation: format!("Block domain '{}' pending investigation.", domain),
                });
            }
        }
        threats
    }

    fn root_domain(domain: &str) -> String {
        let parts: Vec<&str> = domain.split('.').collect();
        if parts.len() >= 2 {
            format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
        } else {
            domain.to_string()
        }
    }
}

// ============================================================
// Port Scan Detector
// ============================================================

pub struct PortScanDetector;

impl PortScanDetector {
    /// Detect port scanning: many distinct destination ports from one source.
    pub fn analyze(connections: &[ConnectionAttempt]) -> Vec<NetworkThreat> {
        let mut by_source: HashMap<String, HashMap<String, std::collections::HashSet<u16>>> =
            HashMap::new();
        for c in connections {
            by_source.entry(c.source_ip.clone())
                .or_insert_with(HashMap::new)
                .entry(c.dest_ip.clone())
                .or_insert_with(std::collections::HashSet::new)
                .insert(c.dest_port);
        }

        let mut threats = Vec::new();
        for (source, dests) in by_source {
            for (dest, ports) in dests {
                if ports.len() >= 10 {
                    let severity = match ports.len() {
                        n if n >= 100 => NetworkSeverity::Critical,
                        n if n >= 50 => NetworkSeverity::High,
                        _ => NetworkSeverity::Medium,
                    };
                    threats.push(NetworkThreat {
                        kind: NetworkThreatKind::PortScan {
                            source: format!("{}→{}", source, dest),
                            port_count: ports.len(),
                        },
                        severity,
                        confidence: (ports.len() as f64 / 100.0).min(0.95),
                        first_seen_ms: 0,
                        mitigation: format!(
                            "Block source {} at firewall. Investigate destination {}.",
                            source, dest,
                        ),
                    });
                }
            }
        }
        threats
    }
}

// ============================================================
// Exfiltration Detector
// ============================================================

pub struct ExfiltrationDetector;

impl ExfiltrationDetector {
    /// Detect unexpected large outbound transfers or unallowlisted destinations.
    pub fn analyze(
        connections: &[ConnectionAttempt],
        allowlist: &[String],
    ) -> Vec<NetworkThreat> {
        let mut threats = Vec::new();
        for c in connections {
            // Large transfer check (> 100 MB outbound)
            if c.bytes_sent > 100 * 1024 * 1024 {
                threats.push(NetworkThreat {
                    kind: NetworkThreatKind::LargeDataTransfer {
                        dest: c.dest_ip.clone(),
                        bytes: c.bytes_sent,
                    },
                    severity: if c.bytes_sent > 1024 * 1024 * 1024 {
                        NetworkSeverity::Critical
                    } else {
                        NetworkSeverity::High
                    },
                    confidence: 0.8,
                    first_seen_ms: c.timestamp_ms,
                    mitigation: format!(
                        "{} MB sent to {}. Investigate for exfiltration.",
                        c.bytes_sent / (1024 * 1024), c.dest_ip,
                    ),
                });
            }

            // Non-allowlisted destination
            if !allowlist.is_empty() && !allowlist.iter().any(|a| c.dest_ip.contains(a)) {
                // Skip if it's a local/private address
                if !Self::is_local(&c.dest_ip) {
                    threats.push(NetworkThreat {
                        kind: NetworkThreatKind::UnexpectedExfiltration {
                            dest: c.dest_ip.clone(),
                        },
                        severity: NetworkSeverity::Medium,
                        confidence: 0.6,
                        first_seen_ms: c.timestamp_ms,
                        mitigation: format!(
                            "Unexpected outbound to {}. Add to allowlist if legitimate.",
                            c.dest_ip,
                        ),
                    });
                }
            }
        }
        threats
    }

    fn is_local(ip: &str) -> bool {
        ip.starts_with("10.") || ip.starts_with("192.168.")
            || ip.starts_with("172.16.") || ip.starts_with("172.17.")
            || ip.starts_with("172.18.") || ip.starts_with("172.19.")
            || ip.starts_with("172.2") || ip.starts_with("172.3")
            || ip.starts_with("127.") || ip == "::1"
    }
}

// ============================================================
// TLS Anomaly Detector
// ============================================================

pub struct TlsAnomalyDetector;

impl TlsAnomalyDetector {
    pub fn analyze(connections: &[ConnectionAttempt]) -> Vec<NetworkThreat> {
        let mut threats = Vec::new();
        for c in connections {
            // Deprecated TLS
            if let Some(ref v) = c.tls_version {
                if v.contains("1.0") || v.contains("1.1") || v == "SSLv3" {
                    threats.push(NetworkThreat {
                        kind: NetworkThreatKind::TlsDowngrade { version: v.clone() },
                        severity: NetworkSeverity::High,
                        confidence: 0.95,
                        first_seen_ms: c.timestamp_ms,
                        mitigation: format!(
                            "Connection to {} using deprecated {}. Block or investigate.",
                            c.dest_ip, v,
                        ),
                    });
                }
            }

            // Invalid certificate
            if c.cert_valid == Some(false) {
                threats.push(NetworkThreat {
                    kind: NetworkThreatKind::InvalidCertificate {
                        dest: c.dest_ip.clone(),
                    },
                    severity: NetworkSeverity::High,
                    confidence: 0.9,
                    first_seen_ms: c.timestamp_ms,
                    mitigation: format!(
                        "Invalid certificate from {}. Possible MITM — do not trust.",
                        c.dest_ip,
                    ),
                });
            }
        }
        threats
    }
}

// ============================================================
// Beacon Detector
// ============================================================

pub struct BeaconDetector;

impl BeaconDetector {
    /// Detect C2-style beaconing: regular intervals to the same destination.
    pub fn analyze(connections: &[ConnectionAttempt]) -> Vec<NetworkThreat> {
        let mut by_dest: HashMap<String, Vec<u64>> = HashMap::new();
        for c in connections {
            by_dest.entry(c.dest_ip.clone())
                .or_insert_with(Vec::new)
                .push(c.timestamp_ms);
        }

        let mut threats = Vec::new();
        for (dest, mut times) in by_dest {
            if times.len() < 10 { continue; }
            times.sort();
            let intervals: Vec<u64> = times.windows(2)
                .map(|w| w[1] - w[0])
                .collect();
            if intervals.is_empty() { continue; }
            let mean = intervals.iter().sum::<u64>() as f64 / intervals.len() as f64;
            let variance: f64 = intervals.iter()
                .map(|&i| (i as f64 - mean).powi(2))
                .sum::<f64>() / intervals.len() as f64;
            let std_dev = variance.sqrt();

            // Regular beacon: low coefficient of variation
            if mean > 0.0 && std_dev / mean < 0.1 && mean > 1000.0 {
                threats.push(NetworkThreat {
                    kind: NetworkThreatKind::BeaconingTraffic {
                        dest: dest.clone(),
                        interval_ms: mean as u64,
                    },
                    severity: NetworkSeverity::High,
                    confidence: 0.85,
                    first_seen_ms: times[0],
                    mitigation: format!(
                        "Regular beaconing to {} at {}ms intervals. Possible C2 — investigate.",
                        dest, mean as u64,
                    ),
                });
            }
        }
        threats
    }
}

// ============================================================
// Unified Analyzer
// ============================================================

pub struct NetworkAnomalyAnalyzer {
    pub allowlist: Vec<String>,
    pub bad_destinations: Vec<String>,
}

impl NetworkAnomalyAnalyzer {
    pub fn new() -> Self {
        Self {
            allowlist: Vec::new(),
            bad_destinations: Vec::new(),
        }
    }

    pub fn with_allowlist(allowlist: Vec<String>) -> Self {
        Self {
            allowlist,
            bad_destinations: Vec::new(),
        }
    }

    pub fn with_bad_destinations(mut self, bad: Vec<String>) -> Self {
        self.bad_destinations = bad;
        self
    }

    pub fn analyze_connections(&self, connections: &[ConnectionAttempt]) -> Vec<NetworkThreat> {
        let mut threats = Vec::new();
        threats.extend(PortScanDetector::analyze(connections));
        threats.extend(ExfiltrationDetector::analyze(connections, &self.allowlist));
        threats.extend(TlsAnomalyDetector::analyze(connections));
        threats.extend(BeaconDetector::analyze(connections));

        // Known-bad destinations
        for c in connections {
            if self.bad_destinations.iter().any(|b| c.dest_ip.contains(b)) {
                threats.push(NetworkThreat {
                    kind: NetworkThreatKind::KnownBadDestination {
                        dest: c.dest_ip.clone(),
                    },
                    severity: NetworkSeverity::Critical,
                    confidence: 0.99,
                    first_seen_ms: c.timestamp_ms,
                    mitigation: format!(
                        "Connection to known-malicious {}. Block immediately.",
                        c.dest_ip,
                    ),
                });
            }
        }

        threats
    }

    pub fn analyze_dns(&self, queries: &[DnsQuery]) -> Vec<NetworkThreat> {
        let mut threats = Vec::new();
        for q in queries {
            if let Some(t) = DnsTunnelingDetector::check_subdomain_length(&q.domain) {
                threats.push(t);
            }
        }
        threats.extend(DnsTunnelingDetector::check_query_rate(queries));
        threats
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(src: &str, dst: &str, port: u16, ts: u64) -> ConnectionAttempt {
        ConnectionAttempt {
            source_ip: src.into(),
            dest_ip: dst.into(),
            dest_port: port,
            timestamp_ms: ts,
            bytes_sent: 100,
            bytes_received: 100,
            tls_version: Some("TLS 1.3".into()),
            cert_valid: Some(true),
        }
    }

    #[test]
    fn test_port_scan_detected() {
        let conns: Vec<ConnectionAttempt> = (1..60)
            .map(|port| conn("1.2.3.4", "10.0.0.5", port, 1000))
            .collect();
        let threats = PortScanDetector::analyze(&conns);
        assert!(!threats.is_empty());
        assert!(threats.iter().any(|t| matches!(&t.kind, NetworkThreatKind::PortScan { port_count, .. } if *port_count >= 50)));
    }

    #[test]
    fn test_normal_traffic_no_portscan() {
        let conns = vec![
            conn("1.2.3.4", "10.0.0.5", 80, 1000),
            conn("1.2.3.4", "10.0.0.5", 443, 1000),
            conn("1.2.3.4", "10.0.0.5", 22, 1000),
        ];
        let threats = PortScanDetector::analyze(&conns);
        assert!(threats.is_empty(), "Three ports shouldn't be a scan");
    }

    #[test]
    fn test_large_transfer_detected() {
        let mut c = conn("10.0.0.1", "evil.com", 443, 1000);
        c.bytes_sent = 500 * 1024 * 1024; // 500 MB
        let threats = ExfiltrationDetector::analyze(&[c], &[]);
        assert!(threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::LargeDataTransfer { .. })));
    }

    #[test]
    fn test_unexpected_destination_flagged() {
        let c = conn("10.0.0.1", "1.2.3.4", 443, 1000);
        let threats = ExfiltrationDetector::analyze(&[c], &vec!["api.example.com".into()]);
        assert!(threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::UnexpectedExfiltration { .. })));
    }

    #[test]
    fn test_local_destination_not_flagged() {
        let c = conn("10.0.0.1", "10.0.0.5", 443, 1000);
        let threats = ExfiltrationDetector::analyze(&[c], &vec!["example.com".into()]);
        assert!(!threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::UnexpectedExfiltration { .. })));
    }

    #[test]
    fn test_tls_downgrade_detected() {
        let mut c = conn("10.0.0.1", "example.com", 443, 1000);
        c.tls_version = Some("TLS 1.0".into());
        let threats = TlsAnomalyDetector::analyze(&[c]);
        assert!(threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::TlsDowngrade { .. })));
    }

    #[test]
    fn test_invalid_cert_detected() {
        let mut c = conn("10.0.0.1", "example.com", 443, 1000);
        c.cert_valid = Some(false);
        let threats = TlsAnomalyDetector::analyze(&[c]);
        assert!(threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::InvalidCertificate { .. })));
    }

    #[test]
    fn test_beacon_detected() {
        // Regular interval beacons
        let conns: Vec<ConnectionAttempt> = (0..20)
            .map(|i| conn("10.0.0.1", "c2.example.com", 443, 1000 + i * 60_000))
            .collect();
        let threats = BeaconDetector::analyze(&conns);
        assert!(threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::BeaconingTraffic { .. })));
    }

    #[test]
    fn test_irregular_traffic_not_beacon() {
        // Highly variable intervals
        let times = [0u64, 1000, 50_000, 51_000, 200_000, 205_000, 500_000, 505_000, 1_000_000, 1_005_000, 2_000_000, 3_200_000];
        let conns: Vec<ConnectionAttempt> = times.iter()
            .map(|&ts| conn("10.0.0.1", "random.com", 443, ts))
            .collect();
        let threats = BeaconDetector::analyze(&conns);
        assert!(!threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::BeaconingTraffic { .. })),
            "Irregular traffic shouldn't be flagged as beacon");
    }

    #[test]
    fn test_long_subdomain_flagged() {
        let long_sub = "a".repeat(80);
        let domain = format!("{}.example.com", long_sub);
        let result = DnsTunnelingDetector::check_subdomain_length(&domain);
        assert!(result.is_some(), "Long subdomain should be flagged");
    }

    #[test]
    fn test_normal_subdomain_not_flagged() {
        let result = DnsTunnelingDetector::check_subdomain_length("mail.example.com");
        assert!(result.is_none());
    }

    #[test]
    fn test_high_rate_dns_flagged() {
        let queries: Vec<DnsQuery> = (0..50)
            .map(|i| DnsQuery {
                source_ip: "10.0.0.1".into(),
                domain: format!("sub{}.tunnel.example.com", i),
                query_type: "A".into(),
                timestamp_ms: 1000 + i * 100, // 10/sec
            })
            .collect();
        let threats = DnsTunnelingDetector::check_query_rate(&queries);
        assert!(!threats.is_empty());
    }

    #[test]
    fn test_known_bad_destination_critical() {
        let c = conn("10.0.0.1", "evil.onion.example", 443, 1000);
        let analyzer = NetworkAnomalyAnalyzer::new()
            .with_bad_destinations(vec!["evil.onion".into()]);
        let threats = analyzer.analyze_connections(&[c]);
        assert!(threats.iter().any(|t|
            matches!(&t.kind, NetworkThreatKind::KnownBadDestination { .. }) &&
            t.severity == NetworkSeverity::Critical));
    }

    #[test]
    fn test_full_analyzer_integration() {
        let analyzer = NetworkAnomalyAnalyzer::with_allowlist(
            vec!["api.example.com".into()]
        );

        let conns = vec![
            conn("1.2.3.4", "unknown-destination.com", 443, 1000),
            ConnectionAttempt {
                source_ip: "10.0.0.1".into(),
                dest_ip: "evil.com".into(),
                dest_port: 443,
                timestamp_ms: 1000,
                bytes_sent: 600 * 1024 * 1024,
                bytes_received: 0,
                tls_version: Some("TLS 1.3".into()),
                cert_valid: Some(true),
            },
        ];

        let threats = analyzer.analyze_connections(&conns);
        assert!(!threats.is_empty());
    }

    #[test]
    fn test_severity_ordering() {
        use NetworkSeverity::*;
        assert!(Critical > High);
        assert!(High > Medium);
        assert!(Medium > Low);
        assert!(Low > Info);
    }

    #[test]
    fn test_mitigation_always_present() {
        let mut c = conn("10.0.0.1", "example.com", 443, 1000);
        c.tls_version = Some("TLS 1.0".into());
        let threats = TlsAnomalyDetector::analyze(&[c]);
        for t in &threats {
            assert!(!t.mitigation.is_empty());
        }
    }
}
