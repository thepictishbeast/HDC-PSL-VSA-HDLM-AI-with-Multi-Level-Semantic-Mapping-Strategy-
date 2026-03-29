// ============================================================
// LFI Sovereign WebSocket & REST API
//
// ENDPOINTS:
//   GET  /ws/telemetry   — Real-time substrate telemetry stream
//   GET  /ws/chat        — Bidirectional chat with CognitiveCore
//   POST /api/auth       — Sovereign key verification
//   GET  /api/status     — Substrate status snapshot
//   GET  /api/facts      — Persistent knowledge facts
//   POST /api/search     — Web search with cross-referencing
//
// PROTOCOL: All WebSocket connections push JSON payloads.
// ============================================================

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use tokio::sync::broadcast;
use std::sync::Arc;
use parking_lot::Mutex;
use serde_json::json;
use serde::Deserialize;
use tracing::{info, debug, warn};
use tower_http::cors::CorsLayer;

use crate::agent::LfiAgent;
use crate::telemetry::MaterialAuditor;
use crate::intelligence::web_search::WebSearchEngine;

/// Shared application state across all handlers.
pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub agent: Mutex<LfiAgent>,
    pub search_engine: WebSearchEngine,
}

/// POST /api/auth body
#[derive(Deserialize)]
pub struct AuthRequest {
    pub key: String,
}

/// POST /api/search body
#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
}

// ============================================================
// WebSocket: Telemetry Stream
// ============================================================

pub async fn telemetry_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_telemetry_socket(socket, state))
}

async fn handle_telemetry_socket(mut socket: WebSocket, state: Arc<AppState>) {
    info!("// AUDIT: SCC Telemetry client connected.");

    loop {
        // Sample telemetry from the agent's VSA state
        let stats = {
            let _agent = state.agent.lock();
            let input_hv = crate::memory_bus::HyperMemory::new(crate::memory_bus::DIM_PROLETARIAT);
            let vsa_ortho = input_hv.audit_orthogonality();
            MaterialAuditor::get_stats(vsa_ortho, 1.0)
        };

        let payload = json!({
            "type": "telemetry",
            "data": stats
        }).to_string();

        if socket.send(Message::Text(payload)).await.is_err() {
            debug!("// AUDIT: Telemetry client disconnected.");
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    }
}

// ============================================================
// WebSocket: Chat Interface
// ============================================================

pub async fn chat_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_chat_socket(socket, state))
}

async fn handle_chat_socket(mut socket: WebSocket, state: Arc<AppState>) {
    info!("// AUDIT: SCC Chat client connected.");

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                debug!("// AUDIT: Chat input received: {} bytes", text.len());

                // Parse the incoming message
                let parsed: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => {
                        // Treat raw text as a chat message
                        json!({ "content": text })
                    }
                };

                let input = parsed.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if input.is_empty() {
                    continue;
                }

                // Route through CognitiveCore
                let response_payload = {
                    let mut agent = state.agent.lock();

                    // Auto-learn from conversational patterns
                    let lower = input.to_lowercase();
                    if lower.starts_with("my name is ") {
                        let name = input[11..].trim();
                        if !name.is_empty() {
                            agent.conversation_facts.insert("sovereign_name".to_string(), name.to_string());
                            let mut guard = agent.shared_knowledge.lock();
                            guard.store.upsert_fact("sovereign_name", name);
                        }
                    }

                    match agent.chat(input) {
                        Ok(response) => {
                            let thought = &response.thought;
                            json!({
                                "type": "chat_response",
                                "content": response.text,
                                "mode": format!("{:?}", thought.mode),
                                "confidence": thought.confidence,
                                "tier": format!("{:?}", agent.current_tier),
                                "intent": thought.intent.as_ref().map(|i| format!("{:?}", i)),
                                "reasoning": thought.reasoning_scratchpad,
                                "plan": thought.plan.as_ref().map(|p| json!({
                                    "steps": p.steps.len(),
                                    "complexity": p.total_complexity,
                                    "goal": p.goal,
                                })),
                            })
                        }
                        Err(e) => {
                            json!({
                                "type": "chat_error",
                                "error": format!("{:?}", e),
                            })
                        }
                    }
                };

                // Check if we should do a web search for unknown intents
                let content = response_payload.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if content.contains("not sure I fully understand") || content.contains("I don't have this") {
                    if let Ok(search_response) = state.search_engine.search(input) {
                        if !search_response.best_summary.is_empty() {
                            let web_payload = json!({
                                "type": "web_result",
                                "query": input,
                                "summary": &search_response.best_summary[..search_response.best_summary.len().min(500)],
                                "source_count": search_response.source_count,
                                "trust": search_response.cross_reference_trust,
                            });
                            let _ = socket.send(Message::Text(web_payload.to_string())).await;
                        }
                    }
                }

                if socket.send(Message::Text(response_payload.to_string())).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    info!("// AUDIT: SCC Chat client disconnected.");
}

