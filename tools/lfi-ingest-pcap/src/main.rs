//! # lfi-ingest-pcap — 802.11 Frame-to-Fact Converter
//!
//! Reads pcap files from pineapple-capture sessions, extracts structured
//! frame metadata, labels with adversary tier from session identity,
//! pseudonymizes ambient device MACs, and writes facts to brain.db.
//!
//! SECURITY: Ambient MACs (not on test-device allowlist) are automatically
//! pseudonymized with SHA-256(MAC + session_salt). Real ambient MACs never
//! enter the fact store.

use clap::Parser;
use sha2::{Sha256, Digest};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "lfi-ingest-pcap", version, about)]
struct Args {
    /// Session UUID
    #[arg(short, long)]
    session_id: String,

    /// Session output directory
    #[arg(long, default_value = "~/lfi/sessions")]
    output_dir: String,

    /// Path to test device registry (JSON with known MACs)
    #[arg(long, default_value = "~/lfi/sessions/devices.json")]
    devices_file: String,

    /// Database path
    #[arg(long, default_value = "~/.local/share/plausiden/brain.db")]
    db_path: String,

    /// Dry run — parse and report but don't insert
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

/// Pcap global header (24 bytes)
#[derive(Debug)]
struct PcapHeader {
    magic: u32,
    version_major: u16,
    version_minor: u16,
    snaplen: u32,
    link_type: u32,
}

/// Pcap packet header (16 bytes)
#[derive(Debug)]
struct PacketHeader {
    ts_sec: u32,
    ts_usec: u32,
    incl_len: u32,
    orig_len: u32,
}

/// Extracted 802.11 frame metadata
#[derive(Debug, serde::Serialize)]
struct FrameFact {
    session_id: String,
    frame_index: u64,
    timestamp_unix: f64,
    frame_type: String,
    frame_subtype: String,
    src_mac: String,
    dst_mac: String,
    bssid: String,
    ssid: Option<String>,
    channel: Option<u8>,
    rssi_dbm: Option<i8>,
    adversary_tier: u8,
    lab_environment: String,
    is_test_device: bool,
}

fn expand_path(p: &str) -> PathBuf {
    PathBuf::from(p.replace("~", &std::env::var("HOME").unwrap_or("/root".into())))
}

/// Pseudonymize a MAC address with session-scoped salt.
/// BUG ASSUMPTION: SHA-256 truncated to 6 bytes could collide; acceptable
/// for training data where uniqueness isn't critical.
fn pseudonymize_mac(mac: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(mac.as_bytes());
    hasher.update(salt.as_bytes());
    let hash = hasher.finalize();
    format!("XX:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        hash[0], hash[1], hash[2], hash[3], hash[4])
}

/// Parse 802.11 frame type/subtype from the first two bytes of the frame.
fn parse_frame_type(fc: u16) -> (String, String) {
    let frame_type = (fc >> 2) & 0x3;
    let subtype = (fc >> 4) & 0xF;

    let type_str = match frame_type {
        0 => "management",
        1 => "control",
        2 => "data",
        _ => "unknown",
    };

    let subtype_str = match (frame_type, subtype) {
        (0, 0) => "association_req",
        (0, 1) => "association_resp",
        (0, 4) => "probe_req",
        (0, 5) => "probe_resp",
        (0, 8) => "beacon",
        (0, 10) => "disassociation",
        (0, 11) => "authentication",
        (0, 12) => "deauthentication",
        (0, 13) => "action",
        (1, 11) => "rts",
        (1, 12) => "cts",
        (1, 13) => "ack",
        (2, 0) => "data",
        (2, 4) => "null_data",
        (2, 8) => "qos_data",
        _ => "other",
    };

    (type_str.to_string(), subtype_str.to_string())
}

/// Extract MAC address from 6 bytes.
fn mac_from_bytes(bytes: &[u8]) -> String {
    if bytes.len() < 6 { return "??:??:??:??:??:??".into(); }
    format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5])
}

/// Extract SSID from beacon/probe response tagged parameters.
fn extract_ssid(frame_body: &[u8]) -> Option<String> {
    // After the fixed parameters (12 bytes for beacon, 0 for probe req),
    // tagged parameters start. Tag 0 = SSID.
    let mut offset = 0;
    while offset + 2 <= frame_body.len() {
        let tag_id = frame_body[offset];
        let tag_len = frame_body[offset + 1] as usize;
        if offset + 2 + tag_len > frame_body.len() { break; }
        if tag_id == 0 && tag_len > 0 {
            let ssid_bytes = &frame_body[offset + 2..offset + 2 + tag_len];
            return Some(String::from_utf8_lossy(ssid_bytes).to_string());
        }
        offset += 2 + tag_len;
    }
    None
}

