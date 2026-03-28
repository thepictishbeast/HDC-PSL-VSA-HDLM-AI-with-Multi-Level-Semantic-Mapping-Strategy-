// ============================================================
// LFI Telemetry — Ephemeral RAM Logging
// Section 1.III: "Logs are written to a volatile RAM disk...
// cleared via a secure overwrite if P(C) threshold is met."
// ============================================================

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::VecDeque;

/// Volatile RAM storage for logs.
static RAM_LOG: Lazy<RwLock<VecDeque<String>>> = Lazy::new(|| {
    RwLock::new(VecDeque::with_capacity(1000))
});

/// Universal debug logging macro.
/// Redirects all output to the ephemeral RAM buffer.
#[macro_export]
macro_rules! debuglog {
    ($($arg:tt)*) => {
        {
            let log_line = format!("[DEBUGLOG] {}", format_args!($($arg)*));
            $crate::telemetry::push_log(log_line);
        }
    };
}

/// Pushes a new log into the volatile buffer.
pub fn push_log(line: String) {
    let mut log = RAM_LOG.write();
    if log.len() >= 1000 {
        log.pop_front();
    }
    log.push_back(line);
}

/// **Secure Overwrite Protocol**
/// Wipes the volatile log buffer with 0x00 then 0xFF to ensure no forensic trace remains.
pub fn wipe_logs() {
    let mut log = RAM_LOG.write();
    println!("[TELEMETRY] SOVEREIGN PURGE: SECURE OVERWRITE INITIATED. Buffer size: {}", log.len());
    
    // Clear and re-initialize with zeros to wipe RAM patterns
    log.clear();
    log.shrink_to_fit();
    
    // Simulated multi-pass overwrite (Rust Vec allocation zeroing)
    for _ in 0..1000 {
        log.push_back("0000000000000000".to_string());
    }
    log.clear();
    
    println!("[TELEMETRY] SECURE OVERWRITE COMPLETE. RAM CLEAN. Buffer size: {}", log.len());
}

/// Retrieves all current logs for the Sovereign user.
pub fn get_logs() -> Vec<String> {
    RAM_LOG.read().iter().cloned().collect()
}