// ============================================================
// REST: Authentication
// ============================================================

async fn auth_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuthRequest>,
) -> impl IntoResponse {
    let mut agent = state.agent.lock();
    if agent.authenticate(&req.key) {
        info!("// AUDIT: Sovereign authentication VERIFIED via REST.");
        Json(json!({ "status": "authenticated", "tier": "Sovereign" }))
    } else {
        warn!("// AUDIT: Authentication REJECTED via REST.");
        Json(json!({ "status": "rejected" }))
    }
}

// ============================================================
// REST: Status
// ============================================================

async fn status_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    let guard = agent.shared_knowledge.lock();
    Json(json!({
        "tier": format!("{:?}", agent.current_tier),
        "authenticated": agent.authenticated,
        "entropy": agent.entropy_level,
        "facts_count": guard.store.facts.len(),
        "concepts_count": guard.store.concepts.len(),
        "session_id": guard.store.current_session_id,
        "background_learning": agent.background_learner.is_running(),
    }))
}

// ============================================================
// REST: Facts
// ============================================================

async fn facts_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    let guard = agent.shared_knowledge.lock();
    let facts: Vec<_> = guard.store.facts.iter()
        .map(|f| json!({ "key": f.key, "value": f.value }))
        .collect();
    Json(json!({ "facts": facts, "count": facts.len() }))
}

// ============================================================
// REST: Web Search
// ============================================================

async fn search_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> impl IntoResponse {
    info!("// AUDIT: Web search requested via REST: '{}'", &req.query[..req.query.len().min(80)]);
    match state.search_engine.search(&req.query) {
        Ok(response) => {
            Json(json!({
                "query": req.query,
                "results": response.results.iter().take(5).map(|r| json!({
                    "title": r.title,
                    "snippet": r.snippet,
                    "source_url": r.source_url,
                    "backend": format!("{:?}", r.backend),
                    "trust": r.source_trust,
                })).collect::<Vec<_>>(),
                "source_count": response.source_count,
                "cross_reference_trust": response.cross_reference_trust,
                "best_summary": response.best_summary,
            }))
        }
        Err(e) => {
            Json(json!({ "error": format!("{:?}", e) }))
        }
    }
}

// ============================================================
// REST: QoS Compliance Report
// ============================================================

async fn qos_handler() -> impl IntoResponse {
    info!("// AUDIT: QoS compliance report requested.");
    let auditor = crate::qos::QosAuditor::new();
    let report = auditor.audit(1.0); // TODO: wire to real axiom history
    Json(serde_json::to_value(&report).unwrap_or(json!({ "error": "serialization failed" })))
}

// ============================================================
// Router Construction
// ============================================================

pub fn create_router() -> Router {
    let (tx, _) = broadcast::channel(100);

    let agent = LfiAgent::new().expect("Failed to initialize LfiAgent for API");
    let state = Arc::new(AppState {
        tx,
        agent: Mutex::new(agent),
        search_engine: WebSearchEngine::new(),
    });

    let cors = CorsLayer::permissive();

    Router::new()
        .route("/ws/telemetry", get(telemetry_handler))
        .route("/ws/chat", get(chat_handler))
        .route("/api/auth", post(auth_handler))
        .route("/api/status", get(status_handler))
        .route("/api/facts", get(facts_handler))
        .route("/api/search", post(search_handler))
        .route("/api/qos", get(qos_handler))
        .layer(cors)
        .with_state(state)
}