/// Parse a pcap file and extract frame facts.
fn parse_pcap(data: &[u8], session_id: &str, tier: u8, test_macs: &HashSet<String>, salt: &str) -> Vec<FrameFact> {
    let mut facts = Vec::new();

    if data.len() < 24 {
        eprintln!("Pcap too short for header");
        return facts;
    }

    // Parse global header
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let _snaplen = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    let link_type = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);

    let is_le = magic == 0xa1b2c3d4;
    if !is_le && magic != 0xd4c3b2a1 {
        eprintln!("Not a valid pcap file (magic: {:08x})", magic);
        return facts;
    }

    // Radiotap header is link_type 127
    let has_radiotap = link_type == 127;

    let mut offset = 24; // After global header
    let mut frame_idx: u64 = 0;

    while offset + 16 <= data.len() {
        // Parse packet header
        let ts_sec = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
        let ts_usec = u32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
        let incl_len = u32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]) as usize;
        offset += 16;

        if offset + incl_len > data.len() { break; }
        let pkt_data = &data[offset..offset + incl_len];
        offset += incl_len;

        // Skip radiotap header if present
        let mut rssi: Option<i8> = None;
        let ieee80211_start = if has_radiotap && pkt_data.len() >= 4 {
            let rt_len = u16::from_le_bytes([pkt_data[2], pkt_data[3]]) as usize;
            // Try to extract RSSI from radiotap (simplified — field at offset 14 in many configs)
            if rt_len > 14 && pkt_data.len() > 14 {
                rssi = Some(pkt_data[14] as i8); // Approximate — real parsing needs present flags
            }
            rt_len
        } else {
            0
        };

        if ieee80211_start >= pkt_data.len() || pkt_data.len() - ieee80211_start < 24 { continue; }
        let frame = &pkt_data[ieee80211_start..];

        // Parse 802.11 header
        let fc = u16::from_le_bytes([frame[0], frame[1]]);
        let (frame_type, frame_subtype) = parse_frame_type(fc);

        // Addresses: addr1 = receiver, addr2 = transmitter, addr3 = BSSID (for mgmt frames)
        let dst_mac = mac_from_bytes(&frame[4..10]);
        let src_mac = mac_from_bytes(&frame[10..16]);
        let bssid = mac_from_bytes(&frame[16..22]);

        // Extract SSID from beacon/probe frames
        let ssid = if frame_subtype == "beacon" || frame_subtype == "probe_resp" {
            if frame.len() > 36 { // Fixed params are 12 bytes after 24-byte header
                extract_ssid(&frame[36..])
            } else { None }
        } else if frame_subtype == "probe_req" {
            if frame.len() > 24 {
                extract_ssid(&frame[24..])
            } else { None }
        } else { None };

        // Pseudonymize ambient MACs
        let src_display = if test_macs.contains(&src_mac.to_uppercase()) {
            src_mac.clone()
        } else {
            pseudonymize_mac(&src_mac, salt)
        };
        let dst_display = if test_macs.contains(&dst_mac.to_uppercase()) || dst_mac == "FF:FF:FF:FF:FF:FF" {
            dst_mac.clone()
        } else {
            pseudonymize_mac(&dst_mac, salt)
        };
        let bssid_display = if test_macs.contains(&bssid.to_uppercase()) || bssid == "FF:FF:FF:FF:FF:FF" {
            bssid.clone()
        } else {
            pseudonymize_mac(&bssid, salt)
        };

        let is_test = test_macs.contains(&src_mac.to_uppercase()) || test_macs.contains(&dst_mac.to_uppercase());

        facts.push(FrameFact {
            session_id: session_id.to_string(),
            frame_index: frame_idx,
            timestamp_unix: ts_sec as f64 + ts_usec as f64 / 1_000_000.0,
            frame_type,
            frame_subtype,
            src_mac: src_display,
            dst_mac: dst_display,
            bssid: bssid_display,
            ssid,
            channel: None, // Would need to parse radiotap channel field
            rssi_dbm: rssi,
            adversary_tier: tier,
            lab_environment: "controlled_home".to_string(),
            is_test_device: is_test,
        });

        frame_idx += 1;
    }

    facts
}

