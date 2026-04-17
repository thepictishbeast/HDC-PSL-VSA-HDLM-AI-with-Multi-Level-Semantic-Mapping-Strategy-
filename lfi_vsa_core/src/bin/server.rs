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

    info!("// AUDIT: Starting Sovereign Command Console (SCC) Backend on ws://0.0.0.0:3000...");
    
    let app = create_router()?;
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// NOTE: Logging is configured via tracing-subscriber in main()
// Logs go to both stdout AND /var/log/lfi/server.log
// TODO: Add rolling file appender for rotation
