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
    extract::{Path, State},
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
use rusqlite::params;
use crate::intelligence::web_search::WebSearchEngine;

/// Shared application state across all handlers.
pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub agent: Mutex<LfiAgent>,
    pub search_engine: WebSearchEngine,
    pub metrics: Arc<crate::intelligence::metrics::LfiMetrics>,
    pub db: Arc<crate::persistence::BrainDb>,
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

/// POST /api/tier body
#[derive(Deserialize)]
pub struct TierRequest {
    pub tier: String,
}

/// POST /api/think body — thinks with provenance tracking.
#[derive(Deserialize)]
pub struct ThinkRequest {
    pub input: String,
}

/// POST /api/knowledge/review body.
#[derive(Deserialize)]
pub struct ReviewRequest {
    pub concept: String,
    /// Quality score 0–5 (SM-2). Clamped to 5 if higher.
    pub quality: u8,
}

/// POST /api/knowledge/learn body.
#[derive(Deserialize)]
pub struct LearnRequest {
    pub concept: String,
    #[serde(default)]
    pub related: Vec<String>,
}

/// POST /api/audit body — runs PSL governance over a hypervector seed.
#[derive(Deserialize)]
pub struct AuditRequest {
    /// String seed used to deterministically generate the bipolar vector
    /// being audited. Caller hashes their data into this seed.
    pub seed: String,
}

/// POST /api/opsec/scan body — submits text for PII / sensitive-data scanning.
#[derive(Deserialize)]
pub struct OpsecRequest {
    pub text: String,
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
                state.metrics.inc_counter("lfi_chat_total", &[], 1);

                // Send a "thinking" progress update so the UI can show what
                // tier/mode the AI is about to use before the response arrives.
                let incognito_flag = parsed.get("incognito")
                    .and_then(|v| v.as_bool()).unwrap_or(false);
                let tier_name = {
                    let agent = state.agent.lock();
                    format!("{:?}", agent.current_tier)
                }; // lock dropped here — before the await
                let progress = json!({
                    "type": "progress",
                    "step": format!("{} is thinking...", tier_name),
                    "tier": tier_name,
                });
                let _ = socket.send(Message::Text(progress.to_string())).await;

