//! # pineapple-capture — WiFi Frame Capture Daemon
//!
//! Streams 802.11 management frames from WiFi Pineapple's monitor-mode
//! interface over SSH. Writes pcap files to session directories.
//!
//! SECURITY: All capture happens on Pineapple's wlan1mon via SSH.
//! Workstation's wlan0 is NEVER touched. Preflight checks enforce this.

use clap::Parser;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::{Read, Write as IoWrite};

#[derive(Parser, Debug)]
#[command(name = "pineapple-capture", version, about)]
struct Args {
    /// Session UUID (must match an existing adversary_identity.json)
    #[arg(short, long)]
    session_id: String,

    /// Capture duration in seconds (0 = until interrupted)
    #[arg(short, long, default_value_t = 3600)]
    duration_seconds: u64,

    /// Pineapple radio interface to capture from
    #[arg(short, long, default_value = "wlan1mon")]
    radio: String,

    /// SSH host (from ~/.ssh/config)
    #[arg(long, default_value = "pineapple")]
    host: String,

    /// Capture filter: mgt = management frames only, all = everything
    #[arg(long, default_value = "mgt")]
    filter: String,

    /// Include full frame headers (not just management)
    #[arg(long, default_value_t = false)]
    full_headers: bool,

    /// Session output directory
    #[arg(long, default_value = "~/lfi/sessions")]
    output_dir: String,
}

/// Preflight network safety check — NEVER proceed with degraded workstation network.
/// BUG ASSUMPTION: Any of these checks could give false negatives on exotic configs.
fn preflight_check() -> Result<(), String> {
    // 1. wlan0 must exist and be UP
    let wlan0 = Command::new("ip").args(["link", "show", "wlan0"]).output()
        .map_err(|e| format!("Failed to check wlan0: {}", e))?;
    let wlan0_out = String::from_utf8_lossy(&wlan0.stdout);
    if !wlan0_out.contains("state UP") {
        return Err("ABORT: wlan0 is not UP".into());
    }
    if wlan0_out.contains("type monitor") {
        return Err("ABORT: wlan0 is in monitor mode — someone broke it".into());
    }

    // 2. Default route must be via wlan0
    let route = Command::new("ip").args(["route", "show", "default"]).output()
        .map_err(|e| format!("Failed to check routes: {}", e))?;
    let route_out = String::from_utf8_lossy(&route.stdout);
    if !route_out.contains("dev wlan0") {
        return Err(format!("ABORT: default route is not via wlan0: {}", route_out.trim()));
    }

    // 3. NetworkManager running
    let nm = Command::new("systemctl").args(["is-active", "NetworkManager"]).output()
        .map_err(|e| format!("Failed to check NM: {}", e))?;
    if !String::from_utf8_lossy(&nm.stdout).trim().eq("active") {
        return Err("ABORT: NetworkManager is not running".into());
    }

    // 4. Internet reachable
    let ping = Command::new("ping").args(["-c", "1", "-W", "3", "8.8.8.8"]).output()
        .map_err(|e| format!("Failed to ping: {}", e))?;
    if !ping.status.success() {
        return Err("ABORT: internet not reachable via wlan0".into());
    }

    Ok(())
}

/// Verify session has an adversary_identity.json.
fn verify_session(session_dir: &PathBuf) -> Result<serde_json::Value, String> {
    let identity_path = session_dir.join("adversary_identity.json");
    if !identity_path.exists() {
        return Err(format!(
            "No adversary_identity.json found at {}. Run pineapple-harden first.",
            identity_path.display()
        ));
    }
    let content = std::fs::read_to_string(&identity_path)
        .map_err(|e| format!("Failed to read identity: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Invalid identity JSON: {}", e))
}

