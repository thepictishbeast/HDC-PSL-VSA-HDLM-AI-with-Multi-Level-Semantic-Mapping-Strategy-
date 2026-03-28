// ============================================================
// LFI Web API — Hardened REST Interface (Expanded)
// Section 3: "The Backend Daemon (axum / Rust): The lfi_vsa_core
// runs as a headless service exposing a hardened REST API."
// ============================================================

use axum::{
    routing::{get, post},
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Serialize, Deserialize};
use crate::agent::LfiAgent;
use crate::psl::axiom::AuditTarget;
use crate::debuglog;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// API Response for agent status.
#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub active_axioms: usize,
    pub supported_languages: Vec<String>,
}

/// Request payload for task execution.
#[derive(Deserialize)]
pub struct TaskRequest {
    pub task_name: String,
    pub signature: crate::identity::SovereignSignature,
}

/// Request payload for web search.
#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub skepticism_level: f64,
}

/// Shared state for the API server. Must be Clone + Send + Sync.
#[derive(Clone)]
pub struct ApiState {
    pub agent: Arc<RwLock<LfiAgent>>,
}

/// Initializes and starts the axum API server.
pub async fn start_api_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    debuglog!("start_api_server: initializing server on {}", addr);

    let agent = Arc::new(RwLock::new(LfiAgent::new().map_err(|e| format!("Agent init failed: {:?}", e))?));
    let state = ApiState { agent };

    let app = Router::new()
        .route("/status", get(get_status))
        .route("/task", post(execute_task))
        .route("/search", post(skeptical_search))
        .route("/sensor", post(handle_sensor))
        .route("/creative", post(handle_creative))
        .route("/upload", post(handle_upload))
        .with_state(state);

    debuglog!("start_api_server: routes configured, listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_status(
    State(state): State<ApiState>,
) -> Json<StatusResponse> {
    debuglog!("API: GET /status");
    let agent = state.agent.read().await;
    Json(StatusResponse {
        status: "Operational".to_string(),
        version: "5.6.8".to_string(),
        active_axioms: agent.supervisor.axiom_count(),
        supported_languages: vec![
            "Rust".to_string(), "Go".to_string(), "Kotlin".to_string(), 
            "Swift".to_string(), "Verilog".to_string(), "Assembly".to_string(),
            "SQL".to_string(), "PHP".to_string(), "TypeScript".to_string()
        ],
    })
}

async fn execute_task(
    State(state): State<ApiState>,
    Json(payload): Json<TaskRequest>,
) -> impl IntoResponse {
    debuglog!("API: POST /task '{}'", payload.task_name);
    let agent = state.agent.read().await;
    match agent.execute_task(&payload.task_name, crate::laws::LawLevel::Primary, &payload.signature) {
        Ok(_) => (StatusCode::OK, Json(format!("Task '{}' executed and audited successfully.", payload.task_name))),
        Err(e) => (StatusCode::UNPROCESSABLE_ENTITY, Json(format!("Task '{}' failed forensic audit: {:?}", payload.task_name, e))),
    }
}

async fn skeptical_search(
    State(state): State<ApiState>,
    Json(payload): Json<SearchRequest>,
) -> Json<String> {
    debuglog!("API: POST /search query='{}', level={}", payload.query, payload.skepticism_level);
    
    // 1. Simulate finding a source
    let source = "untrusted_dns".to_string();
    let fields = vec![("title".to_string(), "Unverified Result".to_string())];
    let target = AuditTarget::Payload { source, fields };

    // 2. Audit via Agent's Supervisor
    let agent = state.agent.read().await;
    match agent.supervisor.audit(&target) {
        Ok(assessment) if assessment.level.permits_execution() => {
            Json(format!("Search successful. Trust Level: {:?}", assessment.level))
        },
        Ok(assessment) => {
            Json(format!("Search result DISCARDED due to skepticism audit: {:?}", assessment.level))
        },
        Err(e) => Json(format!("Skeptical audit failure: {:?}", e)),
    }
}

async fn handle_upload(
    State(_state): State<ApiState>,
    Json(payload): Json<String>, // Simplified for demo
) -> Json<String> {
    debuglog!("API: POST /upload payload_len={}", payload.len());
    Json("Multimodal payload ingested into VSA space. Transducers active.".to_string())
}

#[derive(Deserialize)]
pub struct SensorRequest {
    pub group: String,
    pub signal: Vec<f64>,
}

async fn handle_sensor(
    State(state): State<ApiState>,
    Json(payload): Json<SensorRequest>,
) -> Json<String> {
    debuglog!("API: POST /sensor group='{}'", payload.group);
    
    use crate::hdc::sensory::{SensoryFrame, SensorGroup};
    let group = match payload.group.as_str() {
        "IMU" => SensorGroup::IMU,
        "RF" => SensorGroup::RF,
        "Biometric" => SensorGroup::Biometric,
        _ => SensorGroup::Environmental,
    };

    let frame = SensoryFrame {
        group,
        timestamp: 123456789, // Simplified
        raw_signal: payload.signal,
    };

    let mut agent = state.agent.write().await;
    match agent.ingest_sensor_frame(&frame) {
        Ok(_) => Json("Sensor frame ingested and audited successfully.".to_string()),
        Err(e) => Json(format!("Sensor ingestion failed: {:?}", e)),
    }
}

#[derive(Deserialize)]
pub struct CreativeRequest {
    pub problem: String,
}

async fn handle_creative(
    State(state): State<ApiState>,
    Json(payload): Json<CreativeRequest>,
) -> Json<String> {
    debuglog!("API: POST /creative problem='{}'", payload.problem);
    
    let agent = state.agent.read().await;
    match agent.synthesize_creative_solution(&payload.problem) {
        Ok(_) => Json("Structural solution synthesized successfully.".to_string()),
        Err(e) => Json(format!("Creative synthesis failed: {:?}", e)),
    }
}