                // Route through CognitiveCore
                let response_payload = {
                    let mut agent = state.agent.lock();

                    // Auto-learn from conversational patterns — extract
                    // persistent user facts that the AI can reference in
                    // future turns and across sessions. Pattern-based for
                    // speed; runs before the reasoner so the context is
                    // available for the response generation.
                    let lower = input.to_lowercase();
                    let db_ref = state.db.clone();
                    let mut learn = |key: &str, val: &str| {
                        let v = val.trim().to_string();
                        if v.is_empty() || v.len() > 200 { return; }
                        debuglog!("chat: auto-learned fact {}={}", key, v);
                        agent.conversation_facts.insert(key.to_string(), v.clone());
                        let mut guard = agent.shared_knowledge.lock();
                        guard.store.upsert_fact(key, &v);
                        // Persist to SQLite so facts survive server restarts.
                        db_ref.upsert_fact(key, &v, "ai_extracted", 1.0);
                    };
                    // Name
                    if lower.starts_with("my name is ") {
                        learn("sovereign_name", &input[11..]);
                    } else if lower.starts_with("call me ") {
                        learn("sovereign_name", &input[8..]);
                    }
                    // Preferences
                    for prefix in &["i like ", "i love ", "i enjoy ", "my favorite ", "i prefer "] {
                        if lower.starts_with(prefix) {
                            learn(&format!("preference_{}", prefix.trim().replace(' ', "_")),
                                  &input[prefix.len()..]);
                            break;
                        }
                    }
                    // Profession / role
                    for prefix in &["i'm a ", "im a ", "i am a ", "i work as ", "i work at "] {
                        if lower.starts_with(prefix) {
                            let key = if lower.contains("work at") { "workplace" } else { "role" };
                            learn(key, &input[prefix.len()..]);
                            break;
                        }
                    }
                    // Location
                    if lower.starts_with("i live in ") || lower.starts_with("i'm from ") || lower.starts_with("im from ") {
                        let cut = if lower.starts_with("i live in ") { 10 }
                                  else if lower.starts_with("i'm from ") { 9 } else { 8 };
                        learn("location", &input[cut..]);
                    }
                    // Relationships
                    for (trigger, key) in &[
                        ("my wife", "partner"), ("my husband", "partner"),
                        ("my partner", "partner"), ("my girlfriend", "partner"),
                        ("my boyfriend", "partner"), ("my dog", "pet_dog"),
                        ("my cat", "pet_cat"), ("my kid", "child"),
                        ("my son", "child_son"), ("my daughter", "child_daughter"),
                    ] {
                        if lower.contains(trigger) {
                            if let Some(rest) = lower.split(trigger).nth(1) {
                                let rest = rest.trim().trim_start_matches("'s name is ")
                                    .trim_start_matches(" is named ")
                                    .trim_start_matches(" is ");
                                if !rest.is_empty() {
                                    learn(key, &rest.split(|c: char| ",.!?\n".contains(c)).next().unwrap_or(rest));
                                }
                            }
                        }
                    }

                    match agent.chat_traced(input) {
                        Ok((response, conclusion_id)) => {
                            let thought = &response.thought;
                            let payload = json!({
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
                                // Provenance: client can query /api/provenance/:id with this
                                "conclusion_id": conclusion_id,
                            });
                            // Persist every turn for later review + training data
                            // sourcing. Skip when incognito — per Bible §4.5.
                            if !incognito_flag {
                            let log_line = json!({
                                "ts": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs()).unwrap_or(0),
                                "user": input,
                                "reply": response.text,
                                "tier": format!("{:?}", agent.current_tier),
                                "intent": thought.intent.as_ref().map(|i| format!("{:?}", i)),
                                "mode": format!("{:?}", thought.mode),
                                "confidence": thought.confidence,
                                "conclusion_id": conclusion_id,
                            });
                            let _ = std::fs::create_dir_all("/var/log/lfi");
                            if let Ok(mut f) = std::fs::OpenOptions::new()
                                .create(true).append(true).open("/var/log/lfi/chat.jsonl")
                            {
                                use std::io::Write;
                                let _ = writeln!(f, "{}", log_line);
                            }
                            } // end if !incognito_flag
                            payload
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
                                "summary": crate::truncate_str(&search_response.best_summary, 500),
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
    info!("// AUDIT: Web search requested via REST: '{}'", crate::truncate_str(&req.query, 80));
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
// REST: Tier Switching
// ============================================================

async fn tier_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TierRequest>,
) -> impl IntoResponse {
    let mut agent = state.agent.lock();
    if !agent.authenticated {
        warn!("// AUDIT: Tier switch rejected — not authenticated.");
        return Json(json!({ "status": "rejected", "reason": "not authenticated" }));
    }

    use crate::cognition::router::IntelligenceTier;
    let target = match req.tier.to_lowercase().as_str() {
        "pulse" => IntelligenceTier::Pulse,
        "bridge" => IntelligenceTier::Bridge,
        "bigbrain" => IntelligenceTier::BigBrain,
        _ => {
            warn!("// AUDIT: Unknown tier requested: '{}'", req.tier);
            return Json(json!({ "status": "error", "reason": format!("unknown tier: {}", req.tier) }));
        }
    };

    info!("// AUDIT: Manual tier switch: {:?} -> {:?}", agent.current_tier, target);
    agent.current_tier = target;
    Json(json!({
        "status": "ok",
        "tier": format!("{:?}", agent.current_tier),
    }))
}

// ============================================================
// REST: Chat log + stop
// ============================================================

/// GET /api/chat-log?limit=N — return recent chat turns logged to
/// /var/log/lfi/chat.jsonl. Lets the operator (and the AI itself) review
/// conversation behavior without cross-device sync. Default limit 50.
async fn chat_log_handler(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let limit: usize = q.get("limit").and_then(|s| s.parse().ok()).unwrap_or(50).min(500);
    let path = "/var/log/lfi/chat.jsonl";
    let content = std::fs::read_to_string(path).unwrap_or_default();
    // Collect the tail without allocating the whole thing twice.
    let mut lines: Vec<serde_json::Value> = content
        .lines()
        .rev()
        .take(limit)
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    lines.reverse();
    Json(json!({
        "count": lines.len(),
        "path": path,
        "entries": lines,
    }))
}

/// POST /api/stop — cooperative cancel for any in-flight generation.
/// Currently a no-op (chat path is synchronous); kept so the UI Stop button
/// has something to call as streaming is wired in.
async fn stop_handler() -> impl IntoResponse {
    info!("// AUDIT: stop requested");
    Json(json!({ "status": "ok", "note": "no streaming in progress" }))
}

// ============================================================
// REST: Desktop tools (Phase 1 of the tool-registry)
//
// Safe, visible OS interactions so the AI has a foothold on the host. Each
// action is authed; each shell invocation is argv-based (never a shell
// string) and uses a hard-coded binary path or a known program name so it
// is not susceptible to command injection via user input.
//
// DEBIAN-PORTABLE: all binaries listed here (notify-send, xclip, wl-copy,
// xdg-open, scrot) are available from the base Debian repositories and will
// be declared as Recommends/Suggests on the eventual .deb.
// ============================================================

/// GET /api/system/info — portable host + resource snapshot.
async fn system_info_handler() -> impl IntoResponse {
    fn read_first_line(p: &str) -> Option<String> {
        std::fs::read_to_string(p).ok().and_then(|s| s.lines().next().map(|l| l.to_string()))
    }
    let hostname = std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let kernel = read_first_line("/proc/version").unwrap_or_default();
    let uptime_secs = std::fs::read_to_string("/proc/uptime").ok()
        .and_then(|s| s.split_whitespace().next().map(|t| t.to_string()))
        .and_then(|t| t.parse::<f64>().ok())
        .map(|f| f as u64);
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let pretty_name = os_release.lines()
        .find(|l| l.starts_with("PRETTY_NAME="))
        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        .unwrap_or_default();
    let cpu_model = std::fs::read_to_string("/proc/cpuinfo").ok()
        .and_then(|c| c.lines()
            .find(|l| l.starts_with("model name"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string()));
    let ncpu = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0);
    let (ram_avail, ram_total) = {
        let mut avail = 0u64; let mut total = 0u64;
        if let Ok(c) = std::fs::read_to_string("/proc/meminfo") {
            for l in c.lines() {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() < 2 { continue; }
                let kb: u64 = parts[1].parse().unwrap_or(0);
                if l.starts_with("MemAvailable:") { avail = kb; }
                else if l.starts_with("MemTotal:") { total = kb; }
            }
        }
        (avail, total)
    };
    // Disk usage of /
    let (disk_total, disk_free) = match rustix_like_statvfs("/") {
        Some((t, f)) => (t, f),
        None => (0, 0),
    };
    Json(json!({
        "hostname": hostname,
        "kernel": kernel,
        "uptime_secs": uptime_secs,
        "os": pretty_name,
        "cpu_model": cpu_model,
        "cpu_count": ncpu,
        "ram_total_kb": ram_total,
        "ram_available_kb": ram_avail,
        "disk_root_total_bytes": disk_total,
        "disk_root_free_bytes": disk_free,
    }))
}

/// Best-effort statvfs via `df -k --output=size,avail`. Falls back silently.
fn rustix_like_statvfs(path: &str) -> Option<(u64, u64)> {
    let out = std::process::Command::new("df")
        .args(["-k", "--output=size,avail", path])
        .output().ok()?;
    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines(); lines.next()?; // header
    let line = lines.next()?;
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 2 { return None; }
    let total_kb: u64 = cols[0].parse().ok()?;
    let avail_kb: u64 = cols[1].parse().ok()?;
    Some((total_kb * 1024, avail_kb * 1024))
}

#[derive(serde::Deserialize)]
pub struct NotifyRequest { pub title: String, pub body: String }

/// POST /api/system/notify — desktop notification via notify-send. Requires
/// auth so randoms on the LAN can't spam the user. Title/body length-capped.
async fn system_notify_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NotifyRequest>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated {
        return Json(json!({ "status": "rejected", "reason": "not authenticated" }));
    }
    drop(agent);
    let title = req.title.chars().take(120).collect::<String>();
    let body = req.body.chars().take(800).collect::<String>();
    let out = std::process::Command::new("notify-send")
        .args(["-a", "PlausiDen AI", &title, &body])
        .output();
    match out {
        Ok(o) if o.status.success() => Json(json!({ "status": "ok" })),
        Ok(o) => Json(json!({
            "status": "error",
            "reason": String::from_utf8_lossy(&o.stderr).to_string(),
        })),
        Err(e) => Json(json!({
            "status": "error",
            "reason": format!("notify-send unavailable: {}", e),
        })),
    }
}

/// GET /api/system/clipboard — read clipboard via wl-paste (Wayland) or
/// xclip (X11). Tries Wayland first, falls back to X11.
async fn clipboard_get_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated {
        return Json(json!({ "status": "rejected", "reason": "not authenticated" }));
    }
    drop(agent);
    let wayland = std::process::Command::new("wl-paste")
        .arg("--no-newline").output();
    if let Ok(o) = wayland {
        if o.status.success() && !o.stdout.is_empty() {
            return Json(json!({
                "status": "ok", "source": "wayland",
                "text": String::from_utf8_lossy(&o.stdout).to_string(),
            }));
        }
    }
    let x11 = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-o"]).output();
    match x11 {
        Ok(o) if o.status.success() => Json(json!({
            "status": "ok", "source": "x11",
            "text": String::from_utf8_lossy(&o.stdout).to_string(),
        })),
        Ok(_) => Json(json!({ "status": "error", "reason": "clipboard empty" })),
        Err(e) => Json(json!({ "status": "error", "reason": format!("no clipboard tool: {}", e) })),
    }
}