fn main() {
    let args = Args::parse();
    let session_dir = expand_path(&args.output_dir).join(&args.session_id);
    let db_path = expand_path(&args.db_path);

    println!("=== LFI Pcap Ingest ===");
    println!("Session: {}", args.session_id);

    // Load identity
    let identity_path = session_dir.join("adversary_identity.json");
    let identity: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&identity_path).expect("No adversary_identity.json")
    ).expect("Invalid identity JSON");
    let tier = identity["tier"].as_u64().unwrap_or(0) as u8;
    println!("Tier:    {}", tier);

    // Load test device registry
    let devices_path = expand_path(&args.devices_file);
    let test_macs: HashSet<String> = if devices_path.exists() {
        let devs: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&devices_path).unwrap_or("[]".into())
        ).unwrap_or(serde_json::json!([]));
        devs.as_array().map(|arr| {
            arr.iter().filter_map(|d| d["mac"].as_str().map(|m| m.to_uppercase())).collect()
        }).unwrap_or_default()
    } else {
        println!("Warning: No devices.json — all MACs will be pseudonymized");
        HashSet::new()
    };

    // Session salt for pseudonymization
    let salt = format!("{}:{}", args.session_id, identity["timestamp"].as_str().unwrap_or(""));

    // Find pcap files
    let pcap_files: Vec<PathBuf> = std::fs::read_dir(&session_dir)
        .expect("Can't read session directory")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "pcap").unwrap_or(false))
        .collect();

    if pcap_files.is_empty() {
        eprintln!("No pcap files found in {}", session_dir.display());
        std::process::exit(1);
    }

    let mut total_facts = 0;
    let mut all_facts = Vec::new();

    for pcap_path in &pcap_files {
        println!("Parsing: {}", pcap_path.display());
        let data = std::fs::read(pcap_path).expect("Failed to read pcap");
        let facts = parse_pcap(&data, &args.session_id, tier, &test_macs, &salt);
        println!("  {} frames extracted", facts.len());
        total_facts += facts.len();
        all_facts.extend(facts);
    }

    println!("\nTotal frames: {}", total_facts);

    if args.dry_run {
        println!("DRY RUN — not inserting. Sample facts:");
        for f in all_facts.iter().take(5) {
            println!("  {} {} {} src={} bssid={} ssid={:?} rssi={:?}",
                f.frame_index, f.frame_type, f.frame_subtype,
                f.src_mac, f.bssid, f.ssid, f.rssi_dbm);
        }
        return;
    }

    // Insert into brain.db
    println!("Inserting into brain.db...");
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open brain.db");
    conn.execute_batch("PRAGMA busy_timeout=600000; PRAGMA journal_mode=WAL;").ok();

    let mut inserted = 0;
    for fact in &all_facts {
        let key = format!("wifi:{}:{}:{}", fact.session_id, fact.frame_index, fact.frame_subtype);
        let value = format!(
            "{} frame: src={} dst={} bssid={}{}{} tier={} lab={}",
            fact.frame_subtype, fact.src_mac, fact.dst_mac, fact.bssid,
            fact.ssid.as_ref().map(|s| format!(" ssid={}", s)).unwrap_or_default(),
            fact.rssi_dbm.map(|r| format!(" rssi={}dBm", r)).unwrap_or_default(),
            fact.adversary_tier, fact.lab_environment,
        );
        let quality = if fact.is_test_device { 0.90 } else { 0.85 };

        if conn.execute(
            "INSERT OR IGNORE INTO facts (key, value, confidence, source, domain, quality_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![key, value, quality, format!("pineapple_tier{}", tier), "wireless_security", quality],
        ).is_ok() {
            inserted += 1;
        }
    }

    // Write session summary
    let summary = serde_json::json!({
        "session_id": args.session_id,
        "tier": tier,
        "pcap_files": pcap_files.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "total_frames": total_facts,
        "facts_inserted": inserted,
        "test_devices_seen": all_facts.iter().filter(|f| f.is_test_device).count(),
        "ambient_devices_pseudonymized": all_facts.iter().filter(|f| !f.is_test_device).count(),
        "frame_type_distribution": serde_json::to_value({
            let mut dist = std::collections::HashMap::new();
            for f in &all_facts { *dist.entry(f.frame_subtype.clone()).or_insert(0u64) += 1; }
            dist
        }).unwrap_or_default(),
    });
    std::fs::write(
        session_dir.join("ingest_summary.json"),
        serde_json::to_string_pretty(&summary).unwrap()
    ).ok();

    println!("Inserted: {} facts", inserted);
    println!("Summary:  {}", session_dir.join("ingest_summary.json").display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pseudonymize_mac_deterministic() {
        let mac = "AA:BB:CC:DD:EE:FF";
        let salt = "test-session";
        let p1 = pseudonymize_mac(mac, salt);
        let p2 = pseudonymize_mac(mac, salt);
        assert_eq!(p1, p2, "Same input must produce same output");
        assert!(p1.starts_with("XX:"), "Pseudonymized MAC should start with XX:");
    }

    #[test]
    fn test_pseudonymize_different_salt() {
        let mac = "AA:BB:CC:DD:EE:FF";
        let p1 = pseudonymize_mac(mac, "session1");
        let p2 = pseudonymize_mac(mac, "session2");
        assert_ne!(p1, p2, "Different salts must produce different outputs");
    }

    #[test]
    fn test_parse_frame_type() {
        let (t, s) = parse_frame_type(0x0080); // Beacon
        assert_eq!(t, "management");
        assert_eq!(s, "beacon");
    }

    #[test]
    fn test_mac_from_bytes() {
        let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        assert_eq!(mac_from_bytes(&bytes), "AA:BB:CC:DD:EE:FF");
    }

    #[test]
    fn test_mac_from_short_bytes() {
        let bytes = [0xAA, 0xBB];
        assert_eq!(mac_from_bytes(&bytes), "??:??:??:??:??:??");
    }
}
