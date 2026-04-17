// NODE 029: SCC Backend Telemetry Server
// STATUS: ALPHA - WebSocket Broadcast Active
// PROTOCOL: Substrate-to-UI Bridge

use lfi_vsa_core::api::create_router;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize standard tracing
    // Structured logging to both stdout and /var/log/lfi/server.log
    let _ = std::fs::create_dir_all("/var/log/lfi");
    let file_appender = tracing_appender::rolling::daily("/var/log/lfi", "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(non_blocking)
        .try_init();

    // SUPERSOCIETY: Startup banner — log version, environment, and system state
    info!("╔══════════════════════════════════════════════╗");
    info!("║  PlausiDen AI — Sovereign Intelligence v{}  ║", env!("CARGO_PKG_VERSION"));
    info!("╚══════════════════════════════════════════════╝");
    info!("// AUDIT: Starting server on ws://0.0.0.0:3000");
    info!("// AUDIT: Architecture: neurosymbolic (HDC + PSL)");
    info!("// AUDIT: Mode: local-first, zero-telemetry");

    // Log brain.db status if available
    let db_path = std::path::Path::new(
        &std::env::var("BRAINDB_PATH")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
                format!("{}/.local/share/plausiden/brain.db", home)
            })
    ).to_path_buf();
    if db_path.exists() {
        if let Ok(conn) = rusqlite::Connection::open_with_flags(
            &db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            if let Ok(count) = conn.query_row("SELECT count(*) FROM facts", [], |r| r.get::<_, i64>(0)) {
                let domains: i64 = conn.query_row("SELECT count(DISTINCT domain) FROM facts", [], |r| r.get(0)).unwrap_or(0);
                let sources: i64 = conn.query_row("SELECT count(DISTINCT source) FROM facts", [], |r| r.get(0)).unwrap_or(0);
                info!("// AUDIT: brain.db: {} facts, {} domains, {} sources", count, domains, sources);
            }
        }
    }

    // Check Ollama availability
    match std::process::Command::new("curl")
        .args(["-sf", "http://localhost:11434/api/tags", "--max-time", "3"])
        .output()
    {
        Ok(o) if o.status.success() => info!("// AUDIT: Ollama: available"),
        _ => info!("// AUDIT: Ollama: not available (local inference only)"),
    }

    let app = create_router()?;
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// NOTE: Logging is configured via tracing-subscriber in main()
// Logs go to both stdout AND /var/log/lfi/server.log
// Log rotation: daily rotation via tracing_appender::rolling::daily
// For size-based rotation (max 100MB), would need tracing-appender 0.3+
// with RollingFileAppender::builder().max_log_files(10).build()