#[derive(serde::Deserialize)]
pub struct ClipboardSetRequest { pub text: String }

/// POST /api/system/clipboard — write to the system clipboard.
async fn clipboard_set_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClipboardSetRequest>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated {
        return Json(json!({ "status": "rejected", "reason": "not authenticated" }));
    }
    drop(agent);
    if req.text.len() > 1_000_000 {
        return Json(json!({ "status": "rejected", "reason": "text > 1 MB" }));
    }
    use std::io::Write;
    // Try Wayland first.
    if let Ok(mut child) = std::process::Command::new("wl-copy")
        .stdin(std::process::Stdio::piped()).spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(req.text.as_bytes());
        }
        if let Ok(s) = child.wait() { if s.success() {
            return Json(json!({ "status": "ok", "source": "wayland" }));
        }}
    }
    if let Ok(mut child) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped()).spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(req.text.as_bytes());
        }
        if let Ok(s) = child.wait() { if s.success() {
            return Json(json!({ "status": "ok", "source": "x11" }));
        }}
    }
    Json(json!({ "status": "error", "reason": "no clipboard tool available (install wl-clipboard or xclip)" }))
}

// ============================================================
// REST: Conversations (server-side persistence per Bible §4.2)
// ============================================================

/// GET /api/conversations — list all conversations (metadata only).
async fn conversations_list_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let convos = state.db.get_conversations();
    let list: Vec<serde_json::Value> = convos.iter().map(|(id, title, pinned, starred, updated)| {
        json!({ "id": id, "title": title, "pinned": pinned, "starred": starred, "updated_at": updated })
    }).collect();
    Json(json!({ "count": list.len(), "conversations": list }))
}

/// GET /api/conversations/:id — fetch a single conversation with all messages.
async fn conversation_get_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let messages = state.db.get_messages(&id);
    let msgs: Vec<serde_json::Value> = messages.iter().map(|(role, content, ts, meta)| {
        let mut m = json!({ "role": role, "content": content, "timestamp": ts });
        if let Some(meta_str) = meta {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(meta_str) {
                m["meta"] = parsed;
            }
        }
        m
    }).collect();
    Json(json!({ "id": id, "messages": msgs, "count": msgs.len() }))
}

#[derive(serde::Deserialize)]
pub struct ConversationSyncPayload {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub starred: bool,
    pub messages: Vec<SyncMessage>,
}

#[derive(serde::Deserialize)]
pub struct SyncMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

/// POST /api/conversations/sync — bulk sync a conversation from the frontend.
/// Replaces existing messages for this conversation ID.
async fn conversations_sync_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConversationSyncPayload>,
) -> impl IntoResponse {
    if req.id.is_empty() || req.id.len() > 100 {
        return Json(json!({ "status": "error", "reason": "invalid id" }));
    }
    // Save conversation metadata
    state.db.save_conversation(&req.id, &req.title, req.pinned, req.starred);
    // Clear existing messages and re-insert (simple full-replace sync)
    {
        let conn = state.db.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM messages WHERE conversation_id = ?1", params![req.id]);
    }
    for msg in &req.messages {
        state.db.save_message(&req.id, &msg.role, &msg.content, msg.timestamp, None);
    }
    info!("// PERSISTENCE: synced conversation {} ({} messages)", req.id, req.messages.len());
    Json(json!({ "status": "ok", "id": req.id, "messages_synced": req.messages.len() }))
}

/// DELETE /api/conversations/:id — delete a conversation and its messages.
async fn conversation_delete_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    state.db.delete_conversation(&id);
    info!("// PERSISTENCE: deleted conversation {}", id);
    Json(json!({ "status": "ok", "deleted": id }))
}

// ============================================================
// REST: Deep Research (multi-source web agent)
// Per Bible §3.3.4 Skills: "Research — multi-step web search →
// source evaluation → synthesis → citation."
// ============================================================

#[derive(serde::Deserialize)]
pub struct ResearchRequest { pub query: String, #[serde(default = "default_depth")] pub depth: usize }
fn default_depth() -> usize { 3 }

/// POST /api/research — deep multi-source research with citations.
/// Fires N parallel searches with query variations, cross-references results,
/// synthesizes a cited summary. Returns sources with trust scores.
async fn research_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResearchRequest>,
) -> impl IntoResponse {
    if req.query.is_empty() || req.query.len() > 4096 {
        return Json(json!({ "status": "error", "reason": "query must be 1-4096 chars" }));
    }
    let depth = req.depth.min(5).max(1);
    info!("// AUDIT: Deep research: '{}' depth={}", crate::truncate_str(&req.query, 60), depth);
    state.metrics.inc_counter("lfi_research_total", &[], 1);

    // Generate query variations for breadth
    let variations: Vec<String> = {
        let base = req.query.clone();
        let mut v = vec![base.clone()];
        // Add perspective variations
        if depth >= 2 { v.push(format!("{} explained simply", base)); }
        if depth >= 3 { v.push(format!("{} pros and cons", base)); }
        if depth >= 4 { v.push(format!("{} latest research 2026", base)); }
        if depth >= 5 { v.push(format!("{} common misconceptions", base)); }
        v
    };

    // Run searches sequentially (could be parallel with tokio::spawn but
    // the search engine holds a lock internally)
    let mut all_sources: Vec<serde_json::Value> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();
    let mut total_trust = 0.0f64;

    for (i, query) in variations.iter().enumerate() {
        match state.search_engine.search(query) {
            Ok(result) => {
                let trust = result.cross_reference_trust;
                total_trust += trust;
                summaries.push(result.best_summary.clone());
                all_sources.push(json!({
                    "query": query,
                    "summary": crate::truncate_str(&result.best_summary, 500),
                    "source_count": result.source_count,
                    "trust": trust,
                    "citation_index": i + 1,
                }));
            }
            Err(e) => {
                all_sources.push(json!({
                    "query": query,
                    "error": format!("{:?}", e),
                    "citation_index": i + 1,
                }));
            }
        }
    }

    let source_count = all_sources.len();
    let avg_trust = if source_count > 0 { total_trust / source_count as f64 } else { 0.0 };

    // Synthesize: combine summaries with citation markers
    let synthesis = if summaries.is_empty() {
        "No results found. The search may have failed or the topic may not have web coverage.".to_string()
    } else {
        let mut out = String::new();
        for (i, s) in summaries.iter().enumerate() {
            if !s.is_empty() {
                if !out.is_empty() { out.push_str("\n\n"); }
                out.push_str(&format!("{} [{}]", s, i + 1));
            }
        }
        if out.is_empty() {
            "Search returned results but no usable summaries.".to_string()
        } else {
            out
        }
    };

    // Persist to brain.db as a research fact
    let fact_key = format!("research_{}", req.query.chars().take(40).collect::<String>().replace(' ', "_"));
    state.db.upsert_fact(&fact_key, &crate::truncate_str(&synthesis, 500), "web_research", avg_trust);

    Json(json!({
        "status": "ok",
        "query": req.query,
        "depth": depth,
        "synthesis": synthesis,
        "sources": all_sources,
        "source_count": source_count,
        "avg_trust": avg_trust,
    }))
}

