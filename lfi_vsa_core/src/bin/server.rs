// NODE 029: SCC Backend Telemetry Server
// STATUS: ALPHA - WebSocket Broadcast Active
// PROTOCOL: Substrate-to-UI Bridge

use lfi_vsa_core::api::create_router;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize standard tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("// AUDIT: Starting Sovereign Command Console (SCC) Backend on ws://0.0.0.0:3000...");
    
    let app = create_router();
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