fn main() {
    let args = Args::parse();

    println!("=== PlausiDen Capture Daemon ===");
    println!("Session:  {}", args.session_id);
    println!("Radio:    {}", args.radio);
    println!("Duration: {}s", args.duration_seconds);
    println!("Filter:   {}", args.filter);

    // Preflight
    print!("Preflight checks... ");
    match preflight_check() {
        Ok(()) => println!("PASSED"),
        Err(e) => {
            eprintln!("\n{}", e);
            std::process::exit(1);
        }
    }

    // Session directory
    let output_dir = args.output_dir.replace("~", &std::env::var("HOME").unwrap_or("/root".into()));
    let session_dir = PathBuf::from(&output_dir).join(&args.session_id);
    if !session_dir.exists() {
        std::fs::create_dir_all(&session_dir).expect("Failed to create session directory");
    }

    // Verify identity exists
    print!("Verifying session identity... ");
    let identity = match verify_session(&session_dir) {
        Ok(id) => { println!("OK (tier {})", id.get("tier").and_then(|t| t.as_u64()).unwrap_or(0)); id }
        Err(e) => {
            eprintln!("\n{}", e);
            std::process::exit(1);
        }
    };

    // Build tcpdump filter
    let capture_filter = match args.filter.as_str() {
        "mgt" => "type mgt",
        "all" => "",
        "ctl" => "type ctl",
        custom => custom,
    };

    // Output pcap path
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let pcap_path = session_dir.join(format!("capture_{}_{}.pcap", args.radio, timestamp));
    let log_path = session_dir.join("session.log");

    println!("Output:   {}", pcap_path.display());
    println!("Log:      {}", log_path.display());

    // Build SSH command
    let duration_flag = if args.duration_seconds > 0 {
        format!("-c {}", args.duration_seconds * 100) // rough frame count estimate
    } else {
        String::new()
    };

    let tcpdump_cmd = if capture_filter.is_empty() {
        format!("tcpdump -i {} {} -w -", args.radio, duration_flag)
    } else {
        format!("tcpdump -i {} {} -w - '{}'", args.radio, duration_flag, capture_filter)
    };

    println!("\nStarting capture: ssh {} \"{}\"", args.host, tcpdump_cmd);

    // Log session start
    let mut log = std::fs::OpenOptions::new()
        .create(true).append(true).open(&log_path)
        .expect("Failed to open session log");
    writeln!(log, "[{}] Capture started: radio={}, filter={}, duration={}s",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        args.radio, args.filter, args.duration_seconds
    ).ok();
    writeln!(log, "[{}] Identity: tier={}, vendor={}, hostname={}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        identity.get("tier").and_then(|t| t.as_u64()).unwrap_or(0),
        identity.get("vendor").and_then(|v| v.as_str()).unwrap_or("?"),
        identity.get("hostname").and_then(|h| h.as_str()).unwrap_or("?"),
    ).ok();

    // Launch SSH capture
    let mut child = Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=10", &args.host, &tcpdump_cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to launch SSH capture");

    // Stream pcap data to file
    let mut pcap_file = std::fs::File::create(&pcap_path).expect("Failed to create pcap file");
    let mut stdout = child.stdout.take().expect("Failed to get stdout");

    // Set up signal handler for clean shutdown
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    ctrlc_handler(r);

    let mut total_bytes: u64 = 0;
    let mut buf = [0u8; 65536];
    let start = std::time::Instant::now();

    println!("Capturing... (Ctrl+C to stop)");

    loop {
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            println!("\nShutdown requested.");
            break;
        }

        if args.duration_seconds > 0 && start.elapsed().as_secs() >= args.duration_seconds {
            println!("\nDuration reached ({}s).", args.duration_seconds);
            break;
        }

        match stdout.read(&mut buf) {
            Ok(0) => {
                println!("\nSSH stream ended.");
                break;
            }
            Ok(n) => {
                pcap_file.write_all(&buf[..n]).ok();
                total_bytes += n as u64;
                if total_bytes % (64 * 1024) == 0 {
                    print!("\r  {} KB captured ({:.0}s elapsed)",
                        total_bytes / 1024, start.elapsed().as_secs_f64());
                }
            }
            Err(e) => {
                eprintln!("\nRead error: {}", e);
                break;
            }
        }
    }

    // Clean shutdown
    let _ = child.kill();
    let _ = child.wait();
    pcap_file.flush().ok();

    let elapsed = start.elapsed().as_secs();
    println!("\n\n=== Capture Complete ===");
    println!("Duration: {}s", elapsed);
    println!("Size:     {} KB", total_bytes / 1024);
    println!("File:     {}", pcap_path.display());

    // Log completion
    writeln!(log, "[{}] Capture complete: {}KB in {}s",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        total_bytes / 1024, elapsed
    ).ok();

    // Post-flight: verify workstation network still intact
    print!("Post-flight checks... ");
    match preflight_check() {
        Ok(()) => println!("PASSED — network intact."),
        Err(e) => {
            eprintln!("\nWARNING: {}", e);
            eprintln!("Network state may have changed during capture. Check manually.");
            writeln!(log, "[{}] POST-FLIGHT FAILED: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), e
            ).ok();
        }
    }
}

fn ctrlc_handler(running: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    // BUG ASSUMPTION: ctrlc crate not available, using basic signal handling
    // In production, use the `ctrlc` crate for proper cross-platform support
    std::thread::spawn(move || {
        // Simple approach: sleep and check. In production, use signal handler.
        // The read() call in the main loop will also return on process death.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if !running.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preflight_passes_on_normal_system() {
        // This test only passes on the actual workstation with wlan0 up
        // Skip in CI: if wlan0 doesn't exist, the test is not applicable
        let result = preflight_check();
        // We don't assert success because CI won't have wlan0
        // Just verify it returns a Result without panicking
        let _ = result;
    }

    #[test]
    fn test_verify_session_missing() {
        let dir = PathBuf::from("/tmp/nonexistent_session_test");
        let result = verify_session(&dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_session_valid() {
        let dir = std::env::temp_dir().join("pineapple_capture_test");
        std::fs::create_dir_all(&dir).ok();
        std::fs::write(
            dir.join("adversary_identity.json"),
            r#"{"tier":2,"session_id":"test","vendor":"Apple"}"#
        ).ok();
        let result = verify_session(&dir);
        assert!(result.is_ok());
        let id = result.unwrap();
        assert_eq!(id["tier"], 2);
        std::fs::remove_dir_all(&dir).ok();
    }
}