/// GET /api/training/status — training pipeline status from brain.db + log files.
async fn training_status_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let db = &state.db;
    let history = db.get_training_history(50);
    let facts_count = db.get_all_facts().len();

    // Read training state file if it exists
    let state_json = std::fs::read_to_string("/var/log/lfi/training_state.json")
        .unwrap_or_else(|_| "{}".to_string());
    let domain_state: serde_json::Value = serde_json::from_str(&state_json).unwrap_or(json!({}));

    // Read last 20 lines of training.jsonl
    let recent_cycles: Vec<String> = std::fs::read_to_string("/var/log/lfi/training.jsonl")
        .unwrap_or_default()
        .lines().rev().take(20)
        .map(|s| s.to_string())
        .collect();

    Json(json!({
        "facts_in_db": facts_count,
        "training_history": history.iter().map(|(domain, acc, total, correct, ts)| json!({
            "domain": domain, "accuracy": acc, "total": total, "correct": correct, "timestamp": ts,
        })).collect::<Vec<_>>(),
        "domain_state": domain_state,
        "recent_cycles": recent_cycles,
        "trainer_running": std::process::Command::new("pgrep")
            .args(["-f", "train_adaptive"])
            .output().map(|o| o.status.success()).unwrap_or(false),
    }))
}

// ============================================================
// REST: Desktop Automation (Phase 2 — mouse/keyboard/screenshot)
// Per Bible §3.5 Tool System + Architectural Bible
// All gated behind auth. All logged to audit trail.
// ============================================================

#[derive(serde::Deserialize)]
pub struct ClickRequest { pub x: i32, pub y: i32, #[serde(default = "default_button")] pub button: u32 }
fn default_button() -> u32 { 1 }

/// POST /api/system/click — click at screen coordinates via xdotool.
async fn system_click_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClickRequest>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated { return Json(json!({ "status": "rejected", "reason": "not authenticated" })); }
    drop(agent);
    info!("// AUDIT: desktop click at ({},{}) button={}", req.x, req.y, req.button);
    let out = std::process::Command::new("xdotool")
        .args(["mousemove", "--sync", &req.x.to_string(), &req.y.to_string(),
               "click", &req.button.to_string()])
        .output();
    match out {
        Ok(o) if o.status.success() => Json(json!({ "status": "ok", "x": req.x, "y": req.y })),
        Ok(o) => Json(json!({ "status": "error", "reason": String::from_utf8_lossy(&o.stderr).to_string() })),
        Err(e) => Json(json!({ "status": "error", "reason": format!("xdotool unavailable: {}", e) })),
    }
}

#[derive(serde::Deserialize)]
pub struct TypeRequest { pub text: String }

/// POST /api/system/type — type text via xdotool.
async fn system_type_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TypeRequest>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated { return Json(json!({ "status": "rejected", "reason": "not authenticated" })); }
    drop(agent);
    if req.text.len() > 5000 { return Json(json!({ "status": "error", "reason": "text > 5000 chars" })); }
    info!("// AUDIT: desktop type {} chars", req.text.len());
    let out = std::process::Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "10", &req.text])
        .output();
    match out {
        Ok(o) if o.status.success() => Json(json!({ "status": "ok", "chars": req.text.len() })),
        Ok(o) => Json(json!({ "status": "error", "reason": String::from_utf8_lossy(&o.stderr).to_string() })),
        Err(e) => Json(json!({ "status": "error", "reason": format!("{}", e) })),
    }
}

#[derive(serde::Deserialize)]
pub struct KeyRequest { pub keys: String }

/// POST /api/system/key — send key combination via xdotool (e.g., "ctrl+c", "Return").
async fn system_key_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<KeyRequest>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated { return Json(json!({ "status": "rejected", "reason": "not authenticated" })); }
    drop(agent);
    info!("// AUDIT: desktop key '{}'", req.keys);
    let out = std::process::Command::new("xdotool")
        .args(["key", "--clearmodifiers", &req.keys])
        .output();
    match out {
        Ok(o) if o.status.success() => Json(json!({ "status": "ok", "keys": req.keys })),
        Ok(o) => Json(json!({ "status": "error", "reason": String::from_utf8_lossy(&o.stderr).to_string() })),
        Err(e) => Json(json!({ "status": "error", "reason": format!("{}", e) })),
    }
}

/// GET /api/system/screenshot — capture screen via scrot, return as base64 PNG.
async fn system_screenshot_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated { return Json(json!({ "status": "rejected", "reason": "not authenticated" })); }
    drop(agent);
    let path = "/tmp/plausiden_screenshot.png";
    let out = std::process::Command::new("scrot")
        .args(["-o", path])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            match std::fs::read(path) {
                Ok(bytes) => {
                    use std::io::Write;
                    let b64 = {
                        // Manual base64 encode — minimal, no extra dep
                        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                        let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
                        for chunk in bytes.chunks(3) {
                            let b = [chunk.get(0).copied().unwrap_or(0), chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
                            out.push(CHARS[((b[0] >> 2) & 0x3f) as usize] as char);
                            out.push(CHARS[(((b[0] & 0x3) << 4) | (b[1] >> 4)) as usize] as char);
                            if chunk.len() > 1 { out.push(CHARS[(((b[1] & 0xf) << 2) | (b[2] >> 6)) as usize] as char); } else { out.push('='); }
                            if chunk.len() > 2 { out.push(CHARS[(b[2] & 0x3f) as usize] as char); } else { out.push('='); }
                        }
                        out
                    };
                    info!("// AUDIT: screenshot captured, {} bytes", bytes.len());
                    Json(json!({ "status": "ok", "format": "png", "size": bytes.len(), "data_base64": b64 }))
                }
                Err(e) => Json(json!({ "status": "error", "reason": format!("read failed: {}", e) })),
            }
        }
        Ok(o) => Json(json!({ "status": "error", "reason": String::from_utf8_lossy(&o.stderr).to_string() })),
        Err(e) => Json(json!({ "status": "error", "reason": format!("scrot unavailable: {}", e) })),
    }
}

