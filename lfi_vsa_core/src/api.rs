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
use crate::intelligence::web_search::WebSearchEngine;

/// Shared application state across all handlers.
pub struct AppState {
    pub tx: broadcast::Sender<String>,
    pub agent: Mutex<LfiAgent>,
    pub search_engine: WebSearchEngine,
    pub metrics: Arc<crate::intelligence::metrics::LfiMetrics>,
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

                    match agent.chat_traced(input) {
                        Ok((response, conclusion_id)) => {
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
                                // Provenance: client can query /api/provenance/:id with this
                                "conclusion_id": conclusion_id,
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

    let state = Arc::new(AppState {
        tx,
        agent: Mutex::new(agent),
        search_engine: WebSearchEngine::new(),
        metrics,
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