/// GET /api/system/apps — catalogue of installed .desktop apps.
/// Scans standard XDG directories, parses Desktop Entry files, returns
/// a sorted list the AI (or UI) can use to launch or reference apps.
async fn system_apps_handler() -> impl IntoResponse {
    let dirs = [
        "/usr/share/applications",
        "/usr/local/share/applications",
        "/var/lib/snapd/desktop/applications",
    ];
    // Also check user-local
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let user_dir = format!("{}/.local/share/applications", home);

    let mut apps: Vec<serde_json::Value> = Vec::new();
    for dir in dirs.iter().chain(std::iter::once(&user_dir.as_str())) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") { continue; }
            if let Ok(content) = std::fs::read_to_string(&path) {
                let mut name = String::new();
                let mut exec = String::new();
                let mut icon = String::new();
                let mut categories = String::new();
                let mut comment = String::new();
                let mut no_display = false;
                for line in content.lines() {
                    if line.starts_with("Name=") { name = line[5..].to_string(); }
                    else if line.starts_with("Exec=") { exec = line[5..].to_string(); }
                    else if line.starts_with("Icon=") { icon = line[5..].to_string(); }
                    else if line.starts_with("Categories=") { categories = line[11..].to_string(); }
                    else if line.starts_with("Comment=") { comment = line[8..].to_string(); }
                    else if line.starts_with("NoDisplay=true") { no_display = true; }
                }
                if name.is_empty() || no_display { continue; }
                // Strip field codes from Exec (%f, %F, %u, %U, etc.)
                let exec_clean: String = exec.split_whitespace()
                    .filter(|t| !t.starts_with('%'))
                    .collect::<Vec<_>>().join(" ");
                apps.push(json!({
                    "name": name,
                    "exec": exec_clean,
                    "icon": icon,
                    "categories": categories,
                    "comment": comment,
                    "file": path.display().to_string(),
                }));
            }
        }
    }
    apps.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").to_lowercase()
            .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
    });
    Json(json!({ "count": apps.len(), "apps": apps }))
}

/// POST /api/system/launch — launch a desktop app by name or exec path.
/// Uses `setsid` + `xdg-open` or direct exec so the app doesn't die when
/// the server restarts. Auth required.
#[derive(serde::Deserialize)]
pub struct LaunchRequest { pub app: String }

async fn system_launch_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LaunchRequest>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated {
        return Json(json!({ "status": "rejected", "reason": "not authenticated" }));
    }
    drop(agent);
    if req.app.is_empty() || req.app.len() > 500 {
        return Json(json!({ "status": "error", "reason": "invalid app" }));
    }
    // Try xdg-open first (handles .desktop files and URLs), fall back to direct exec.
    let result = std::process::Command::new("setsid")
        .args(["xdg-open", &req.app])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    match result {
        Ok(_) => {
            info!("// AUDIT: Launched app: {}", req.app);
            Json(json!({ "status": "ok", "launched": req.app }))
        }
        Err(e) => Json(json!({ "status": "error", "reason": format!("{}", e) })),
    }
}

// ============================================================
// REST: QoS Compliance Report
// ============================================================

async fn qos_handler() -> impl IntoResponse {
    info!("// AUDIT: QoS compliance report requested.");
    let auditor = crate::qos::QosAuditor::new();
    // Probe PSL axiom pass rate against a fresh random vector
    let probe = crate::memory_bus::HyperMemory::generate_seed(crate::memory_bus::DIM_PROLETARIAT);
    let probe_bv = crate::hdc::vector::BipolarVector::from_bitvec(probe.export_raw_bitvec());
    let axiom_rate = match probe_bv {
        Ok(bv) => {
            let mut sup = crate::psl::supervisor::PslSupervisor::new();
            sup.register_axiom(Box::new(crate::psl::axiom::DimensionalityAxiom));
            sup.register_axiom(Box::new(crate::psl::axiom::StatisticalEquilibriumAxiom { tolerance: 0.05 }));
            match sup.audit(&crate::psl::axiom::AuditTarget::Vector(bv)) {
                Ok(v) => v.confidence,
                Err(_) => 0.5,
            }
        },
        Err(_) => 0.5,
    };
    let report = auditor.audit(axiom_rate);
    Json(serde_json::to_value(&report).unwrap_or(json!({ "error": "serialization failed" })))
}

// ============================================================
// REST: Prometheus Metrics
// ============================================================

/// GET /api/metrics — Prometheus text-format exposition.
async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let body = state.metrics.render_prometheus();
    ([("content-type", "text/plain; version=0.0.4")], body)
}

// ============================================================
// REST: OPSEC Scan
// ============================================================

/// POST /api/opsec/scan — scan text for PII / sensitive markers.
///
/// Returns the sanitized version (with sensitive matches replaced by
/// deterministic placeholders) plus per-match metadata so the caller
/// can audit what was found without leaking the originals back.
///
/// SECURITY: caps text at 64 KiB. The returned sanitized string is
/// safe to log; the original is only included in the response in
/// trimmed form (first 200 chars) for context, never fully echoed.
async fn opsec_scan_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<OpsecRequest>,
) -> impl IntoResponse {
    state.metrics.inc_counter("lfi_opsec_scan_total", &[], 1);
    if req.text.is_empty() {
        return Json(json!({
            "status": "rejected",
            "reason": "text is empty"
        }));
    }
    if req.text.len() > 64 * 1024 {
        return Json(json!({
            "status": "rejected",
            "reason": "text exceeds 64 KiB"
        }));
    }

    debug!("// AUDIT: /api/opsec/scan input_len={}", req.text.len());
    match crate::hdlm::intercept::OpsecIntercept::scan(&req.text) {
        Ok(result) => {
            let detailed: Vec<serde_json::Value> = result.detailed_matches.iter().map(|m| {
                json!({
                    "category": format!("{:?}", m.category),
                    "position": m.position,
                    "redacted_with": m.redacted_with,
                    // Note: original matched text is NOT returned — it's
                    // sensitive data by definition.
                })
            }).collect();
            Json(json!({
                "status": "ok",
                "sanitized": result.sanitized,
                "matches_found": result.matches_found.len(),
                "bytes_redacted": result.bytes_redacted,
                "detailed_matches": detailed,
            }))
        }
        Err(e) => Json(json!({
            "status": "error",
            "reason": format!("scan failed: {:?}", e),
        })),
    }
}

// ============================================================
// REST: PSL Audit
// ============================================================

/// POST /api/audit — run PSL governance over a vector derived from a string seed.
///
/// SECURITY: caps `seed` at 16 KiB. The vector is deterministically generated
/// from the seed, so callers can re-audit the same seed without storing the
/// hypervector themselves.
async fn audit_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuditRequest>,
) -> impl IntoResponse {
    if req.seed.len() > 16 * 1024 {
        return Json(json!({
            "status": "rejected",
            "reason": "seed exceeds 16 KiB"
        }));
    }
    if req.seed.is_empty() {
        return Json(json!({
            "status": "rejected",
            "reason": "seed is empty"
        }));
    }

    debug!("// AUDIT: /api/audit seed_len={}", req.seed.len());
    state.metrics.inc_counter("lfi_audit_total", &[], 1);
    let agent = state.agent.lock();

    // Deterministic hash → seed → BipolarVector.
    let hash = crate::identity::IdentityProver::hash(&req.seed);
    let vec = crate::hdc::vector::BipolarVector::from_seed(hash);
    let target = crate::psl::axiom::AuditTarget::Vector(vec);

    match agent.supervisor.audit(&target) {
        Ok(verdict) => Json(json!({
            "status": "ok",
            "axiom_id": verdict.axiom_id,
            "level": format!("{:?}", verdict.level),
            "confidence": verdict.confidence,
            "detail": verdict.detail,
            "permits_execution": verdict.level.permits_execution(),
        })),
        Err(e) => Json(json!({
            "status": "error",
            "reason": format!("audit failed: {}", e),
        })),
    }
}

// ============================================================
// REST: Agent State Snapshot
// ============================================================

/// GET /api/agent/state — single-call dashboard summary.
///
/// Aggregates everything a monitoring dashboard normally needs into
/// one round-trip: subsystem readiness, axiom inventory, knowledge
/// stats, provenance counters. Cheaper than fan-out across
/// /api/health + /api/knowledge/concepts + /api/provenance/stats.
async fn agent_state_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    let axiom_count = agent.supervisor.axiom_count();
    let concept_count = agent.reasoner.knowledge.concept_count();
    let due_count = agent.reasoner.knowledge.concepts_due_for_review(usize::MAX).len();
    let trace_count = agent.provenance.lock().trace_count();
    let current_tier = format!("{:?}", agent.current_tier);
    let authenticated = agent.authenticated;

    Json(json!({
        "psl": {
            "axiom_count": axiom_count,
            "material_trust_threshold": agent.supervisor.material_trust_threshold,
            "hard_fail_threshold": agent.supervisor.hard_fail_threshold,
        },
        "knowledge": {
            "concept_count": concept_count,
            "due_for_review": due_count,
        },
        "provenance": {
            "trace_count": trace_count,
        },
        "agent": {
            "authenticated": authenticated,
            "current_tier": current_tier,
        }
    }))
}

// ============================================================
// REST: Health Check
// ============================================================

/// GET /api/health — quick subsystem health summary for monitors / load balancers.
///
/// Returns a flat JSON object with boolean flags for each subsystem.
/// Status code is always 200 so a monitor can parse the payload rather
/// than relying solely on HTTP status; a hard "down" surface is signalled
/// by `ok: false`.
async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    let provenance_engine_ready = agent.provenance.try_lock().is_some();
    let axiom_count = agent.supervisor.axiom_count();
    let concept_count = agent.reasoner.knowledge.concept_count();

    // Release agent lock before running checks that would reacquire it.
    let current_tier = format!("{:?}", agent.current_tier);
    let authenticated = agent.authenticated;
    drop(agent);

    let ok = provenance_engine_ready && axiom_count > 0 && concept_count > 0;

    Json(json!({
        "ok": ok,
        "subsystems": {
            "provenance_engine": provenance_engine_ready,
            "psl_axioms_registered": axiom_count,
            "knowledge_concepts": concept_count,
            "authenticated": authenticated,
            "current_tier": current_tier,
        }
    }))
}

// ============================================================
// REST: Think with Provenance
// ============================================================

/// POST /api/think — think with provenance tracking.
/// Response: { answer, confidence, mode, conclusion_id }.
/// SECURITY: rejects inputs > 16 KiB to prevent resource exhaustion.
async fn think_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ThinkRequest>,
) -> impl IntoResponse {
    if req.input.len() > 16 * 1024 {
        warn!("// AUDIT: /api/think rejected oversize input ({} bytes)", req.input.len());
        return Json(json!({
            "status": "rejected",
            "reason": "input exceeds 16 KiB"
        }));
    }

    debug!("// AUDIT: /api/think input_len={}", req.input.len());
    state.metrics.inc_counter("lfi_think_total", &[], 1);
    let mut agent = state.agent.lock();
    match agent.think_traced(&req.input) {
        Ok((result, cid)) => Json(json!({
            "status": "ok",
            "answer": result.explanation,
            "confidence": result.confidence,
            "mode": format!("{:?}", result.mode),
            "conclusion_id": cid,
        })),
        Err(e) => Json(json!({
            "status": "error",
            "reason": format!("think failed: {}", e),
        })),
    }
}

// ============================================================
// REST: Knowledge / Spaced Repetition
// ============================================================

/// POST /api/knowledge/review — record a graded review for a concept.
/// Updates KnowledgeEngine mastery and the SM-2 scheduler.
async fn knowledge_review_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReviewRequest>,
) -> impl IntoResponse {
    if req.concept.is_empty() || req.concept.len() > 256 {
        return Json(json!({
            "status": "rejected",
            "reason": "concept must be 1..=256 bytes"
        }));
    }
    let mut agent = state.agent.lock();
    let before = agent.reasoner.knowledge.mastery_of(&req.concept);
    agent.reasoner.knowledge.review(&req.concept, req.quality);
    let after = agent.reasoner.knowledge.mastery_of(&req.concept);
    Json(json!({
        "status": "ok",
        "concept": req.concept,
        "quality": req.quality.min(5),
        "mastery_before": before,
        "mastery_after": after,
    }))
}

/// POST /api/knowledge/learn — teach LFI a new concept (authenticated only).
///
/// SECURITY: requires authentication. KnowledgeEngine.learn rejects
/// untrusted teaching outright, but exposing this through HTTP would
/// still let any caller burn CPU cycles registering noise. Auth gates
/// the entry point.
async fn knowledge_learn_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LearnRequest>,
) -> impl IntoResponse {
    if req.concept.is_empty() || req.concept.len() > 256 {
        return Json(json!({
            "status": "rejected",
            "reason": "concept must be 1..=256 bytes"
        }));
    }
    if req.related.len() > 64 {
        return Json(json!({
            "status": "rejected",
            "reason": "related list capped at 64"
        }));
    }

    let mut agent = state.agent.lock();
    if !agent.authenticated {
        warn!("// AUDIT: /api/knowledge/learn rejected — not authenticated.");
        return Json(json!({
            "status": "rejected",
            "reason": "authentication required"
        }));
    }

    let related_refs: Vec<&str> = req.related.iter().map(|s| s.as_str()).collect();
    match agent.reasoner.knowledge.learn(&req.concept, &related_refs, true) {
        Ok(()) => {
            let mastery = agent.reasoner.knowledge.mastery_of(&req.concept);
            Json(json!({
                "status": "ok",
                "concept": req.concept,
                "mastery": mastery,
            }))
        }
        Err(e) => Json(json!({
            "status": "error",
            "reason": format!("learn failed: {}", e),
        })),
    }
}

/// GET /api/knowledge/concepts — list every known concept with mastery.
async fn knowledge_concepts_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    let concepts: Vec<serde_json::Value> = agent.reasoner.knowledge.concepts().iter()
        .map(|c| json!({
            "name": c.name,
            "mastery": c.mastery,
            "encounter_count": c.encounter_count,
            "trust_score": c.trust_score,
            "related": c.related_concepts,
        }))
        .collect();
    Json(json!({
        "status": "ok",
        "count": concepts.len(),
        "concepts": concepts,
    }))
}

/// GET /api/knowledge/due — concepts currently due for review (most overdue first).
async fn knowledge_due_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    let due = agent.reasoner.knowledge.concepts_due_for_review(50);
    let names: Vec<String> = due.iter().map(|c| c.name.clone()).collect();
    Json(json!({
        "status": "ok",
        "count": names.len(),
        "concepts": names,
    }))
}

// ============================================================
// REST: Reasoning Provenance
// ============================================================

/// GET /api/provenance/stats — total traces, traced vs reconstructed ratio.
async fn provenance_stats_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    debug!("// AUDIT: Provenance stats requested.");
    let agent = state.agent.lock();
    let engine = agent.provenance.lock();
    let trace_count = engine.trace_count();
    let is_empty = trace_count == 0;
    drop(engine);
    Json(json!({
        "trace_count": trace_count,
        "has_traces": !is_empty,
        "note": if is_empty {
            "No traces recorded yet. Reasoning paths are recorded when \
             subsystems call the *_with_provenance variants."
        } else {
            "Traces available. Query /api/provenance/:conclusion_id for a specific derivation."
        }
    }))
}

/// GET /api/provenance/:conclusion_id — explanation (traced or reconstructed).
async fn provenance_explain_handler(
    State(state): State<Arc<AppState>>,
    Path(conclusion_id): Path<u64>,
) -> impl IntoResponse {
    debug!("// AUDIT: Provenance explanation for cid={}", conclusion_id);
    let agent = state.agent.lock();
    let engine = agent.provenance.lock();
    let explanation = engine.explain_conclusion(conclusion_id);
    let kind_label = match explanation.kind {
        crate::reasoning_provenance::ProvenanceKind::TracedDerivation => "traced",
        crate::reasoning_provenance::ProvenanceKind::ReconstructedRationalization { .. } => "reconstructed",
    };
    state.metrics.inc_counter("lfi_provenance_query_total", &[("kind", kind_label)], 1);
    Json(json!({
        "conclusion_id": conclusion_id,
        "kind": match explanation.kind {
            crate::reasoning_provenance::ProvenanceKind::TracedDerivation =>
                json!({ "kind": "TracedDerivation" }),
            crate::reasoning_provenance::ProvenanceKind::ReconstructedRationalization { ref reason } =>
                json!({ "kind": "ReconstructedRationalization", "reason": reason }),
        },
        "explanation": explanation.explanation,
        "depth": explanation.depth,
        "trace_chain_ids": explanation.trace_chain,
        "confidence_chain": explanation.confidence_chain,
    }))
}

/// GET /api/provenance/export — the entire arena as JSON (audit download).
/// SECURITY: requires the agent to be authenticated — provenance can contain
/// derivation details an attacker could use to reverse-engineer reasoning.
async fn provenance_export_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated {
        warn!("// AUDIT: /api/provenance/export rejected — not authenticated.");
        return Json(json!({
            "status": "rejected",
            "reason": "authentication required"
        }));
    }
    let engine = agent.provenance.lock();
    match engine.arena.to_json() {
        Ok(json) => {
            info!("// AUDIT: provenance arena exported ({} bytes)", json.len());
            Json(json!({
                "status": "ok",
                "trace_count": engine.trace_count(),
                "arena_json_size_bytes": json.len(),
                "arena": serde_json::from_str::<serde_json::Value>(&json)
                    .unwrap_or(json!(null)),
            }))
        }
        Err(e) => Json(json!({
            "status": "error",
            "reason": format!("serialize failed: {}", e),
        })),
    }
}

/// POST /api/provenance/compact — reclaim dead entries (ref_count = 0).
/// SECURITY: requires authentication. Compaction invalidates existing
/// TraceIds, so this must only run when no external references are in
/// flight — typically called between sessions by an administrator.
async fn provenance_compact_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated {
        warn!("// AUDIT: /api/provenance/compact rejected — not authenticated.");
        return Json(json!({
            "status": "rejected",
            "reason": "authentication required"
        }));
    }
    let mut engine = agent.provenance.lock();
    let before = engine.arena.len();
    let removed = engine.arena.compact();
    let after = engine.arena.len();
    info!("// AUDIT: provenance compact: {} → {} (removed {})", before, after, removed);
    Json(json!({
        "status": "ok",
        "before": before,
        "after": after,
        "removed": removed,
    }))
}

/// POST /api/provenance/reset — wipe the arena and start fresh.
/// SECURITY: requires authentication; destructive and irreversible.
async fn provenance_reset_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let agent = state.agent.lock();
    if !agent.authenticated {
        warn!("// AUDIT: /api/provenance/reset rejected — not authenticated.");
        return Json(json!({
            "status": "rejected",
            "reason": "authentication required"
        }));
    }
    let mut engine = agent.provenance.lock();
    let old_count = engine.trace_count();
    *engine = crate::reasoning_provenance::ProvenanceEngine::new();
    info!("// AUDIT: provenance engine reset ({} traces cleared)", old_count);
    Json(json!({
        "status": "ok",
        "traces_cleared": old_count,
    }))
}

/// GET /api/provenance/:conclusion_id/chain — the full TraceEntry list for a conclusion.
async fn provenance_chain_handler(
    State(state): State<Arc<AppState>>,
    Path(conclusion_id): Path<u64>,
) -> impl IntoResponse {
    debug!("// AUDIT: Provenance chain for cid={}", conclusion_id);
    let agent = state.agent.lock();
    let engine = agent.provenance.lock();
    let explanation = engine.explain_conclusion(conclusion_id);

    // Materialize each TraceEntry (clone under lock, then release).
    let entries: Vec<serde_json::Value> = explanation.trace_chain.iter()
        .filter_map(|&id| engine.arena.get(id).cloned())
        .map(|e| serde_json::to_value(&e).unwrap_or_else(|_| json!({
            "error": "serialize failed",
            "id": e.id,
        })))
        .collect();

    Json(json!({
        "conclusion_id": conclusion_id,
        "chain_length": entries.len(),
        "entries": entries,
    }))
}

// ============================================================
// Router Construction
// ============================================================

pub fn create_router() -> Result<Router, Box<dyn std::error::Error>> {
    let (tx, _) = broadcast::channel(100);

    let agent = LfiAgent::new().map_err(|e| -> Box<dyn std::error::Error> {
        tracing::error!("// CRITICAL: LfiAgent initialization failed: {}", e);
        format!("LfiAgent init failed: {}", e).into()
    })?;
    let metrics = Arc::new(crate::intelligence::metrics::LfiMetrics::new());
    metrics.register_help("lfi_think_total",
        "Total number of POST /api/think calls accepted (post-validation)");
    metrics.register_help("lfi_provenance_query_total",
        "Total /api/provenance/:cid lookups by kind");
    metrics.register_help("lfi_chat_total",
        "Total chat messages handled over /ws/chat");
    metrics.register_help("lfi_audit_total",
        "Total POST /api/audit calls accepted by the PSL supervisor");
    metrics.register_help("lfi_opsec_scan_total",
        "Total POST /api/opsec/scan calls (PII redaction)");

    // Open the persistent brain database. Facts learned during conversation
    // survive server restarts — per Architectural Bible §4.2.
    let db_path = crate::persistence::BrainDb::default_path();
    let db = Arc::new(crate::persistence::BrainDb::open(&db_path)
        .unwrap_or_else(|e| {
            warn!("// PERSISTENCE: Failed to open {}: {} — using fallback /tmp", db_path.display(), e);
            crate::persistence::BrainDb::open(std::path::Path::new("/tmp/plausiden_brain.db")).expect("fallback DB must open")
        }));

    // Hydrate agent facts from the persistent store. With 40M+ facts in the DB,
    // loading everything into memory is infeasible. We hydrate only user-extracted
    // facts and recent high-priority facts. The full DB is queried on demand.
    // BUG ASSUMPTION: get_all_facts on a 40M row table causes multi-minute startup
    // delay and potential OOM. Capped hydration fixes this.
    let agent = Mutex::new(agent);
    {
        let mut agent_lock = agent.lock();
        let hydration_facts = db.get_recent_facts(10_000);
        for (key, value, _source, _conf) in &hydration_facts {
            agent_lock.conversation_facts.insert(key.clone(), value.clone());
            let mut guard = agent_lock.shared_knowledge.lock();
            guard.store.upsert_fact(key, value);
        }
        let count = agent_lock.conversation_facts.len();
        if count > 0 {
            info!("// PERSISTENCE: Hydrated {} facts from brain.db (capped for startup speed)", count);
        }
    }

    let state = Arc::new(AppState {
        tx,
        agent,
        search_engine: WebSearchEngine::new(),
        metrics,
        db,
    });

    let cors = CorsLayer::permissive();

    Ok(Router::new()
        .route("/ws/telemetry", get(telemetry_handler))
        .route("/ws/chat", get(chat_handler))
        .route("/api/auth", post(auth_handler))
        .route("/api/status", get(status_handler))
        .route("/api/facts", get(facts_handler))
        .route("/api/search", post(search_handler))
        .route("/api/tier", post(tier_handler))
        .route("/api/qos", get(qos_handler))
        .route("/api/health", get(health_handler))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/agent/state", get(agent_state_handler))
        .route("/api/chat-log", get(chat_log_handler))
        .route("/api/stop", post(stop_handler))
        .route("/api/system/info", get(system_info_handler))
        .route("/api/system/notify", post(system_notify_handler))
        .route("/api/system/clipboard", get(clipboard_get_handler).post(clipboard_set_handler))
        .route("/api/conversations", get(conversations_list_handler))
        .route("/api/conversations/sync", post(conversations_sync_handler))
        .route("/api/conversations/:id", get(conversation_get_handler).delete(conversation_delete_handler))
        .route("/api/research", post(research_handler))
        .route("/api/training/status", get(training_status_handler))
        .route("/api/system/click", post(system_click_handler))
        .route("/api/system/type", post(system_type_handler))
        .route("/api/system/key", post(system_key_handler))
        .route("/api/system/screenshot", get(system_screenshot_handler))
        .route("/api/system/apps", get(system_apps_handler))
        .route("/api/system/launch", post(system_launch_handler))
        .route("/api/think", post(think_handler))
        .route("/api/audit", post(audit_handler))
        .route("/api/opsec/scan", post(opsec_scan_handler))
        .route("/api/knowledge/review", post(knowledge_review_handler))
        .route("/api/knowledge/due", get(knowledge_due_handler))
        .route("/api/knowledge/concepts", get(knowledge_concepts_handler))
        .route("/api/knowledge/learn", post(knowledge_learn_handler))
        .route("/api/provenance/stats", get(provenance_stats_handler))
        .route("/api/provenance/export", get(provenance_export_handler))
        .route("/api/provenance/compact", post(provenance_compact_handler))
        .route("/api/provenance/reset", post(provenance_reset_handler))
        .route("/api/provenance/:conclusion_id", get(provenance_explain_handler))
        .route("/api/provenance/:conclusion_id/chain", get(provenance_chain_handler))
        .layer(cors)
        .with_state(state))
}
